//! M6: uma oferta de chamada para quem está com o app fechado (sem WebSocket)
//! pede um push de chamada e é entregue quando o aparelho acorda e se registra.
//! Sobe o binário real do signaling e um push server falso.

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

/// Push server stand-in: records the JSON bodies it receives.
async fn fake_push_server() -> (String, Arc<Mutex<Vec<serde_json::Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::clone(&seen);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
                    let len = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if raw.len() >= split + 4 + len {
                        if let Ok(json) = serde_json::from_slice(&raw[split + 4..split + 4 + len]) {
                            store.lock().unwrap().push(json);
                        }
                        let _ = socket
                            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                            .await;
                        return;
                    }
                }
            });
        }
    });
    (url, seen)
}

fn identity(seed: u8) -> (libp2p_identity::Keypair, String) {
    let key = libp2p_identity::Keypair::ed25519_from_bytes([seed; 32]).unwrap();
    let peer = key.public().to_peer_id().to_string();
    (key, peer)
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn register(url: &str, key: &libp2p_identity::Keypair, peer: &str) -> Socket {
    let (mut ws, _) = connect_async(url).await.expect("connect");
    let ts = chrono::Utc::now().timestamp();
    let sig = key
        .sign(format!("signaling-register:{peer}:{ts}").as_bytes())
        .unwrap();
    let register = serde_json::json!({
        "type": "register",
        "peer_id": peer,
        "ts": ts,
        "sig": general_purpose::STANDARD.encode(sig),
    });
    ws.send(Message::Text(register.to_string())).await.unwrap();
    ws
}

#[tokio::test]
async fn call_offer_to_a_closed_app_pushes_and_is_delivered_on_register() {
    let (push_url, pushes) = fake_push_server().await;
    let port = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let _server = Server(
        Command::new(env!("CARGO_BIN_EXE_zaplivre-signaling"))
            .env("PORT", port.to_string())
            .env("PUSH_SERVER_URL", &push_url)
            .env("PUSH_SERVICE_SECRET", "test-secret-0123456789abcdef0123")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start signaling server"),
    );
    let ws_url = format!("ws://127.0.0.1:{port}/ws");
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let (alice_key, alice) = identity(1);
    let (bob_key, bob) = identity(2);

    // Alice calls Bob, whose app is closed (no WebSocket).
    let mut alice_ws = register(&ws_url, &alice_key, &alice).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let offer = serde_json::json!({
        "type": "signal",
        "from_peer_id": alice,
        "to_peer_id": bob,
        "payload": {"type": "call_offer", "call_id": "call-1", "sdp": "v=0"},
    });
    alice_ws
        .send(Message::Text(offer.to_string()))
        .await
        .unwrap();

    // The push server is asked to wake Bob for this call, and only once.
    let mut push = None;
    for _ in 0..50 {
        if let Some(p) = pushes.lock().unwrap().first().cloned() {
            push = Some(p);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let push = push.expect("no wake-up push for the call");
    assert_eq!(push["peer_id"], bob.as_str());
    assert_eq!(push["data"]["type"], "incoming_call");
    assert_eq!(push["data"]["sender_peer_id"], alice.as_str());
    assert_eq!(push["data"]["call_id"], "call-1");

    // Bob's app wakes up and registers: the parked offer is delivered.
    let mut bob_ws = register(&ws_url, &bob_key, &bob).await;
    let received = tokio::time::timeout(Duration::from_secs(5), bob_ws.next())
        .await
        .expect("offer not delivered after register")
        .expect("socket closed")
        .expect("socket error");
    let Message::Text(text) = received else {
        panic!("unexpected frame: {received:?}");
    };
    let delivered: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(delivered["from_peer_id"], alice.as_str());
    assert_eq!(delivered["payload"]["call_id"], "call-1");
    assert_eq!(pushes.lock().unwrap().len(), 1, "one push per call");
}
