//! Plaintext fallback policy tests (A6 / SEC-01)
//!
//! Política: mensagens sem sessão E2E NUNCA caem em plaintext por padrão.
//! O downgrade só acontece explicitamente com `ZAPLIVRE_ALLOW_PLAINTEXT=true`.
//!
//! Este binário é **isolado** (próprio processo) para que o `set_var` no
//! ambiente não vaze para os demais testes de integração (que rodam em
//! paralelo no mesmo binário `message_integration`).

use libp2p::PeerId;
use std::{rc::Rc, time::Duration};
use tokio::time::sleep;
use zaplivre_core::api::ClientBuilder;

const ENV_PLAINTEXT: &str = "ZAPLIVRE_ALLOW_PLAINTEXT";

#[tokio::test]
async fn test_plaintext_downgrade_policy() {
    // Estado limpo: garante que o teste parte do comportamento default.
    std::env::remove_var(ENV_PLAINTEXT);

    let local = tokio::task::LocalSet::new();
    let result = local
        .run_until(async {
            let dir_a = tempfile::TempDir::new().unwrap();
            let dir_b = tempfile::TempDir::new().unwrap();

            let client_a = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_a.path().to_path_buf())
                    .build()
                    .await
                    .expect("build client A"),
            );
            // A política de recepção é fixada na construção do cliente: só o B,
            // criado com a env ligada, aceita plaintext. O A nasce no default.
            std::env::set_var(ENV_PLAINTEXT, "true");
            let client_b = Rc::new(
                ClientBuilder::new()
                    .data_dir(dir_b.path().to_path_buf())
                    .build()
                    .await
                    .expect("build client B"),
            );
            std::env::remove_var(ENV_PLAINTEXT);

            client_a
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen A");
            client_b
                .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
                .await
                .expect("listen B");

            // Drivers de rede (equivalente ao poll loop do FFI)
            for client in [Rc::clone(&client_a), Rc::clone(&client_b)] {
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

            // Aguardar o endereço de escuta do B e conectar A → B (sem sessão E2E)
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

            // 1) SEM ZAPLIVRE_ALLOW_PLAINTEXT: sem sessão E2E, o envio FALHA
            //    (nunca plaintext) — SEC-01.
            let err = client_a
                .send_text_message(peer_b, "não deve sair".to_string())
                .await
                .expect_err("envio sem sessão E2E e sem env deveria falhar");
            let err_str = err.to_string();
            assert!(
                err_str.contains("plaintext fallback is disabled"),
                "erro inesperado: {err_str}"
            );

            // 2) COM ZAPLIVRE_ALLOW_PLAINTEXT=true: o downgrade explícito é
            //    permitido e a mensagem (plaintext) chega ao destinatário.
            std::env::set_var(ENV_PLAINTEXT, "true");
            let message_id = client_a
                .send_text_message(peer_b, "fallback plaintext explícito".to_string())
                .await
                .expect("env permitindo plaintext deve enviar");

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
            assert!(received, "Peer B nunca recebeu a mensagem plaintext");

            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await;

    // Restaura o ambiente (não vazar para outros processos/binários).
    std::env::remove_var(ENV_PLAINTEXT);

    result.expect("teste de política plaintext falhou");
}
