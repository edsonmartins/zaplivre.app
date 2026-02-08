//! MePassa Signaling Server
//!
//! WebSocket relay for WebRTC signaling fallback.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

#[derive(Clone, Default)]
struct AppState {
    peers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireMessage {
    Register { peer_id: String },
    Signal {
        from_peer_id: String,
        to_peer_id: String,
        payload: serde_json::Value,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mepassa_signaling=info,info")
        .init();

    let state = AppState::default();

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8086));
    info!("📡 Signaling server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut registered_peer: Option<String> = None;

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            let parsed = serde_json::from_str::<WireMessage>(&text);
            match parsed {
                Ok(WireMessage::Register { peer_id }) => {
                    info!("🔌 Peer registered: {}", peer_id);
                    state.peers.write().await.insert(peer_id.clone(), out_tx.clone());
                    registered_peer = Some(peer_id);
                }
                Ok(WireMessage::Signal {
                    from_peer_id,
                    to_peer_id,
                    payload,
                }) => {
                    let msg = WireMessage::Signal {
                        from_peer_id,
                        to_peer_id: to_peer_id.clone(),
                        payload,
                    };
                    if let Ok(text) = serde_json::to_string(&msg) {
                        if let Some(target) = state.peers.read().await.get(&to_peer_id) {
                            let _ = target.send(Message::Text(text));
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
        state.peers.write().await.remove(&peer_id);
    }

    send_task.abort();
}
