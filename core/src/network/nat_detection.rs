//! NAT type detection helper
//!
//! Provides utilities for detecting the type of NAT a peer is behind,
//! which helps determine the best connection strategy.

use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

/// Types of NAT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// Full Cone NAT: External IP:Port same for all destinations
    /// Direct connection likely works
    FullCone,

    /// Restricted NAT: External IP same, port varies by destination
    /// Medium difficulty, hole punching might work
    Restricted,

    /// Port Restricted NAT: Both IP and port vary by destination
    /// Hard, DCUtR needed
    PortRestricted,

    /// Symmetric NAT: Different external IP:Port per destination
    /// Hardest, relay required
    Symmetric,

    /// Unknown NAT type
    Unknown,
}

/// NAT detector
#[derive(Debug, Clone, Default)]
pub struct NatDetector {
    /// Observed external addresses from different peers
    observed_addrs: Vec<Multiaddr>,
}

impl NatDetector {
    /// Create a new NAT detector
    pub fn new() -> Self {
        Self {
            observed_addrs: Vec::new(),
        }
    }

    /// Add an observed external address
    pub fn add_observed_address(&mut self, addr: Multiaddr) {
        if !self.observed_addrs.contains(&addr) {
            self.observed_addrs.push(addr);
        }
    }

    /// Guess NAT type based on observed addresses
    ///
    /// Simple heuristic:
    /// - If all observed addresses match → FullCone
    /// - If same IP but different ports → PortRestricted
    /// - If different IPs → Symmetric
    pub fn guess_nat_type(&self) -> NatType {
        if self.observed_addrs.is_empty() {
            return NatType::Unknown;
        }

        if self.observed_addrs.len() == 1 {
            // Only one observation, assume FullCone
            return NatType::FullCone;
        }

        let mut first_ip: Option<String> = None;
        let mut first_port: Option<u16> = None;
        let mut ip_changed = false;
        let mut port_changed = false;

        for addr in &self.observed_addrs {
            let (ip, port) = match Self::extract_ip_port(addr) {
                Some(value) => value,
                None => continue,
            };

            if first_ip.is_none() {
                first_ip = Some(ip);
                first_port = Some(port);
                continue;
            }

            if let Some(ref first_ip_val) = first_ip {
                if *first_ip_val != ip {
                    ip_changed = true;
                }
            }
            if let Some(first_port_val) = first_port {
                if first_port_val != port {
                    port_changed = true;
                }
            }
        }

        // Mapeamento externo variando (porta ou IP) entre observações é a
        // assinatura de NAT simétrico: cada destino recebe um mapeamento
        // diferente. Cones (restricted/port-restricted) mantêm o mesmo
        // mapeamento externo e não são distinguíveis só por endereços
        // observados - detecção real exige STUN/AutoNAT (NAT-01).
        if ip_changed || port_changed {
            NatType::Symmetric
        } else {
            NatType::FullCone
        }
    }

    /// Determine if relay should be used based on NAT type
    pub fn should_use_relay(&self) -> bool {
        matches!(
            self.guess_nat_type(),
            NatType::Symmetric | NatType::PortRestricted
        )
    }

    /// Get all observed addresses
    pub fn observed_addresses(&self) -> &[Multiaddr] {
        &self.observed_addrs
    }

    /// Clear observed addresses
    pub fn clear(&mut self) {
        self.observed_addrs.clear();
    }

    /// Get recommendation for connection strategy
    pub fn connection_recommendation(&self) -> ConnectionStrategy {
        match self.guess_nat_type() {
            NatType::FullCone => ConnectionStrategy::DirectFirst,
            NatType::Restricted => ConnectionStrategy::DirectFirst,
            NatType::PortRestricted => ConnectionStrategy::HolePunchFirst,
            NatType::Symmetric => ConnectionStrategy::RelayFirst,
            NatType::Unknown => ConnectionStrategy::DirectFirst, // Optimistic default
        }
    }

    fn extract_ip_port(addr: &Multiaddr) -> Option<(String, u16)> {
        let mut ip: Option<String> = None;
        let mut port: Option<u16> = None;

        for protocol in addr.iter() {
            match protocol {
                Protocol::Ip4(v4) => ip = Some(v4.to_string()),
                Protocol::Ip6(v6) => ip = Some(v6.to_string()),
                Protocol::Tcp(p) => port = Some(p),
                Protocol::Udp(p) => port = Some(p),
                _ => {}
            }
        }

        match (ip, port) {
            (Some(ip_val), Some(port_val)) => Some((ip_val, port_val)),
            _ => None,
        }
    }
}

/// Recommended connection strategy based on NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStrategy {
    /// Try direct connection first (best for FullCone/Restricted NAT)
    DirectFirst,
    /// Try hole punching first (best for PortRestricted NAT)
    HolePunchFirst,
    /// Use relay immediately (best for Symmetric NAT)
    RelayFirst,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_detector() {
        let detector = NatDetector::new();
        assert_eq!(detector.guess_nat_type(), NatType::Unknown);
        assert!(!detector.should_use_relay());
    }

    #[test]
    fn test_single_observation() {
        let mut detector = NatDetector::new();
        let addr: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();
        detector.add_observed_address(addr);

        assert_eq!(detector.guess_nat_type(), NatType::FullCone);
        assert!(!detector.should_use_relay());
    }

    #[test]
    fn test_full_cone_detection() {
        let mut detector = NatDetector::new();
        let addr: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();

        // Add same address multiple times
        detector.add_observed_address(addr.clone());
        detector.add_observed_address(addr.clone());
        detector.add_observed_address(addr);

        assert_eq!(detector.guess_nat_type(), NatType::FullCone);
        assert!(!detector.should_use_relay());
    }

    #[test]
    fn test_symmetric_detection() {
        let mut detector = NatDetector::new();

        let addr1: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();
        let addr2: Multiaddr = "/ip4/1.2.3.4/tcp/5678".parse().unwrap();

        detector.add_observed_address(addr1);
        detector.add_observed_address(addr2);

        assert_eq!(detector.guess_nat_type(), NatType::Symmetric);
        assert!(detector.should_use_relay());
    }

    #[test]
    fn test_connection_strategy() {
        let mut detector = NatDetector::new();
        assert_eq!(
            detector.connection_recommendation(),
            ConnectionStrategy::DirectFirst
        );

        let addr1: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();
        let addr2: Multiaddr = "/ip4/1.2.3.4/tcp/5678".parse().unwrap();

        detector.add_observed_address(addr1);
        detector.add_observed_address(addr2);

        assert_eq!(
            detector.connection_recommendation(),
            ConnectionStrategy::RelayFirst
        );
    }

    #[test]
    fn test_clear() {
        let mut detector = NatDetector::new();
        let addr: Multiaddr = "/ip4/1.2.3.4/tcp/1234".parse().unwrap();

        detector.add_observed_address(addr);
        assert_eq!(detector.observed_addresses().len(), 1);

        detector.clear();
        assert_eq!(detector.observed_addresses().len(), 0);
        assert_eq!(detector.guess_nat_type(), NatType::Unknown);
    }
}
