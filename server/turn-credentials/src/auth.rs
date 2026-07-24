//! TURN credential generation (RFC 5389)
//!
//! Implements time-limited TURN credentials using HMAC-SHA1

use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Generate TURN credentials using HMAC-SHA1
///
/// Format (RFC 5389):
/// - username: `<timestamp>:<user_id>`
/// - password: base64(HMAC-SHA1(static_secret, username))
///
/// The timestamp ensures credentials are time-limited.
pub fn generate_turn_credentials(
    user_id: &str,
    ttl_seconds: i64,
    static_secret: &str,
) -> (String, String) {
    // Calculate expiration timestamp
    let timestamp = chrono::Utc::now().timestamp() + ttl_seconds;

    // Format: timestamp:user_id
    let turn_username = format!("{}:{}", timestamp, user_id);

    // HMAC-SHA1(static_secret, turn_username)
    let mut mac = Hmac::<Sha1>::new_from_slice(static_secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(turn_username.as_bytes());
    let password = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    (turn_username, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_generation() {
        let (username, password) = generate_turn_credentials(
            "user123",
            86400, // 24 hours
            "test_secret",
        );

        // Username should contain timestamp and user_id
        assert!(username.contains(':'));
        assert!(username.ends_with("user123"));

        // Password should not be empty and should be base64
        assert!(!password.is_empty());
        assert!(general_purpose::STANDARD.decode(&password).is_ok());
    }

    #[test]
    fn test_different_users_different_passwords() {
        let (_, password1) = generate_turn_credentials("user1", 3600, "secret");

        let (_, password2) = generate_turn_credentials("user2", 3600, "secret");

        // Different users should have different passwords
        assert_ne!(password1, password2);
    }

    #[test]
    fn test_timestamp_in_username() {
        let (username, _) = generate_turn_credentials("testuser", 7200, "secret");

        // Extract timestamp from username
        let parts: Vec<&str> = username.split(':').collect();
        assert_eq!(parts.len(), 2);

        let timestamp: i64 = parts[0].parse().unwrap();
        let now = chrono::Utc::now().timestamp();

        // Timestamp should be in the future (now + ttl)
        assert!(timestamp > now);
        assert!(timestamp <= now + 7200);
    }
}
