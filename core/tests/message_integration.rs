//! Message Integration Tests
//!
//! - Teste E2E REAL (TST-02): dois Clients completos trocando mensagem por
//!   TCP local, com verificação de recepção no destinatário e de ACK
//!   (status Delivered) no remetente
//! - Testes do MessageHandler (processamento + ACK) com a API atual

use libp2p::PeerId;
use mepassa_core::{
    api::ClientBuilder,
    crypto::SignalSessionManager,
    identity::Identity,
    network::{MessageEvent, MessageHandler},
    protocol::{pb::message::Payload, AckStatus, Message, MessageType, TextMessage},
    storage::{schema::init_schema, Database, MessageStatus},
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::sleep};
use uuid::Uuid;

/// TST-02: troca de mensagem ponta a ponta entre dois Clients reais.
/// Roda em LocalSet (requisito do ClientBuilder) com um driver de rede por
/// client (mesmo papel do loop do FFI em produção).
#[tokio::test]
async fn test_end_to_end_message_exchange() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

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
            let client_b = Arc::new(
                ClientBuilder::new()
                    .data_dir(dir_b.path().to_path_buf())
                    .build()
                    .await
                    .expect("build client B"),
            );

            client_a
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen A");
            client_b
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen B");

            // Drivers de rede (equivalente ao poll loop do FFI)
            for client in [Arc::clone(&client_a), Arc::clone(&client_b)] {
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

            // Aguardar o endereço de escuta do B
            let addr_b = {
                let mut found = None;
                for _ in 0..100 {
                    let addrs = client_b.listening_addresses().await;
                    if let Some(addr) = addrs.iter().find(|a| a.contains("127.0.0.1")) {
                        found = Some(addr.clone());
                        break;
                    }
                    sleep(Duration::from_millis(50)).await;
                }
                found.expect("B nunca reportou endereço de escuta")
            };

            let peer_b: PeerId = client_b.local_peer_id();
            client_a
                .connect_to_peer(peer_b, addr_b.parse().unwrap())
                .await
                .expect("dial B");

            // Enviar (ensure_peer_connected aguarda a conexão - CORE-01)
            let message_id = client_a
                .send_text_message(peer_b, "Hello E2E!".to_string())
                .await
                .expect("send message");

            // 1) B deve receber e armazenar a mensagem
            let peer_a_str = client_a.local_peer_id().to_string();
            let mut received = false;
            for _ in 0..200 {
                let messages = client_b
                    .get_conversation_messages(&peer_a_str, None, None)
                    .unwrap_or_default();
                if messages.iter().any(|m| m.message_id == message_id) {
                    received = true;
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
            assert!(received, "Peer B nunca recebeu a mensagem {}", message_id);

            // 2) A deve ver o status Delivered (ACK de B)
            let peer_b_str = peer_b.to_string();
            let mut delivered = false;
            for _ in 0..200 {
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
            assert!(
                delivered,
                "ACK nunca chegou - status da mensagem não virou Delivered no remetente"
            );
        })
        .await;
}

/// Helper: constrói um MessageHandler com a API atual (7 args)
fn build_test_handler(
    local_peer_id: &str,
    db: &Database,
    data_dir: std::path::PathBuf,
    event_tx: tokio::sync::mpsc::UnboundedSender<MessageEvent>,
) -> MessageHandler {
    let mut identity = Identity::generate(1);
    let storage_key = identity.storage_key().expect("storage key");
    let identity = Arc::new(RwLock::new(identity));
    let session_manager = SignalSessionManager::new(Arc::clone(&identity));

    MessageHandler::new(
        local_peer_id.to_string(),
        Arc::new(db.clone()),
        data_dir,
        identity,
        session_manager,
        storage_key,
        Some(event_tx),
    )
}

#[tokio::test]
async fn test_message_handler_processing() {
    let db = Database::in_memory().expect("Failed to create database");
    init_schema(&db).expect("Failed to init schema");

    // Sender precisa existir como contato (FK)
    let sender_peer = PeerId::random();
    use mepassa_core::storage::contacts::NewContact;
    let contact = NewContact {
        peer_id: sender_peer.to_string(),
        username: None,
        display_name: Some("Test Sender".to_string()),
        public_key: vec![1, 2, 3],
        prekey_bundle_json: None,
    };
    db.insert_contact(&contact).expect("Failed to insert contact");

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let tmp = tempfile::TempDir::new().unwrap();
    let handler = build_test_handler("local-peer", &db, tmp.path().to_path_buf(), event_tx);

    let message_id = Uuid::new_v4().to_string();
    let message = Message {
        id: message_id.clone(),
        sender_peer_id: sender_peer.to_string(),
        recipient_peer_id: "local-peer".to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        r#type: MessageType::Text as i32,
        payload: Some(Payload::Text(TextMessage {
            content: "Test message content".to_string(),
            reply_to_id: String::new(),
            metadata: std::collections::HashMap::new(),
        })),
    };

    let ack = handler
        .handle_incoming_message(sender_peer, message)
        .await
        .expect("Failed to handle message");

    assert_eq!(ack.message_id, message_id);
    assert_eq!(ack.status, AckStatus::Received as i32);

    // Evento emitido
    let event = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");
    match event {
        MessageEvent::MessageReceived {
            message_id: recv_msg_id,
            content,
            ..
        } => {
            assert_eq!(recv_msg_id, message_id);
            assert_eq!(content, "Test message content");
        }
        _ => panic!("Expected MessageReceived event"),
    }

    // Persistido (conteúdo cifrado at-rest; plaintext não é gravado)
    let stored = db.get_message(&message_id).expect("Failed to get message");
    assert_eq!(stored.message_id, message_id);
    assert_eq!(stored.status, MessageStatus::Delivered);
    assert!(
        stored.content_encrypted.is_some() || stored.content_plaintext.is_some(),
        "message content missing"
    );
}

#[tokio::test]
async fn test_ack_handling() {
    let db = Database::in_memory().expect("Failed to create database");
    init_schema(&db).expect("Failed to init schema");

    use mepassa_core::storage::contacts::NewContact;
    let local_peer_id = "local-peer".to_string();
    let remote_peer_id = PeerId::random().to_string();

    for (peer, name, key) in [
        (&local_peer_id, "Local", vec![1u8, 2, 3]),
        (&remote_peer_id, "Remote", vec![4u8, 5, 6]),
    ] {
        db.insert_contact(&NewContact {
            peer_id: peer.clone(),
            username: None,
            display_name: Some(name.to_string()),
            public_key: key,
            prekey_bundle_json: None,
        })
        .expect("Failed to insert contact");
    }

    let conversation_id = db
        .get_or_create_conversation(&remote_peer_id)
        .expect("Failed to create conversation");

    use mepassa_core::storage::messages::NewMessage;
    let message_id = Uuid::new_v4().to_string();
    db.insert_message(&NewMessage {
        message_id: message_id.clone(),
        conversation_id,
        sender_peer_id: local_peer_id.clone(),
        recipient_peer_id: Some(remote_peer_id.clone()),
        message_type: "text".to_string(),
        content_encrypted: None,
        content_plaintext: Some("Test message".to_string()),
        status: MessageStatus::Sent,
        parent_message_id: None,
    })
    .expect("Failed to insert message");

    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let tmp = tempfile::TempDir::new().unwrap();
    let handler = build_test_handler(&local_peer_id, &db, tmp.path().to_path_buf(), event_tx);

    use mepassa_core::protocol::AckMessage;
    handler
        .handle_outgoing_ack(AckMessage {
            message_id: message_id.clone(),
            status: AckStatus::Received as i32,
            error: String::new(),
        })
        .await
        .expect("Failed to handle ACK");

    let message = db.get_message(&message_id).expect("Failed to get message");
    assert_eq!(message.status, MessageStatus::Delivered);

    let event = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
        .await
        .expect("Timeout waiting for event")
        .expect("No event received");
    match event {
        MessageEvent::MessageDelivered {
            message_id: delivered_msg_id,
            status,
            ..
        } => {
            assert_eq!(delivered_msg_id, message_id);
            assert_eq!(status, MessageStatus::Delivered);
        }
        _ => panic!("Expected MessageDelivered event"),
    }
}
