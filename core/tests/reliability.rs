//! Testes de confiabilidade (TST-03)
//!
//! CORE-02: mensagem para peer offline (sem store) cai na fila local de
//! retry e é entregue pelo worker quando o peer aparece.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use zaplivre_core::{api::ClientBuilder, storage::MessageStatus};

fn spawn_driver(client: Arc<zaplivre_core::api::Client>) {
    tokio::task::spawn_local(async move {
        loop {
            match client.poll_network_once().await {
                Ok(true) => {}
                Ok(false) => sleep(Duration::from_millis(5)).await,
                Err(_) => sleep(Duration::from_millis(50)).await,
            }
        }
    });
}

#[tokio::test]
async fn test_offline_send_queues_and_retry_delivers() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    // Sem store server: o fallback offline é a fila local
    std::env::remove_var("MESSAGE_STORE_URL");

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir_a = tempfile::TempDir::new().unwrap();
            let dir_b = tempfile::TempDir::new().unwrap();

            let client_a = Arc::new(
                ClientBuilder::new()
                    .data_dir(dir_a.path().to_path_buf())
                    .build()
                    .await
                    .expect("build client A"),
            );
            // B é construído mas ainda NÃO escuta - está "offline"
            let client_b = Arc::new(
                ClientBuilder::new()
                    .data_dir(dir_b.path().to_path_buf())
                    .build()
                    .await
                    .expect("build client B"),
            );
            let peer_b = client_b.local_peer_id();

            client_a
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen A");
            spawn_driver(Arc::clone(&client_a));

            // Enviar com B fora do ar: não pode falhar nem descartar -
            // precisa ficar na fila (CORE-02)
            let message_id = client_a
                .send_text_message(peer_b, "mensagem para peer offline".to_string())
                .await
                .expect("send to offline peer must queue, not fail");

            let peer_b_str = peer_b.to_string();
            let queued = client_a
                .get_conversation_messages(&peer_b_str, None, None)
                .unwrap_or_default()
                .into_iter()
                .find(|m| m.message_id == message_id)
                .expect("queued message must be visible in conversation");
            assert_ne!(
                queued.status,
                MessageStatus::Delivered,
                "message can't be Delivered while B is offline"
            );

            // B entra no ar
            client_b
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen B");
            spawn_driver(Arc::clone(&client_b));

            let addr_b = {
                let mut found = None;
                for _ in 0..100 {
                    let addrs = client_b.listening_addresses().await;
                    if let Some(a) = addrs.iter().find(|a| a.contains("127.0.0.1")) {
                        found = Some(a.clone());
                        break;
                    }
                    sleep(Duration::from_millis(50)).await;
                }
                found.expect("B nunca reportou endereço")
            };
            client_a
                .connect_to_peer(peer_b, addr_b.parse().unwrap())
                .await
                .expect("dial B");

            // Worker de retry: primeiro attempt em +5s, tick de 5s -> a
            // entrega deve acontecer em bem menos de 30s
            let peer_a_str = client_a.local_peer_id().to_string();
            let mut received = false;
            for _ in 0..300 {
                let messages = client_b
                    .get_conversation_messages(&peer_a_str, None, None)
                    .unwrap_or_default();
                if messages.iter().any(|m| m.message_id == message_id) {
                    received = true;
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            assert!(
                received,
                "retry worker never delivered the queued message after B came online"
            );

            // E o remetente precisa ver Delivered (ACK do retry)
            let mut delivered = false;
            for _ in 0..100 {
                let messages = client_a
                    .get_conversation_messages(&peer_b_str, None, None)
                    .unwrap_or_default();
                if messages
                    .iter()
                    .any(|m| m.message_id == message_id && m.status == MessageStatus::Delivered)
                {
                    delivered = true;
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            assert!(delivered, "sender never saw Delivered after retry delivery");
        })
        .await;
}

/// SEC-07: o bundle de prekeys publicado precisa ser o MESMO após restart -
/// bundle novo a cada boot invalida o que os contatos já buscaram
#[tokio::test]
async fn test_prekey_bundle_stable_across_restarts() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::TempDir::new().unwrap();
            // Identidade fixa via env NÃO é usada aqui: o builder gera e
            // persiste identity.key no data_dir, então o segundo boot reusa
            let bundle_first = {
                let client = ClientBuilder::new()
                    .data_dir(dir.path().to_path_buf())
                    .build()
                    .await
                    .expect("build #1");
                client.get_prekey_bundle_json().await.expect("bundle #1")
            };

            let bundle_second = {
                let client = ClientBuilder::new()
                    .data_dir(dir.path().to_path_buf())
                    .build()
                    .await
                    .expect("build #2");
                client.get_prekey_bundle_json().await.expect("bundle #2")
            };

            // O one_time_prekey é sorteado do pool a cada chamada (semântica
            // Signal); o que precisa ser estável é todo o resto: identidade,
            // signed prekey (e assinatura) e kyber prekey
            let first: serde_json::Value = serde_json::from_str(&bundle_first).unwrap();
            let second: serde_json::Value = serde_json::from_str(&bundle_second).unwrap();
            for field in [
                "identity_key",
                "signal_identity_key",
                "signal_registration_id",
                "signal_device_id",
                "signed_prekey_id",
                "signed_prekey",
                "signed_prekey_signature",
                "kyber_prekey_id",
                "kyber_prekey",
                "kyber_prekey_signature",
            ] {
                assert_eq!(
                    first.get(field),
                    second.get(field),
                    "bundle field '{}' changed across restart - pool not persisted",
                    field
                );
            }
        })
        .await;
}
