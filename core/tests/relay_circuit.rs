//! P0-G: o cliente precisa reservar um slot no relay e ser alcançável por um
//! circuito `/p2p-circuit`. Este teste sobe um relay real (o mesmo behaviour
//! que o bootstrap server usa) e exige que um peer que NÃO escuta em nenhuma
//! porta seja alcançado por outro através do relay.

use std::time::Duration;

use futures::StreamExt;
use libp2p::{
    identify,
    identity::Keypair,
    multiaddr::Protocol,
    noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use zaplivre_core::network::NetworkManager;

#[derive(NetworkBehaviour)]
struct RelayServer {
    relay: relay::Behaviour,
    identify: identify::Behaviour,
    ping: ping::Behaviour,
}

fn relay_server() -> Swarm<RelayServer> {
    SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("tcp transport")
        .with_behaviour(|key| RelayServer {
            relay: relay::Behaviour::new(key.public().to_peer_id(), relay::Config::default()),
            identify: identify::Behaviour::new(identify::Config::new(
                "/zaplivre-test/1.0.0".to_string(),
                key.public(),
            )),
            ping: ping::Behaviour::default(),
        })
        .expect("relay behaviour")
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60)))
        .build()
}

/// Drive the three nodes until `done` holds or the deadline passes.
async fn drive_until(
    relay: &mut Swarm<RelayServer>,
    nodes: &mut [&mut NetworkManager],
    what: &str,
    mut done: impl FnMut(&[&mut NetworkManager]) -> bool,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !done(nodes) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for: {what}"
        );
        for node in nodes.iter_mut() {
            while node.poll_once().await.expect("poll node") {}
        }
        let _ = tokio::time::timeout(Duration::from_millis(10), relay.select_next_some()).await;
    }
}

#[tokio::test]
async fn unreachable_peer_is_dialed_through_the_relay() {
    let mut relay = relay_server();
    let relay_peer: PeerId = *relay.local_peer_id();
    relay
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .expect("relay listen");
    let relay_addr: Multiaddr = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = relay.select_next_some().await {
            break address;
        }
    };
    // A relay only hands out reservations carrying its external addresses.
    relay.add_external_address(relay_addr.clone());

    // Bob never listens on a socket: the relay is his only way in.
    let mut bob = NetworkManager::with_relay(
        Keypair::generate_ed25519(),
        Some(relay_peer),
        Some(relay_addr.clone()),
    )
    .expect("bob");
    let bob_peer = *bob.local_peer_id();
    bob.dial(relay_peer, relay_addr.clone())
        .expect("bob dials relay");

    drive_until(
        &mut relay,
        &mut [&mut bob],
        "bob's relay reservation",
        |n| n[0].has_relay(),
    )
    .await;

    let mut alice = NetworkManager::with_relay(
        Keypair::generate_ed25519(),
        Some(relay_peer),
        Some(relay_addr.clone()),
    )
    .expect("alice");
    let circuit = relay_addr
        .with(Protocol::P2p(relay_peer))
        .with(Protocol::P2pCircuit)
        .with(Protocol::P2p(bob_peer));
    alice.dial(bob_peer, circuit).expect("alice dials circuit");

    drive_until(
        &mut relay,
        &mut [&mut alice, &mut bob],
        "alice connected to bob through the circuit",
        |n| n[0].is_connected(&bob_peer),
    )
    .await;
}
