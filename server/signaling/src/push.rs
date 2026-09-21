//! Wake-up push for incoming calls (same service contract as the store's).
//!
//! When a call offer is for a peer with no WebSocket open, the push server is
//! asked to wake that device; the app then registers and receives the parked
//! offer. Nothing about the call goes in the push beyond who is calling.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// One wake-up per call is enough; retries of the same offer do not re-push.
const CALL_PUSH_MEMORY: Duration = Duration::from_secs(120);

pub struct CallPusher {
    client: reqwest::Client,
    push_server_url: String,
    service_secret: String,
    sent: Mutex<HashMap<String, Instant>>,
}

impl CallPusher {
    /// Built from `PUSH_SERVER_URL` and `PUSH_SERVICE_SECRET`; `None` (calls to
    /// closed apps then simply ring nowhere) when either is missing.
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("PUSH_SERVER_URL").ok()?;
        let secret = std::env::var("PUSH_SERVICE_SECRET").ok()?;
        if url.trim().is_empty() || secret.trim().is_empty() {
            return None;
        }
        Some(Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            push_server_url: url.trim_end_matches('/').to_string(),
            service_secret: secret,
            sent: Mutex::new(HashMap::new()),
        })
    }

    /// Fire-and-forget wake-up of `callee` for `call_id` from `caller`.
    pub async fn notify_incoming_call(&'static self, callee: &str, caller: &str, call_id: &str) {
        {
            let now = Instant::now();
            let mut sent = self.sent.lock().await;
            sent.retain(|_, at| now.duration_since(*at) < CALL_PUSH_MEMORY);
            if sent.insert(call_id.to_string(), now).is_some() {
                return;
            }
        }

        let payload = serde_json::json!({
            "peer_id": callee,
            "title": "Chamada recebida",
            "body": "Toque para atender",
            "data": {
                "type": "incoming_call",
                "sender_peer_id": caller,
                "call_id": call_id,
            }
        });
        let url = format!("{}/api/v1/send", self.push_server_url);
        tokio::spawn(async move {
            match self
                .client
                .post(url)
                .bearer_auth(&self.service_secret)
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("🔔 Call wake-up push sent");
                }
                Ok(resp) => tracing::warn!("⚠️ Push server returned {} for a call", resp.status()),
                Err(e) => tracing::warn!("⚠️ Failed to reach push server: {}", e),
            }
        });
    }
}
