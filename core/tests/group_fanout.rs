//! P0-K: mensagem de grupo chega a TODOS os membros, inclusive a quem estava
//! offline. Alice cria o grupo com Bob (online, conectado) e Carol (offline,
//! nunca conectada): Bob recebe por P2P, Carol recebe pelo message store ao
//! drenar a caixa — convite, sender key e mensagem, nessa ordem.

mod common;

use std::{rc::Rc, time::Duration};

use common::spawn_fake_store;
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

async fn build(dir: &tempfile::TempDir, store_url: &str) -> Rc<Client> {
    Rc::new(
        ClientBuilder::new()
            .data_dir(dir.path().to_path_buf())
            .message_store_url(store_url.to_string())
            .build()
            .await
            .expect("build client"),
    )
}

/// Poll until `group_id` on `client` shows a message with this plaintext.
async fn wait_for_group_text(client: &Client, group_id: &str, text: &str, who: &str) {
    for _ in 0..150 {
        if let Ok(messages) = client
            .get_group_messages(group_id.to_string(), None, None)
            .await
        {
            if messages
                .iter()
                .any(|m| m.content_plaintext.as_deref() == Some(text))
            {
                return;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
    panic!("{who} never got the group message {text:?}");
}

#[tokio::test]
async fn group_message_reaches_online_and_offline_members() {
    let (store_url, mailbox) = spawn_fake_store().await;

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dirs: Vec<_> = (0..3).map(|_| tempfile::TempDir::new().unwrap()).collect();
            let alice = build(&dirs[0], &store_url).await;
            let bob = build(&dirs[1], &store_url).await;
            let carol = build(&dirs[2], &store_url).await;

            // Everyone knows everyone's bundle (as after scanning QR codes).
            let clients = [&alice, &bob, &carol];
            for from in clients {
                for to in clients {
                    if Rc::ptr_eq(from, to) {
                        continue;
                    }
                    from.set_contact_prekey_bundle(
                        to.local_peer_id().to_string(),
                        to.get_prekey_bundle_json().await.expect("bundle"),
                    )
                    .expect("store bundle");
                }
            }

            // Alice and Bob are online and connected. Carol is not running a
            // network loop at all: whatever is for her must wait on the store.
            for client in [&alice, &bob] {
                client
                    .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                    .await
                    .expect("listen");
                spawn_driver(Rc::clone(client));
            }
            let bob_addr = loop {
                let addrs = bob.listening_addresses().await;
                if let Some(addr) = addrs.iter().find(|a| a.contains("127.0.0.1")) {
                    break addr.clone();
                }
                sleep(Duration::from_millis(50)).await;
            };
            alice
                .connect_to_peer(bob.local_peer_id(), bob_addr.parse().unwrap())
                .await
                .expect("dial bob");

            let group = alice
                .create_group("Família".to_string(), None)
                .await
                .expect("create group");
            for member in [&bob, &carol] {
                alice
                    .add_group_member(group.id.clone(), member.local_peer_id().to_string())
                    .await
                    .expect("add member");
            }

            alice
                .send_group_message(group.id.clone(), "almoço domingo?".to_string())
                .await
                .expect("send group message");

            wait_for_group_text(&bob, &group.id, "almoço domingo?", "bob (online)").await;

            // Carol comes back: one mailbox drain brings the invite, the sender
            // key and the message.
            let carol_peer = carol.local_peer_id().to_string();
            let waiting = mailbox
                .lock()
                .unwrap()
                .iter()
                .filter(|m| m["recipient_peer_id"] == carol_peer.as_str())
                .count();
            assert!(waiting > 0, "nothing was left on the store for Carol");

            let processed = carol.fetch_offline_messages().await.expect("carol fetch");
            assert_eq!(processed, waiting, "everything for Carol must be processed");
            wait_for_group_text(&carol, &group.id, "almoço domingo?", "carol (offline)").await;

            // The mailbox is clean, and a second drain changes nothing.
            assert!(!mailbox
                .lock()
                .unwrap()
                .iter()
                .any(|m| m["recipient_peer_id"] == carol_peer.as_str()));
            assert_eq!(carol.fetch_offline_messages().await.expect("refetch"), 0);
            let carol_view = carol
                .get_group_messages(group.id.clone(), None, None)
                .await
                .unwrap();
            assert_eq!(
                carol_view
                    .iter()
                    .filter(|m| m.content_plaintext.as_deref() == Some("almoço domingo?"))
                    .count(),
                1
            );
        })
        .await;
}
