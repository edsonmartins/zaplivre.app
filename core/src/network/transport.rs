//! Transport Layer
//!
//! Manages libp2p transport with TCP, QUIC, Noise encryption, and Yamux multiplexing.

use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade},
    identity::Keypair,
    noise, quic, relay, tcp, yamux, PeerId, Transport,
};
use std::time::Duration;

use crate::utils::error::{ZapLivreError, Result};

/// Build a libp2p transport with:
/// - TCP + QUIC (dual-stack)
/// - Noise encryption
/// - Yamux multiplexing
pub fn build_transport(
    keypair: &Keypair,
    local_peer_id: PeerId,
) -> Result<(Boxed<(PeerId, StreamMuxerBox)>, relay::client::Behaviour)> {
    let (relay_transport, relay_behaviour) = relay::client::new(local_peer_id);

    let relay_transport = relay_transport
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(keypair).map_err(|e| {
            ZapLivreError::Network(format!("Failed to create Noise config: {}", e))
        })?)
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

    // TCP transport with Noise + Yamux (using tokio runtime)
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1Lazy)
        .authenticate(noise::Config::new(keypair).map_err(|e| {
            ZapLivreError::Network(format!("Failed to create Noise config: {}", e))
        })?)
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

    // QUIC transport (built-in encryption + multiplexing, using tokio runtime)
    let quic_transport = quic::tokio::Transport::new(quic::Config::new(keypair))
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .boxed();

    // Combine transports (try QUIC first, fallback to TCP)
    let base_transport = quic_transport
        .or_transport(tcp_transport)
        .map(|either, _| either.into_inner())
        .boxed();

    let transport = relay_transport
        .or_transport(base_transport)
        .map(|either, _| either.into_inner())
        .boxed();

    // Resolver /dns4//dns6//dnsaddr antes de discar - sem isso TODOS os
    // endereços por domínio (bootstraps de produção) falham com
    // MultiaddrNotSupported (bug encontrado no primeiro run real)
    let transport = libp2p::dns::tokio::Transport::system(transport)
        .map_err(|e| ZapLivreError::Network(format!("Failed to create DNS transport: {}", e)))?
        .boxed();

    Ok((transport, relay_behaviour))
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_build_transport() {
        let keypair = identity::Keypair::generate_ed25519();
    let transport = build_transport(&keypair, PeerId::from(keypair.public()));
    assert!(transport.is_ok());
    }

    #[test]
    fn test_transport_with_different_keypairs() {
        let keypair1 = identity::Keypair::generate_ed25519();
        let keypair2 = identity::Keypair::generate_ed25519();

        let transport1 = build_transport(&keypair1, PeerId::from(keypair1.public()));
        let transport2 = build_transport(&keypair2, PeerId::from(keypair2.public()));

        assert!(transport1.is_ok());
        assert!(transport2.is_ok());
    }
}
