//! P0-D: mídia grande (acima do limite inline) de ponta a ponta entre dois
//! clientes reais. A oferta viaja dentro da sessão Signal e os chunks carregam
//! apenas o blob selado; o destinatário termina com o arquivo original.

use std::{rc::Rc, time::Duration};

use tokio::time::sleep;
use zaplivre_core::api::{Client, ClientBuilder};

fn spawn_driver(client: Rc<Client>) {
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
async fn large_media_is_transferred_sealed_and_opened_by_the_recipient() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir_a = tempfile::TempDir::new().unwrap();
            let dir_b = tempfile::TempDir::new().unwrap();
            let alice = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_a.path().to_path_buf())
                    .build()
                    .await
                    .expect("build alice"),
            );
            let bob = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_b.path().to_path_buf())
                    .build()
                    .await
                    .expect("build bob"),
            );
            for client in [&alice, &bob] {
                client
                    .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                    .await
                    .expect("listen");
                spawn_driver(Rc::clone(client));
            }

            let bob_peer = bob.local_peer_id();
            let alice_peer = alice.local_peer_id();
            alice
                .set_contact_prekey_bundle(
                    bob_peer.to_string(),
                    bob.get_prekey_bundle_json().await.expect("bob bundle"),
                )
                .expect("store bob bundle");

            let bob_addr = loop {
                let addrs = bob.listening_addresses().await;
                if let Some(addr) = addrs.iter().find(|a| a.contains("127.0.0.1")) {
                    break addr.clone();
                }
                sleep(Duration::from_millis(50)).await;
            };
            alice
                .connect_to_peer(bob_peer, bob_addr.parse().unwrap())
                .await
                .expect("dial bob");

            // 1.5 MiB with a non-trivial pattern: well above the inline limit
            // and spread over many 64 KiB chunks.
            let original: Vec<u8> = (0..1_572_864u32).map(|i| (i % 251) as u8).collect();
            let message_id = alice
                .send_document_message(
                    bob_peer,
                    &original,
                    "relatorio.pdf".to_string(),
                    "application/pdf".to_string(),
                )
                .await
                .expect("send document");

            // Bob learns about the media from the (encrypted) offer.
            let media = {
                let mut found = None;
                for _ in 0..100 {
                    if let Ok(list) = bob.get_message_media(&message_id) {
                        if let Some(media) = list.into_iter().next() {
                            found = Some(media);
                            break;
                        }
                    }
                    sleep(Duration::from_millis(100)).await;
                }
                found.expect("bob never received the media offer")
            };
            assert!(media.local_path.is_none(), "nothing downloaded yet");
            assert_eq!(media.file_size, Some(original.len() as i64));
            assert_eq!(media.file_name.as_deref(), Some("relatorio.pdf"));

            // The media is known by the hash of its sealed form, which says
            // nothing about the content.
            use sha2::{Digest, Sha256};
            assert_ne!(media.media_hash, format!("{:x}", Sha256::digest(&original)));

            let downloaded = bob
                .download_media(&media.media_hash)
                .await
                .expect("download");
            assert_eq!(
                downloaded, original,
                "bob must end up with the original file"
            );

            // On disk Bob keeps the plaintext only; the sealed .part is gone.
            let stored = bob.get_message_media(&message_id).unwrap().remove(0);
            let path = stored.local_path.expect("local path recorded");
            assert_eq!(std::fs::read(&path).unwrap(), original);
            let tmp = dir_b.path().join("media").join("tmp");
            assert!(std::fs::read_dir(&tmp).unwrap().next().is_none());

            // Both sides see the message in the conversation.
            let bob_view = bob
                .get_conversation_messages(&alice_peer.to_string(), None, None)
                .unwrap();
            assert!(bob_view.iter().any(|m| m.message_id == message_id));
        })
        .await;
}
