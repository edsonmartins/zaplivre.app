//! M3: o cliente obtém ICE servers do `turn-credentials` REAL, com request
//! assinado. Exige o servidor no ar:
//!
//! ```text
//! TURN_STATIC_SECRET=... TURN_HOST=turn.example SERVER_PORT=18082 \
//!     cargo run -p turn-credentials
//! TURN_CREDENTIALS_URL=http://127.0.0.1:18082 \
//!     cargo test -p zaplivre-core --test ice_servers -- --ignored
//! ```

use zaplivre_core::api::ClientBuilder;

#[tokio::test]
#[ignore = "requires a running turn-credentials server (TURN_CREDENTIALS_URL)"]
async fn client_gets_ice_servers_from_the_real_credentials_server() {
    let url = std::env::var("TURN_CREDENTIALS_URL").expect("TURN_CREDENTIALS_URL");

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::TempDir::new().unwrap();
            let client = ClientBuilder::new()
                .data_dir(dir.path().to_path_buf())
                .turn_credentials_url(url)
                .build()
                .await
                .expect("build client");

            let servers = client.ice_servers().await.expect("signed request accepted");

            let turn = servers
                .iter()
                .find(|s| s.urls.iter().any(|u| u.starts_with("turn:")))
                .expect("a TURN entry");
            assert!(turn.username.as_deref().is_some_and(|u| !u.is_empty()));
            assert!(turn.credential.as_deref().is_some_and(|c| !c.is_empty()));
            assert!(
                servers
                    .iter()
                    .any(|s| s.urls.iter().all(|u| u.starts_with("stun:")) && s.username.is_none()),
                "a STUN entry on our own host"
            );

            // Without a configured server the app gets a clear error.
            let dir = tempfile::TempDir::new().unwrap();
            let unconfigured = ClientBuilder::new()
                .data_dir(dir.path().to_path_buf())
                .build()
                .await
                .expect("build client");
            assert!(unconfigured.ice_servers().await.is_err());
        })
        .await;
}
