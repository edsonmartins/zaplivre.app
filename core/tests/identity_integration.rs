//! Integration Tests - Identity Client + Storage
//!
//! Tests the full integration:
//! 1. Register username on Identity Server
//! 2. Store contact locally in SQLite
//! 3. Lookup username from Identity Server
//! 4. Store remote contact locally
//! 5. Query contacts from local storage
//!
//! Requires Identity Server running on http://localhost:8083
//! (override with IDENTITY_SERVER_URL)
//!
//! Run with: cargo test --test identity_integration --features integration-tests

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use zaplivre_core::{
        identity::Identity,
        identity_client::IdentityClient,
        storage::{Database, NewContact},
    };

    /// URL do identity-server (8083 por padrão; 8080 é o message-store)
    fn identity_server_url() -> String {
        std::env::var("IDENTITY_SERVER_URL").unwrap_or_else(|_| "http://localhost:8083".to_string())
    }

    /// Setup: Create in-memory database with schema
    fn setup_db() -> Database {
        let db = Database::in_memory().unwrap();
        zaplivre_core::storage::migrate(&db).unwrap();
        db
    }

    #[tokio::test]
    async fn test_full_flow_register_and_store() {
        // 1. Generate identity
        let alice = Identity::generate(10);
        let peer_id = format!("12D3KooW{}", rand::random::<u64>());
        let username = format!("alice_{}", rand::random::<u32>());

        // 2. Register on Identity Server
        let client = IdentityClient::new(&identity_server_url()).unwrap();

        let response = client
            .register_username(&alice, &username, &peer_id)
            .await
            .unwrap();

        assert_eq!(response.username, username);
        assert_eq!(response.peer_id, peer_id);

        // 3. Store contact locally
        let db = setup_db();

        let contact = NewContact {
            peer_id: response.peer_id.clone(),
            username: Some(response.username.clone()),
            display_name: Some("Alice Wonderland".to_string()),
            public_key: alice.keypair().public_key_bytes().to_vec(),
            prekey_bundle_json: None,
        };

        db.insert_contact(&contact).unwrap();

        // 4. Query from local storage
        let stored = db.get_contact_by_username(&username).unwrap();

        assert_eq!(stored.username, Some(username));
        assert_eq!(stored.peer_id, peer_id);
        assert_eq!(stored.display_name, Some("Alice Wonderland".to_string()));
    }

    #[tokio::test]
    async fn test_lookup_and_cache_locally() {
        // Setup: Alice registers
        let alice = Identity::generate(10);
        let alice_peer_id = format!("12D3KooW{}", rand::random::<u64>());
        let alice_username = format!("alice_{}", rand::random::<u32>());

        let client = IdentityClient::new(&identity_server_url()).unwrap();

        client
            .register_username(&alice, &alice_username, &alice_peer_id)
            .await
            .unwrap();

        // Bob looks up Alice
        let db = setup_db();

        let lookup_response = client.lookup_username(&alice_username).await.unwrap();

        assert_eq!(lookup_response.username, alice_username);
        assert_eq!(lookup_response.peer_id, alice_peer_id);

        // Bob saves Alice to local contacts
        let prekey_bundle_json = serde_json::to_string(&lookup_response.prekey_bundle).unwrap();

        let contact = NewContact {
            peer_id: lookup_response.peer_id.clone(),
            username: Some(lookup_response.username.clone()),
            display_name: None,
            public_key: alice.keypair().public_key_bytes().to_vec(),
            prekey_bundle_json: Some(prekey_bundle_json.clone()),
        };

        db.insert_contact(&contact).unwrap();

        // Bob queries Alice from local storage (cache hit)
        let cached = db.get_contact_by_username(&alice_username).unwrap();

        assert_eq!(cached.username, Some(alice_username));
        assert_eq!(cached.peer_id, alice_peer_id);
        assert_eq!(cached.prekey_bundle_json, Some(prekey_bundle_json));
    }

    #[tokio::test]
    async fn test_update_prekeys_and_refresh_cache() {
        // Alice registers
        let alice = Identity::generate(10);
        let peer_id = format!("12D3KooW{}", rand::random::<u64>());
        let username = format!("alice_{}", rand::random::<u32>());

        let client = IdentityClient::new(&identity_server_url()).unwrap();

        client
            .register_username(&alice, &username, &peer_id)
            .await
            .unwrap();

        // Alice updates prekeys
        let update_response = client.update_prekeys(&alice, &peer_id).await.unwrap();

        assert!(update_response.updated_at > chrono::Utc::now() - chrono::Duration::seconds(10));

        // Bob looks up Alice again (should get new prekeys)
        let lookup = client.lookup_username(&username).await.unwrap();

        assert_eq!(lookup.username, username);

        // Bob updates local cache
        let db = setup_db();

        let contact = NewContact {
            peer_id: lookup.peer_id.clone(),
            username: Some(lookup.username.clone()),
            display_name: None,
            public_key: alice.keypair().public_key_bytes().to_vec(),
            prekey_bundle_json: Some(serde_json::to_string(&lookup.prekey_bundle).unwrap()),
        };

        db.insert_contact(&contact).unwrap();

        let cached = db.get_contact_by_username(&username).unwrap();

        assert!(cached.prekey_bundle_json.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_username_local_and_remote() {
        let username = format!("alice_{}", rand::random::<u32>());

        // Alice registers
        let alice = Identity::generate(10);
        let alice_peer_id = format!("12D3KooW{}", rand::random::<u64>());

        let client = IdentityClient::new(&identity_server_url()).unwrap();

        client
            .register_username(&alice, &username, &alice_peer_id)
            .await
            .unwrap();

        // Save locally
        let db = setup_db();

        let contact = NewContact {
            peer_id: alice_peer_id.clone(),
            username: Some(username.clone()),
            display_name: None,
            public_key: alice.keypair().public_key_bytes().to_vec(),
            prekey_bundle_json: None,
        };

        db.insert_contact(&contact).unwrap();

        // Eve tries to register same username (should fail on server)
        let eve = Identity::generate(10);
        let eve_peer_id = format!("12D3KooW{}", rand::random::<u64>());

        let result = client
            .register_username(&eve, &username, &eve_peer_id)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("USERNAME_TAKEN"));

        // Eve tries to save locally with same username (should also fail)
        let eve_contact = NewContact {
            peer_id: eve_peer_id,
            username: Some(username.clone()),
            display_name: None,
            public_key: eve.keypair().public_key_bytes().to_vec(),
            prekey_bundle_json: None,
        };

        let result = db.insert_contact(&eve_contact);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_contacts_after_registration() {
        let db = setup_db();
        let client = IdentityClient::new(&identity_server_url()).unwrap();

        // Register multiple users
        let users = vec![
            ("alice", "Alice Wonderland"),
            ("bob", "Bob Builder"),
            ("carol", "Carol Singer"),
        ];

        for (username, display_name) in &users {
            let identity = Identity::generate(5);
            let peer_id = format!("12D3KooW{}", rand::random::<u64>());
            let full_username = format!("{}_{}", username, rand::random::<u16>());

            // Register on server
            client
                .register_username(&identity, &full_username, &peer_id)
                .await
                .unwrap();

            // Save locally
            let contact = NewContact {
                peer_id,
                username: Some(full_username),
                display_name: Some(display_name.to_string()),
                public_key: identity.keypair().public_key_bytes().to_vec(),
                prekey_bundle_json: None,
            };

            db.insert_contact(&contact).unwrap();
        }

        // Search for "alice"
        let results = db.search_contacts("alice").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].display_name,
            Some("Alice Wonderland".to_string())
        );

        // Search for "Builder"
        let results = db.search_contacts("Builder").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].username.as_ref().unwrap().starts_with("bob_"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = IdentityClient::new(&identity_server_url()).unwrap();
        let health = client.health_check().await.unwrap();

        assert_eq!(health["status"], "healthy");
        assert_eq!(health["database"]["status"], "connected");
        assert_eq!(health["redis"]["status"], "connected");
    }
}
