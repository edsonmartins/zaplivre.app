//! Probe isolado do signaling server.
//!
//! Gera dois peer IDs libp2p (ed25519), registra ambos no signaling server via
//! WebSocket e troca um `signal` de um para o outro. Serve para isolar se o
//! relay /ws entrega corretamente de ponta a ponta, sem depender do P2P/libp2p,
//! do app ou do WebRTC.
//!
//! Uso:
//!   cargo run --example signaling_probe -- <ws_url>
//!   ex.: cargo run --example signaling_probe -- ws://localhost:8086/ws

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use libp2p::identity::Keypair;
use libp2p::PeerId;
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let ws_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://localhost:8086/ws".into());

    // Gera dois peers ed25519.
    let (kp_a, peer_a, ts_a) = new_peer();
    let (kp_b, peer_b, _) = new_peer();
    println!("peer A: {peer_a}");
    println!("peer B: {peer_b}");

    // Conecta e registra ambos.
    let mut ws_a = register(&ws_url, &kp_a, &peer_a, ts_a).await;
    let mut ws_b = register(&ws_url, &kp_b, &peer_b, chrono::Utc::now().timestamp()).await;
    println!("✔ ambos registrados");

    // Payload de exemplo (um CallOffer típico).
    let payload = json!({"call_id":"probe-123","sdp":"v=0\r\nm=audio 9 RTP/AVP 0\r\n"});

    // A envia signal para B.
    let out = json!({
        "type": "signal",
        "from_peer_id": peer_a.to_string(),
        "to_peer_id": peer_b.to_string(),
        "payload": payload
    });
    println!("→ A envia signal para B ...");
    ws_a.send(Message::Text(out.to_string())).await.unwrap();

    // B deve receber o signal remetido pelo relay.
    let mut delivered = false;
    for _ in 0..10 {
        if let Some(Ok(Message::Text(text))) = ws_b.next().await {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            println!("→ B recebeu: {v}");
            if v["type"] == "signal" && v["from_peer_id"] == peer_a.to_string() {
                delivered = true;
                break;
            }
        }
    }

    if delivered {
        println!("✅ RELAY OK: signal entregue de A para B via signaling server");
    } else {
        println!("❌ RELAY FALHOU: signal não chegou a B");
        std::process::exit(1);
    }
}

/// Gera um keypair ed25519 + PeerId correspondente + timestamp p/ registro.
fn new_peer() -> (Keypair, PeerId, i64) {
    let kp = Keypair::generate_ed25519();
    let peer = PeerId::from_public_key(&kp.public());
    (kp, peer, chrono::Utc::now().timestamp())
}

/// Conecta ao /ws e registra o peer com assinatura ed25519
/// sobre "signaling-register:{peer_id}:{ts}" via Keypair::sign (libp2p).
async fn register(
    url: &str,
    kp: &Keypair,
    peer: &PeerId,
    ts: i64,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let (mut ws, _) = connect_async(url).await.expect("conectar ao /ws");

    let msg = format!("signaling-register:{}:{}", peer, ts);
    let sig = kp.sign(msg.as_bytes()).expect("assinar registro");
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig);
    let reg = json!({
        "type": "register",
        "peer_id": peer.to_string(),
        "ts": ts,
        "sig": sig_b64
    });
    ws.send(Message::Text(reg.to_string()))
        .await
        .expect("enviar register");

    // Aguarda close do servidor se o registro falhar, senão segue.
    ws.send(Message::Text(json!({"type":"ping"}).to_string()))
        .await
        .ok();
    ws
}
