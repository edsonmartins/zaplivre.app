//! Store-and-forward de ponta a ponta contra um message store falso (HTTP):
//! o remetente deixa a mensagem no store porque o destinatário está offline, o
//! destinatário drena a caixa, e só o que foi processado é apagado do servidor.

mod common;

use std::{rc::Rc, time::Duration};

use common::spawn_fake_store;
use serde_json::json;
use zaplivre_core::api::ClientBuilder;

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
