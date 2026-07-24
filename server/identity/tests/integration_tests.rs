//! Integration Tests for Identity Server
//!
//! These tests require the Identity Server to be running on http://localhost:8083
//! and PostgreSQL + Redis to be available (`make up`).
//!
//! Run with: cargo test --test integration_tests -- --ignored --test-threads=1

use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;

const BASE_URL: &str = "http://localhost:8083";

#[derive(Debug, Serialize, Deserialize)]
struct PreKeyBundle {
    identity_key: String,
    signed_prekey_id: i32,
    signed_prekey: String,
    signed_prekey_signature: String,
    one_time_prekey: Option<OneTimePreKey>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OneTimePreKey {
    id: i32,
    public_key: String,
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    username: String,
    peer_id: String,
    public_key: String,
    prekey_bundle: PreKeyBundle,
    signature: String,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    username: String,
    peer_id: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct LookupResponse {
    username: String,
    peer_id: String,
    prekey_bundle: PreKeyBundle,
    last_updated: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    message: String,
    suggestions: Option<Vec<String>>,
}

/// Generate a random username for testing
fn random_username() -> String {
    format!("test_{}", rand::random::<u32>())
}

/// Generate a random peer_id for testing
fn random_peer_id() -> String {
    format!("12D3KooW{}", rand::random::<u64>())
}

/// Create a dummy Ed25519 keypair and signature (SEC-14: a mensagem cobre
/// username + peer_id + public_key + timestamp)
fn create_test_signature(username: &str, peer_id: &str, timestamp: i64) -> (String, String) {
    use base64::{engine::general_purpose, Engine as _};
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let public_key = general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());

    let message = format!(
        "register:{}:{}:{}:{}",
        username, peer_id, public_key, timestamp
    );
    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());

    (public_key, signature_b64)
}

/// Create a dummy prekey bundle
fn create_test_prekey_bundle() -> PreKeyBundle {
    use base64::{engine::general_purpose, Engine as _};
    PreKeyBundle {
        identity_key: general_purpose::STANDARD.encode(vec![0u8; 32]),
        signed_prekey_id: 1,
        signed_prekey: general_purpose::STANDARD.encode(vec![1u8; 32]),
        signed_prekey_signature: general_purpose::STANDARD.encode(vec![2u8; 64]),
        one_time_prekey: Some(OneTimePreKey {
            id: 1,
            public_key: general_purpose::STANDARD.encode(vec![3u8; 32]),
        }),
    }
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_health_check() {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health", BASE_URL))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let health: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    assert_eq!(health["status"], "healthy");
    assert_eq!(health["database"]["status"], "connected");
    assert_eq!(health["redis"]["status"], "connected");
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_register_username_success() {
    let client = reqwest::Client::new();
    let username = random_username();
    let peer_id = random_peer_id();
    let timestamp = chrono::Utc::now().timestamp();

    let (public_key, signature) = create_test_signature(&username, &peer_id, timestamp);

    let request = RegisterRequest {
        username: username.clone(),
        peer_id: peer_id.clone(),
        public_key,
        prekey_bundle: create_test_prekey_bundle(),
        signature,
        timestamp,
    };

    let response = client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let register_response: RegisterResponse = response.json().await.expect("Failed to parse JSON");

    assert_eq!(register_response.username, username);
    assert_eq!(register_response.peer_id, peer_id);
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_lookup_username_success() {
    let client = reqwest::Client::new();
    let username = random_username();
    let peer_id = random_peer_id();
    let timestamp = chrono::Utc::now().timestamp();

    let (public_key, signature) = create_test_signature(&username, &peer_id, timestamp);

    // First, register the username
    let request = RegisterRequest {
        username: username.clone(),
        peer_id: peer_id.clone(),
        public_key,
        prekey_bundle: create_test_prekey_bundle(),
        signature,
        timestamp,
    };

    client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request)
        .send()
        .await
        .expect("Failed to register");

    // Then, lookup the username
    let response = client
        .get(&format!("{}/api/v1/lookup?username={}", BASE_URL, username))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let lookup_response: LookupResponse = response.json().await.expect("Failed to parse JSON");

    assert_eq!(lookup_response.username, username);
    assert_eq!(lookup_response.peer_id, peer_id);
    assert!(lookup_response.prekey_bundle.one_time_prekey.is_some());
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_register_duplicate_username() {
    let client = reqwest::Client::new();
    let username = random_username();
    let timestamp = chrono::Utc::now().timestamp();

    let peer_id1 = random_peer_id();
    let (public_key1, signature1) = create_test_signature(&username, &peer_id1, timestamp);

    // Register first user
    let request1 = RegisterRequest {
        username: username.clone(),
        peer_id: peer_id1,
        public_key: public_key1,
        prekey_bundle: create_test_prekey_bundle(),
        signature: signature1,
        timestamp,
    };

    let response1 = client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request1)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response1.status(), 200);

    // Try to register same username with different peer_id (should fail)
    let timestamp2 = chrono::Utc::now().timestamp();
    let peer_id2 = random_peer_id();
    let (public_key2, signature2) = create_test_signature(&username, &peer_id2, timestamp2);

    let request2 = RegisterRequest {
        username: username.clone(),
        peer_id: peer_id2,
        public_key: public_key2,
        prekey_bundle: create_test_prekey_bundle(),
        signature: signature2,
        timestamp: timestamp2,
    };

    let response2 = client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request2)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response2.status(), 409); // Conflict

    let error: ErrorResponse = response2.json().await.expect("Failed to parse JSON");

    assert_eq!(error.error, "USERNAME_TAKEN");
    assert!(error.message.contains(&username));
    assert!(error.suggestions.is_some()); // Should provide username suggestions
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_lookup_nonexistent_username() {
    let client = reqwest::Client::new();
    let username = format!("nonexistent_{}", rand::random::<u32>());

    let response = client
        .get(&format!("{}/api/v1/lookup?username={}", BASE_URL, username))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 404); // Not Found

    let error: ErrorResponse = response.json().await.expect("Failed to parse JSON");

    assert_eq!(error.error, "USERNAME_NOT_FOUND");
    assert!(error.message.contains(&username));
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_invalid_username_format() {
    let client = reqwest::Client::new();
    let timestamp = chrono::Utc::now().timestamp();

    // Invalid username: uppercase letters
    let invalid_username = "InvalidUsername";
    let peer_id = random_peer_id();
    let (public_key, signature) = create_test_signature(invalid_username, &peer_id, timestamp);

    let request = RegisterRequest {
        username: invalid_username.to_string(),
        peer_id,
        public_key,
        prekey_bundle: create_test_prekey_bundle(),
        signature,
        timestamp,
    };

    let response = client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400); // Bad Request

    let error: ErrorResponse = response.json().await.expect("Failed to parse JSON");

    assert_eq!(error.error, "INVALID_USERNAME");
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_rate_limiting_register() {
    let client = reqwest::Client::new();

    // Try to register 6 usernames rapidly (limit is 5/hour)
    let mut responses = Vec::new();

    for i in 0..6 {
        let username = format!("ratelimit_{}", i);
        let timestamp = chrono::Utc::now().timestamp();
        let peer_id = random_peer_id();
        let (public_key, signature) = create_test_signature(&username, &peer_id, timestamp);

        let request = RegisterRequest {
            username: username.clone(),
            peer_id,
            public_key,
            prekey_bundle: create_test_prekey_bundle(),
            signature,
            timestamp,
        };

        let response = client
            .post(&format!("{}/api/v1/register", BASE_URL))
            .json(&request)
            .send()
            .await
            .expect("Failed to send request");

        responses.push(response.status());
    }

    // First 5 should succeed (200) or fail due to other reasons
    // 6th should fail with 429 (Too Many Requests)
    assert_eq!(responses[5], 429);
}

#[tokio::test]
#[ignore = "requires live identity server + postgres/redis (make up)"]
async fn test_rate_limit_headers() {
    let client = reqwest::Client::new();
    let username = random_username();
    let timestamp = chrono::Utc::now().timestamp();
    let peer_id = random_peer_id();
    let (public_key, signature) = create_test_signature(&username, &peer_id, timestamp);

    let request = RegisterRequest {
        username: username.clone(),
        peer_id,
        public_key,
        prekey_bundle: create_test_prekey_bundle(),
        signature,
        timestamp,
    };

    let response = client
        .post(&format!("{}/api/v1/register", BASE_URL))
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    // Check rate limit headers
    assert!(response.headers().contains_key("X-RateLimit-Limit"));
    assert!(response.headers().contains_key("X-RateLimit-Remaining"));

    let limit = response
        .headers()
        .get("X-RateLimit-Limit")
        .unwrap()
        .to_str()
        .unwrap();

    assert_eq!(limit, "5"); // 5 requests per hour for register
}
