//! Helpers shared by the integration tests.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Mailbox contents, oldest first, in the store's wire format.
pub type Mailbox = Arc<Mutex<Vec<Value>>>;

/// Minimal message store: POST stores, GET lists (honouring `limit`), DELETE
/// removes by id. Request signatures are not checked here.
pub async fn spawn_fake_store() -> (String, Mailbox) {
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
