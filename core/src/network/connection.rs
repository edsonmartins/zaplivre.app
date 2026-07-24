//! Connection strategy with automatic fallback
//!
//! Manages connection attempts with automatic fallback from direct → hole punch → relay.

use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::retry::RetryPolicy;

/// Connection manager tracks connection strategies for multiple peers
#[derive(Debug)]
pub struct ConnectionManager {
    /// Per-peer connection strategies
    strategies: HashMap<PeerId, ConnectionStrategy>,
    /// Default retry policy
    retry_policy: RetryPolicy,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            strategies: HashMap::new(),
            retry_policy,
        }
    }

    /// Get or create strategy for a peer
    pub fn get_or_create_strategy(&mut self, peer_id: PeerId) -> &mut ConnectionStrategy {
        self.strategies
            .entry(peer_id)
            .or_insert_with(|| ConnectionStrategy::new(peer_id, self.retry_policy.clone()))
    }

    /// Get strategy for a peer
    pub fn get_strategy(&self, peer_id: &PeerId) -> Option<&ConnectionStrategy> {
        self.strategies.get(peer_id)
    }

    /// Remove strategy for a peer
    pub fn remove_strategy(&mut self, peer_id: &PeerId) -> Option<ConnectionStrategy> {
        self.strategies.remove(peer_id)
    }

    /// Check if we should try relay for a peer
    pub fn should_try_relay(&self, peer_id: &PeerId) -> bool {
        if let Some(strategy) = self.strategies.get(peer_id) {
            strategy.should_try_relay()
        } else {
            false
        }
    }

    /// Record a connection failure
    pub fn record_failure(&mut self, peer_id: PeerId) {
        let strategy = self.get_or_create_strategy(peer_id);
        strategy.record_failure();
    }

    /// Record a connection success
    pub fn record_success(&mut self, peer_id: PeerId, connection_type: ConnectionType) {
        let strategy = self.get_or_create_strategy(peer_id);
        strategy.record_success(connection_type);
    }

    /// Get current connection state for a peer
    pub fn get_state(&self, peer_id: &PeerId) -> ConnectionState {
        self.strategies
            .get(peer_id)
            .map(|s| s.state.clone())
            .unwrap_or(ConnectionState::Disconnected)
    }
}

/// Connection strategy for a single peer
#[derive(Debug, Clone)]
pub struct ConnectionStrategy {
    /// Peer ID
    pub peer_id: PeerId,
    /// Current connection state
    pub state: ConnectionState,
    /// Connection attempts history
    pub attempts: Vec<ConnectionAttempt>,
    /// Retry policy
    pub retry_policy: RetryPolicy,
}

impl ConnectionStrategy {
    /// Create a new connection strategy
    pub fn new(peer_id: PeerId, retry_policy: RetryPolicy) -> Self {
        Self {
            peer_id,
            state: ConnectionState::Disconnected,
            attempts: Vec::new(),
            retry_policy,
        }
    }

    /// Record a connection failure
    pub fn record_failure(&mut self) {
        let attempt = ConnectionAttempt {
            started_at: Instant::now(),
            duration: Duration::from_secs(0), // Will be updated
            success: false,
            connection_type: match &self.state {
                ConnectionState::AttemptingDirect { .. } => ConnectionType::Direct,
                ConnectionState::AttemptingHolePunch { .. } => ConnectionType::HolePunch,
                ConnectionState::AttemptingRelay { .. } => ConnectionType::Relayed,
                _ => ConnectionType::Direct,
            },
        };

        self.attempts.push(attempt);

        // Update state based on failure
        match &self.state {
            ConnectionState::AttemptingDirect { attempt, started } => {
                let elapsed = started.elapsed();
                let new_attempt = attempt + 1;

                if new_attempt >= 3 || elapsed > Duration::from_secs(15) {
                    // Move to hole punching
                    self.state = ConnectionState::AttemptingHolePunch {
                        started: Instant::now(),
                    };
                } else {
                    // Retry direct connection
                    self.state = ConnectionState::AttemptingDirect {
                        attempt: new_attempt,
                        started: *started,
                    };
                }
            }
            ConnectionState::AttemptingHolePunch { started } => {
                if started.elapsed() > Duration::from_secs(10) {
                    // Move to relay
                    self.state = ConnectionState::AttemptingRelay {
                        started: Instant::now(),
                    };
                }
            }
            _ => {
                // Start fresh with direct connection
                // attempt starts at 1 since we're recording the first failure
                self.state = ConnectionState::AttemptingDirect {
                    attempt: 1,
                    started: Instant::now(),
                };
            }
        }
    }

    /// Record a connection success
    pub fn record_success(&mut self, connection_type: ConnectionType) {
        let attempt = ConnectionAttempt {
            started_at: Instant::now(),
            duration: Duration::from_secs(0),
            success: true,
            connection_type: connection_type.clone(),
        };

        self.attempts.push(attempt);
        self.state = ConnectionState::Connected(connection_type);
    }

    /// Check if we should try relay
    pub fn should_try_relay(&self) -> bool {
        match &self.state {
            ConnectionState::AttemptingDirect { attempt, started } => {
                *attempt >= 3 || started.elapsed() > Duration::from_secs(15)
            }
            ConnectionState::AttemptingHolePunch { started } => {
                started.elapsed() > Duration::from_secs(10)
            }
            ConnectionState::AttemptingRelay { .. } => true,
            _ => false,
        }
    }

    /// Get success rate for a connection type
    pub fn success_rate(&self, connection_type: &ConnectionType) -> f64 {
        let total = self
            .attempts
            .iter()
            .filter(|a| &a.connection_type == connection_type)
            .count();

        if total == 0 {
            return 0.0;
        }

        let successful = self
            .attempts
            .iter()
            .filter(|a| &a.connection_type == connection_type && a.success)
            .count();

        successful as f64 / total as f64
    }
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,

    /// Attempting direct connection
    AttemptingDirect { attempt: u32, started: Instant },

    /// Attempting hole punching via relay
    AttemptingHolePunch { started: Instant },

    /// Attempting relayed connection
    AttemptingRelay { started: Instant },

    /// Successfully connected
    Connected(ConnectionType),
}

/// Type of connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    /// Direct P2P connection
    Direct,

    /// Hole punched connection (DCUtR)
    HolePunch,

    /// Relayed connection via bootstrap
    Relayed,
}

/// Connection attempt record
#[derive(Debug, Clone)]
pub struct ConnectionAttempt {
    /// When the attempt started
    pub started_at: Instant,
    /// How long it took
    pub duration: Duration,
    /// Whether it succeeded
    pub success: bool,
    /// Type of connection attempted
    pub connection_type: ConnectionType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::new(RetryPolicy::default());
        let peer_id = PeerId::random();

        assert!(manager.get_strategy(&peer_id).is_none());
        assert!(!manager.should_try_relay(&peer_id));
    }

    #[test]
    fn test_record_failure_triggers_fallback() {
        let mut manager = ConnectionManager::new(RetryPolicy::default());
        let peer_id = PeerId::random();

        // Start with direct connection
        manager.get_or_create_strategy(peer_id);

        // Record 3 failures
        manager.record_failure(peer_id);
        manager.record_failure(peer_id);
        manager.record_failure(peer_id);

        // Should now recommend hole punching
        let state = manager.get_state(&peer_id);
        assert!(matches!(state, ConnectionState::AttemptingHolePunch { .. }));
    }

    #[test]
    fn test_connection_type_equality() {
        assert_eq!(ConnectionType::Direct, ConnectionType::Direct);
        assert_ne!(ConnectionType::Direct, ConnectionType::Relayed);
        assert_ne!(ConnectionType::HolePunch, ConnectionType::Relayed);
    }

    #[test]
    fn test_success_rate() {
        let peer_id = PeerId::random();
        let mut strategy = ConnectionStrategy::new(peer_id, RetryPolicy::default());

        strategy.attempts.push(ConnectionAttempt {
            started_at: Instant::now(),
            duration: Duration::from_secs(1),
            success: true,
            connection_type: ConnectionType::Direct,
        });

        strategy.attempts.push(ConnectionAttempt {
            started_at: Instant::now(),
            duration: Duration::from_secs(1),
            success: false,
            connection_type: ConnectionType::Direct,
        });

        assert_eq!(strategy.success_rate(&ConnectionType::Direct), 0.5);
        assert_eq!(strategy.success_rate(&ConnectionType::Relayed), 0.0);
    }
}
