//! Signals for peers that are not connected right now.
//!
//! A phone with the app closed has no WebSocket open, so a call offer sent to
//! it used to be dropped ("target peer not connected"). The offer is now parked
//! for a short while and delivered as soon as the peer registers — which a push
//! notification, sent for call-starting signals, prompts it to do.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// How long a parked signal stays deliverable. A call rings for about this
/// long; an offer delivered later would only ring for a call already over.
pub const PENDING_TTL: Duration = Duration::from_secs(45);

/// Signals kept per absent peer (an offer plus its ICE candidates fit easily).
pub const MAX_PENDING_PER_PEER: usize = 64;

/// Absent peers tracked at once, so the map cannot grow without bound.
pub const MAX_PENDING_PEERS: usize = 10_000;

#[derive(Default)]
pub struct PendingSignals {
    by_peer: HashMap<String, VecDeque<(Instant, String)>>,
}

impl PendingSignals {
    /// Park `text` for `peer`. Returns false when it was not kept.
    pub fn park(&mut self, peer: &str, text: String, now: Instant) -> bool {
        self.purge_expired(now);
        if !self.by_peer.contains_key(peer) && self.by_peer.len() >= MAX_PENDING_PEERS {
            return false;
        }
        let queue = self.by_peer.entry(peer.to_string()).or_default();
        if queue.len() >= MAX_PENDING_PER_PEER {
            queue.pop_front();
        }
        queue.push_back((now, text));
        true
    }

    /// Signals still deliverable to `peer`, oldest first; they leave the queue.
    pub fn take(&mut self, peer: &str, now: Instant) -> Vec<String> {
        self.by_peer
            .remove(peer)
            .unwrap_or_default()
            .into_iter()
            .filter(|(parked_at, _)| now.duration_since(*parked_at) <= PENDING_TTL)
            .map(|(_, text)| text)
            .collect()
    }

    fn purge_expired(&mut self, now: Instant) {
        self.by_peer.retain(|_, queue| {
            queue.retain(|(parked_at, _)| now.duration_since(*parked_at) <= PENDING_TTL);
            !queue.is_empty()
        });
    }
}

/// The call id when a signaling payload starts a call (and so deserves a
/// wake-up push).
pub fn starts_call(payload: &serde_json::Value) -> Option<&str> {
    match payload.get("type").and_then(|t| t.as_str()) {
        Some("call_offer") | Some("platform_offer") => {
            payload.get("call_id").and_then(|c| c.as_str())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parked_signals_are_delivered_in_order_once() {
        let mut pending = PendingSignals::default();
        let t0 = Instant::now();
        assert!(pending.park("bob", "offer".into(), t0));
        assert!(pending.park("bob", "ice".into(), t0));

        assert_eq!(pending.take("bob", t0), vec!["offer", "ice"]);
        assert!(pending.take("bob", t0).is_empty(), "delivered only once");
    }

    #[test]
    fn expired_signals_are_not_delivered() {
        let mut pending = PendingSignals::default();
        let t0 = Instant::now();
        pending.park("bob", "old offer".into(), t0);
        let later = t0 + PENDING_TTL + Duration::from_secs(1);
        assert!(pending.take("bob", later).is_empty());
    }

    #[test]
    fn queues_are_bounded() {
        let mut pending = PendingSignals::default();
        let t0 = Instant::now();
        for i in 0..MAX_PENDING_PER_PEER + 5 {
            pending.park("bob", format!("s{i}"), t0);
        }
        let taken = pending.take("bob", t0);
        assert_eq!(taken.len(), MAX_PENDING_PER_PEER);
        assert_eq!(taken[0], "s5", "the oldest signals are dropped first");
    }

    #[test]
    fn only_call_offers_start_a_call() {
        let offer = serde_json::json!({"type": "call_offer", "call_id": "c1", "sdp": "x"});
        let platform = serde_json::json!({"type": "platform_offer", "call_id": "c2", "sdp": "x"});
        let ice = serde_json::json!({"type": "ice_candidate", "call_id": "c1"});
        assert_eq!(starts_call(&offer), Some("c1"));
        assert_eq!(starts_call(&platform), Some("c2"));
        assert_eq!(starts_call(&ice), None);
    }
}
