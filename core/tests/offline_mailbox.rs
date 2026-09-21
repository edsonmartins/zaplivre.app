//! Store-and-forward de ponta a ponta contra um message store falso (HTTP):
//! o remetente deixa a mensagem no store porque o destinatário está offline, o
//! destinatário drena a caixa, e só o que foi processado é apagado do servidor.

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use zaplivre_core::api::ClientBuilder;

/// Mailbox contents, oldest first, in the store's wire format.
type Mailbox = Arc<Mutex<Vec<Value>>>;

/// Minimal message store: POST stores, GET lists (honouring `limit`), DELETE
/// removes by id. Request signatures are not checked here.
async fn spawn_fake_store() -> (String, Mailbox) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let mailbox: Mailbox = Arc::default();

    let state = Arc::clone(&mailbox);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                let mut raw = Vec::new();
                let mut buf = [0u8; 8192];
                let (head, body) = loop {
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&raw[..split]).to_string();
                    let length = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                        })
                        .unwrap_or(0);
                    if raw.len() >= split + 4 + length {
                        break (head, raw[split + 4..split + 4 + length].to_vec());
                    }
                };

                let request_line = head.lines().next().unwrap_or_default().to_string();
                let mut parts = request_line.split(' ');
                let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                let response = match method {
                    "POST" => {
                        let mut message: Value = serde_json::from_slice(&body).unwrap();
                        message["created_at"] = json!("2026-01-01T00:00:00Z");
                        message["message_type"] = json!("encrypted");
                        state.lock().unwrap().push(message);
                        json!({ "status": "stored" })
                    }
                    "GET" => {
                        let limit = target
                            .split("limit=")
                            .nth(1)
                            .and_then(|v| v.split('&').next())
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(100);
                        let messages: Vec<Value> =
                            state.lock().unwrap().iter().take(limit).cloned().collect();
                        json!({ "total": messages.len(), "messages": messages })
                    }
                    "DELETE" => {
                        let request: Value = serde_json::from_slice(&body).unwrap();
                        let ids: Vec<String> =
                            serde_json::from_value(request["message_ids"].clone()).unwrap();
                        state
                            .lock()
                            .unwrap()
                            .retain(|m| !ids.iter().any(|id| m["message_id"] == json!(id)));
                        json!({ "deleted": ids.len() })
                    }
                    _ => json!({}),
                };
                let payload = response.to_string();
                let status = if method == "POST" {
                    "201 Created"
                } else {
                    "200 OK"
                };
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });

    (url, mailbox)
}

#[tokio::test]
async fn offline_message_is_delivered_through_the_store() {
    let (store_url, mailbox) = spawn_fake_store().await;

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir_a = tempfile::TempDir::new().unwrap();
            let dir_b = tempfile::TempDir::new().unwrap();
            let alice = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_a.path().to_path_buf())
                    .message_store_url(store_url.clone())
                    .build()
                    .await
                    .expect("build alice"),
            );
            let bob = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_b.path().to_path_buf())
                    .message_store_url(store_url.clone())
                    .build()
                    .await
                    .expect("build bob"),
            );
            let alice_peer = alice.local_peer_id();
            let bob_peer = bob.local_peer_id();

            // Alice only knows Bob's bundle (as from a QR code); they never connect.
            alice
                .set_contact_prekey_bundle(
                    bob_peer.to_string(),
                    bob.get_prekey_bundle_json().await.expect("bob bundle"),
                )
                .expect("store bob bundle");

            let driver = Rc::clone(&alice);
            tokio::task::spawn_local(async move {
                loop {
                    if !driver.poll_network_once().await.unwrap_or(false) {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            });

            let message_id = alice
                .send_text_message(bob_peer, "guardado no store".to_string())
                .await
                .expect("send while bob is offline");
            assert_eq!(
                mailbox.lock().unwrap().len(),
                1,
                "message parked on the store"
            );

            // A payload Bob can never decode must stay on the server instead of
            // being deleted as if it had been processed.
            mailbox.lock().unwrap().push(json!({
                "recipient_peer_id": bob_peer.to_string(),
                "sender_peer_id": alice_peer.to_string(),
                "encrypted_payload": "AAAA",
                "message_type": "encrypted",
                "message_id": "poison",
                "created_at": "2026-01-01T00:00:00Z",
            }));

            assert_eq!(bob.fetch_offline_messages().await.expect("fetch"), 1);
            let received = bob
                .get_conversation_messages(&alice_peer.to_string(), None, None)
                .expect("bob conversation");
            assert!(
                received.iter().any(|m| m.message_id == message_id),
                "bob must have the message Alice left on the store"
            );

            let left: Vec<String> = mailbox
                .lock()
                .unwrap()
                .iter()
                .map(|m| m["message_id"].as_str().unwrap_or_default().to_string())
                .collect();
            assert_eq!(left, vec!["poison"], "only processed messages are deleted");

            // Draining again is a no-op and must not duplicate anything.
            assert_eq!(bob.fetch_offline_messages().await.expect("refetch"), 0);
            let after = bob
                .get_conversation_messages(&alice_peer.to_string(), None, None)
                .expect("bob conversation");
            assert_eq!(after.len(), received.len());
        })
        .await;
}
