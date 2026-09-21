//! ZapLivre Signaling Server
//!
//! WebSocket relay for WebRTC signaling fallback.
//!
//! Segurança (SEC-11):
//! - `register` exige assinatura Ed25519 sobre "signaling-register:{peer_id}:{ts}"
//!   verificada com a chave pública embutida no próprio peer ID libp2p
//! - o relay só aceita `signal` de conexões registradas e força
//!   `from_peer_id` = peer autenticado da conexão (anti-spoofing)

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose, Engine as _};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{info, warn};

#[derive(Clone, Default)]
struct AppState {
    peers: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
    registrations: Arc<Mutex<HashMap<String, i64>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireMessage {
    Register {
        peer_id: String,
        #[serde(default)]
        ts: i64,
        #[serde(default)]
        sig: String,
    },
    Signal {
        from_peer_id: String,
        to_peer_id: String,
        payload: serde_json::Value,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "zaplivre_signaling=info,info".into()),
        )
        .init();

    let state = AppState::default();

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8086);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("📡 Signaling server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    // Enforce the payload limit while reading the frame, not after the
    // default (multi-megabyte) frame has already been buffered.
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state))
}

/// Limite de payload de sinalização (SDP/ICE são pequenos)
const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const OUTBOUND_QUEUE_CAPACITY: usize = 64;
const RATE_WINDOW_SECONDS: i64 = 60;
const MAX_MESSAGES_PER_WINDOW: usize = 120;
/// A socket that has not proven a peer ID within this time is closed, so
/// unauthenticated connections cannot be parked forever.
const REGISTRATION_DEADLINE: Duration = Duration::from_secs(15);

/// Drop `peer_id` from the routing table only if it still points at this
/// connection. A client that reconnects (Wi-Fi to 4G) registers on a new
/// socket before the old one dies; removing by peer ID alone made the dying
/// socket erase the fresh registration and left the peer unreachable for calls.
fn unregister_if_owner(
    peers: &mut HashMap<String, mpsc::Sender<Message>>,
    peer_id: &str,
    connection: &mpsc::Sender<Message>,
) {
    if peers
        .get(peer_id)
        .is_some_and(|current| current.same_channel(connection))
    {
        peers.remove(peer_id);
    }
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(OUTBOUND_QUEUE_CAPACITY);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut registered_peer: Option<String> = None;
    let mut request_window_start = chrono::Utc::now().timestamp();
    let mut request_count = 0usize;

    let registration_deadline = tokio::time::Instant::now() + REGISTRATION_DEADLINE;
    loop {
        let next = if registered_peer.is_some() {
            receiver.next().await
        } else {
            match tokio::time::timeout_at(registration_deadline, receiver.next()).await {
                Ok(next) => next,
                Err(_) => {
                    warn!("closing socket that never registered");
                    break;
                }
            }
        };
        let Some(Ok(msg)) = next else {
            break;
        };
        let now = chrono::Utc::now().timestamp();
        if now - request_window_start >= RATE_WINDOW_SECONDS {
            request_window_start = now;
            request_count = 0;
        }
        request_count += 1;
        if request_count > MAX_MESSAGES_PER_WINDOW {
            warn!("signaling rate limit exceeded");
            break;
        }
        if let Message::Text(text) = msg {
            if text.len() > MAX_MESSAGE_BYTES {
                warn!(
                    "⚠️ Oversized signaling message dropped ({} bytes)",
                    text.len()
                );
                continue;
            }
            let parsed = serde_json::from_str::<WireMessage>(&text);
            match parsed {
                Ok(WireMessage::Register { peer_id, ts, sig }) => {
                    // One identity per connection: a second register would
                    // leave the first peer ID routed to this socket.
                    if registered_peer.is_some() {
                        warn!("🚫 Second registration on the same connection ignored");
                        continue;
                    }
                    // SEC-11: prova de posse do peer ID
                    match verify_registration(&peer_id, ts, &sig) {
                        Ok(()) => {
                            let mut registrations = state.registrations.lock().await;
                            registrations.retain(|_, seen| now - *seen <= 300);
                            let replay_key = format!("{}:{}:{}", peer_id, ts, sig);
                            if registrations.insert(replay_key, now).is_some() {
                                warn!("replayed signaling registration rejected");
                                continue;
                            }
                            drop(registrations);
                            info!("🔌 Peer registered: {}", peer_id);
                            state
                                .peers
                                .write()
                                .await
                                .insert(peer_id.clone(), out_tx.clone());
                            registered_peer = Some(peer_id);
                        }
                        Err(reason) => {
                            warn!("🚫 Registration rejected for {}: {}", peer_id, reason);
                        }
                    }
                }
                Ok(WireMessage::Signal {
                    from_peer_id,
                    to_peer_id,
                    payload,
                }) => {
                    // SEC-11: só conexões registradas relayam, e apenas em
                    // nome do próprio peer autenticado
                    let Some(authenticated) = registered_peer.as_ref() else {
                        warn!("🚫 Signal from unregistered connection dropped");
                        continue;
                    };
                    if from_peer_id != *authenticated {
                        warn!(
                            "🚫 Spoofed from_peer_id {} on connection of {}",
                            from_peer_id, authenticated
                        );
                        continue;
                    }

                    let msg = WireMessage::Signal {
                        from_peer_id,
                        to_peer_id: to_peer_id.clone(),
                        payload,
                    };
                    if let Ok(text) = serde_json::to_string(&msg) {
                        // Clone the sender out of the lock: a slow target must
                        // not hold the routing table against new registrations.
                        let target = state.peers.read().await.get(&to_peer_id).cloned();
                        if let Some(target) = target {
                            if tokio::time::timeout(
                                Duration::from_secs(2),
                                target.send(Message::Text(text)),
                            )
                            .await
                            .is_err()
                            {
                                warn!("signaling target queue timeout: {}", to_peer_id);
                            }
                        } else {
                            warn!("⚠️ Target peer not connected: {}", to_peer_id);
                        }
                    }
                }
                Err(err) => {
                    warn!("⚠️ Invalid signaling message: {}", err);
                }
            }
        }
    }

    if let Some(peer_id) = registered_peer {
        info!("🔌 Peer disconnected: {}", peer_id);
        unregister_if_owner(&mut *state.peers.write().await, &peer_id, &out_tx);
    }

    send_task.abort();
}

/// Verifica a assinatura de registro: Ed25519 sobre
/// "signaling-register:{peer_id}:{ts}", chave extraída do peer ID
fn verify_registration(peer_id: &str, ts: i64, sig_b64: &str) -> Result<(), &'static str> {
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err("timestamp outside allowed window");
    }

    let verifying_key =
        public_key_from_peer_id(peer_id).ok_or("peer id has no inline ed25519 key")?;

    let sig_bytes = general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|_| "invalid signature encoding")?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| "invalid signature length")?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    let message = format!("signaling-register:{}:{}", peer_id, ts);

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(message.as_bytes(), &signature)
        .map_err(|_| "invalid signature")
}

/// Extrai a chave pública Ed25519 embutida num peer ID libp2p
fn public_key_from_peer_id(peer_id_str: &str) -> Option<ed25519_dalek::VerifyingKey> {
    let peer_id: libp2p_identity::PeerId = peer_id_str.parse().ok()?;
    let multihash = peer_id.as_ref();
    if multihash.code() != 0x00 {
        return None;
    }
    let public_key = libp2p_identity::PublicKey::try_decode_protobuf(multihash.digest()).ok()?;
    let ed25519 = public_key.try_into_ed25519().ok()?;
    ed25519_dalek::VerifyingKey::from_bytes(&ed25519.to_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_connection_does_not_erase_a_reconnected_peer() {
        let mut peers = HashMap::new();
        let (old_conn, _old_rx) = mpsc::channel::<Message>(1);
        let (new_conn, _new_rx) = mpsc::channel::<Message>(1);

        // Peer registers on Wi-Fi, then again on 4G before the first socket dies.
        peers.insert("peer".to_string(), old_conn.clone());
        peers.insert("peer".to_string(), new_conn.clone());

        unregister_if_owner(&mut peers, "peer", &old_conn);
        assert!(peers.contains_key("peer"), "new registration must survive");

        unregister_if_owner(&mut peers, "peer", &new_conn);
        assert!(!peers.contains_key("peer"));
    }
}
