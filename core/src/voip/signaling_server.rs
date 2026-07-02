//! WebSocket signaling client (server fallback for WebRTC signaling).
//!
//! This connects to a signaling server when direct P2P signaling fails.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{signaling::SignalingMessage, Result, VoipError};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SignalingWireMessage {
    Register {
        peer_id: String,
    },
    Signal {
        from_peer_id: String,
        to_peer_id: String,
        payload: SignalingMessage,
    },
}

/// WebSocket signaling client used as fallback when P2P signaling fails.
#[derive(Clone)]
pub struct SignalingServerClient {
    outbound_tx: mpsc::UnboundedSender<SignalingWireMessage>,
    local_peer_id: PeerId,
}

impl SignalingServerClient {
    /// Connect to signaling server and start background tasks.
    pub async fn connect(
        url: String,
        local_peer_id: PeerId,
        inbound_tx: mpsc::UnboundedSender<(PeerId, SignalingMessage)>,
    ) -> Result<Self> {
        let ws_url = normalize_ws_url(&url);
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| VoipError::NetworkError(format!("Failed to connect signaling server: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        let register = SignalingWireMessage::Register {
            peer_id: local_peer_id.to_string(),
        };
        let register_json = serde_json::to_string(&register)
            .map_err(|e| VoipError::NetworkError(format!("Failed to serialize register: {}", e)))?;
        write
            .send(Message::Text(register_json))
            .await
            .map_err(|e| VoipError::NetworkError(format!("Failed to send register: {}", e)))?;

        let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<SignalingWireMessage>();

        let write_task = tokio::spawn(async move {
            while let Some(msg) = outbound_rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if write.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
        });

        let inbound_tx = Arc::new(Mutex::new(inbound_tx));
        let read_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(parsed) = serde_json::from_str::<SignalingWireMessage>(&text) {
                        if let SignalingWireMessage::Signal {
                            from_peer_id,
                            payload,
                            ..
                        } = parsed
                        {
                            if let Ok(peer_id) = from_peer_id.parse::<PeerId>() {
                                let sender = inbound_tx.lock().await;
                                let _ = sender.send((peer_id, payload));
                            }
                        }
                    }
                }
            }
        });

        tokio::spawn(async move {
            let _ = tokio::join!(write_task, read_task);
        });

        Ok(Self {
            outbound_tx,
            local_peer_id,
        })
    }

    /// Send a signaling message via the signaling server.
    pub fn send_signal(&self, to_peer_id: PeerId, signal: SignalingMessage) -> Result<()> {
        let msg = SignalingWireMessage::Signal {
            from_peer_id: self.local_peer_id.to_string(),
            to_peer_id: to_peer_id.to_string(),
            payload: signal,
        };

        self.outbound_tx
            .send(msg)
            .map_err(|e| VoipError::NetworkError(format!("Failed to send signaling message: {}", e)))?;

        Ok(())
    }
}

fn normalize_ws_url(url: &str) -> String {
    if url.starts_with("ws://") || url.starts_with("wss://") {
        return url.to_string();
    }
    if url.starts_with("https://") {
        return url.replacen("https://", "wss://", 1);
    }
    if url.starts_with("http://") {
        return url.replacen("http://", "ws://", 1);
    }
    format!("ws://{}", url)
}
