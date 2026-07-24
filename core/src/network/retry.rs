//! Exponential backoff retry logic
//!
//! Implements retry policies with exponential backoff for connection attempts.

use std::time::Duration;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(max_attempts: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts,
            base_delay,
            max_delay,
        }
    }

    /// Calculate the delay for the next retry attempt
    ///
    /// Returns `None` if max attempts exceeded, otherwise returns the delay duration
    /// using exponential backoff: base_delay * 2^attempt
    pub fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, ...
        let delay = self.base_delay * 2u32.pow(attempt);
        Some(delay.min(self.max_delay))
    }

    /// Check if we should retry
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    /// Get total wait time for all attempts
    pub fn total_wait_time(&self) -> Duration {
        (0..self.max_attempts)
            .filter_map(|attempt| self.next_delay(attempt))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.next_delay(0), Some(Duration::from_secs(1)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_secs(2)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_secs(4)));
        assert_eq!(policy.next_delay(3), Some(Duration::from_secs(8)));
        assert_eq!(policy.next_delay(4), Some(Duration::from_secs(16)));
        assert_eq!(policy.next_delay(5), None); // Exceeded max_attempts
    }

    #[test]
    fn test_max_delay_cap() {
        let policy = RetryPolicy {
            max_attempts: 10,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        };

        assert_eq!(policy.next_delay(0), Some(Duration::from_secs(1)));
        assert_eq!(policy.next_delay(3), Some(Duration::from_secs(8)));
        assert_eq!(policy.next_delay(4), Some(Duration::from_secs(10))); // Capped
        assert_eq!(policy.next_delay(5), Some(Duration::from_secs(10))); // Capped
    }

    #[test]
    fn test_should_retry() {
        let policy = RetryPolicy::default();

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(4));
        assert!(!policy.should_retry(5));
        assert!(!policy.should_retry(10));
    }

    #[test]
    fn test_total_wait_time() {
        let policy = RetryPolicy::default();

        // 1 + 2 + 4 + 8 + 16 = 31 seconds
        assert_eq!(policy.total_wait_time(), Duration::from_secs(31));
    }

    #[test]
    fn test_custom_policy() {
        let policy = RetryPolicy::new(3, Duration::from_millis(500), Duration::from_secs(5));

        assert_eq!(policy.next_delay(0), Some(Duration::from_millis(500)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_millis(1000)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_millis(2000)));
        assert_eq!(policy.next_delay(3), None);
    }
}
