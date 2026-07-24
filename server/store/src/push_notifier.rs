//! Integração store -> push server (PSH-02)
//!
//! Quando uma mensagem entra no store (destinatário offline), dispara uma
//! notificação push genérica via push-server para acordar o app do
//! destinatário. O conteúdo NUNCA vai no push (E2E) - apenas metadados
//! mínimos para navegação.

#[derive(Clone)]
pub struct PushNotifier {
    client: reqwest::Client,
    push_server_url: Option<String>,
    service_secret: Option<String>,
}

impl PushNotifier {
    pub fn new(push_server_url: Option<String>, service_secret: Option<String>) -> Self {
        if push_server_url.is_none() {
            tracing::info!("ℹ️ PUSH_SERVER_URL not set - offline messages will not trigger push");
        }
        Self {
            client: reqwest::Client::new(),
            push_server_url: push_server_url.map(|url| url.trim_end_matches('/').to_string()),
            service_secret,
        }
    }

    /// Fire-and-forget: notifica o destinatário de uma mensagem offline
    pub fn notify_offline_message(&self, recipient_peer_id: &str, sender_peer_id: &str) {
        let Some(base_url) = self.push_server_url.clone() else {
            return;
        };
        let client = self.client.clone();
        let Some(service_secret) = self.service_secret.clone() else {
            tracing::error!("PUSH_SERVICE_SECRET missing - push notification suppressed");
            return;
        };
        let recipient = recipient_peer_id.to_string();
        let sender = sender_peer_id.to_string();

        tokio::spawn(async move {
            let payload = serde_json::json!({
                "peer_id": recipient,
                "title": "Nova mensagem",
                "body": "Você recebeu uma nova mensagem",
                "data": {
                    "type": "offline_message",
                    "sender_peer_id": sender,
                }
            });

            match client
                .post(format!("{}/api/v1/send", base_url))
                .bearer_auth(service_secret)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    tracing::debug!("🔔 Push notification triggered for {}", recipient);
                }
                Ok(resp) => {
                    tracing::warn!(
                        "⚠️ Push server returned {} for offline notification",
                        resp.status()
                    );
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to reach push server: {}", e);
                }
            }
        });
    }
}
