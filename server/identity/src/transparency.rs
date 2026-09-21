//! Verification primitives for the append-only identity log.

use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};

use crate::models::TransparencyLogEntry;

pub fn entry_hash(previous_hash: &[u8], peer_id: &str, public_key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"zaplivre-key-transparency-v1\0");
    hasher.update(previous_hash);
    hasher.update(peer_id.as_bytes());
    hasher.update(public_key);
    hasher.finalize().into()
}

/// Verify ordering, predecessor links, and hashes in a downloaded segment.
pub fn verify_segment(
    entries: &[TransparencyLogEntry],
    initial_previous: Option<&[u8]>,
) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut previous = initial_previous
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| vec![0; 32]);
    let first_sequence = entries[0].sequence;
    for (sequence, entry) in (first_sequence..).zip(entries) {
        if entry.sequence != sequence {
            return Err("transparency sequence gap or reordering".to_string());
        }
        let advertised_previous = general_purpose::STANDARD
            .decode(&entry.previous_hash)
            .map_err(|_| "invalid previous hash encoding".to_string())?;
        if advertised_previous != previous {
            return Err(format!(
                "invalid predecessor at sequence {}",
                entry.sequence
            ));
        }
        let public_key = general_purpose::STANDARD
            .decode(&entry.public_key)
            .map_err(|_| "invalid public key encoding".to_string())?;
        let advertised_hash = general_purpose::STANDARD
            .decode(&entry.entry_hash)
            .map_err(|_| "invalid entry hash encoding".to_string())?;
        if advertised_hash != entry_hash(&previous, &entry.peer_id, &public_key) {
            return Err(format!("invalid entry hash at sequence {}", entry.sequence));
        }
        previous = advertised_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(sequence: i64, previous: &[u8], peer: &str, key: &[u8]) -> TransparencyLogEntry {
        let hash = entry_hash(previous, peer, key);
        TransparencyLogEntry {
            sequence,
            peer_id: peer.to_string(),
            public_key: general_purpose::STANDARD.encode(key),
            previous_hash: general_purpose::STANDARD.encode(previous),
            entry_hash: general_purpose::STANDARD.encode(hash),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn verifies_chain_and_rejects_tampering() {
        let first = entry(1, &[0; 32], "peer-a", &[1; 32]);
        let first_hash = general_purpose::STANDARD.decode(&first.entry_hash).unwrap();
        let mut second = entry(2, &first_hash, "peer-b", &[2; 32]);
        assert!(verify_segment(&[first.clone(), second.clone()], None).is_ok());
        second.peer_id = "attacker".to_string();
        assert!(verify_segment(&[first, second], None).is_err());
    }
}
