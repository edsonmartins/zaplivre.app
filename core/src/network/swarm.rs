//! Swarm Management
//!
//! Manages the libp2p Swarm for P2P networking.

use libp2p::{
    identity::Keypair,
    kad::{self, Quorum, QueryId, Record, RecordKey},
    swarm::{Config as SwarmConfig, Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use futures::stream::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::select;
use tokio::sync::oneshot;
#[cfg(any(feature = "voip", feature = "video"))]
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

use super::{
    behaviour::MePassaBehaviour,
    connection::{ConnectionManager, ConnectionType},
    message_handler::MessageHandler,
    relay::RelayManager,
    nat_detection::NatDetector,
    retry::RetryPolicy,
    transport::build_transport,
};
use crate::{
    protocol::{pb::message::Payload, Message, MessageType},
    utils::error::{MePassaError, Result},
};
use crate::group::GroupManager;

/// P2P Network Manager
pub struct NetworkManager {
    swarm: Swarm<MePassaBehaviour>,
    local_peer_id: PeerId,
    connection_manager: ConnectionManager,
    relay_manager: RelayManager,
    message_handler: Option<std::sync::Arc<MessageHandler>>,
    pending_kad_get: HashMap<QueryId, oneshot::Sender<Option<Multiaddr>>>,
    last_published_addr: Option<Multiaddr>,
    nat_detector: NatDetector,
    prefer_relay: bool,
    group_manager: Option<Arc<GroupManager>>,
    #[cfg(any(feature = "voip", feature = "video"))]
    voip_signaling_sender:
        Option<mpsc::UnboundedSender<(PeerId, crate::voip::signaling::SignalingMessage)>>,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new(keypair: Keypair) -> Result<Self> {
        Self::with_relay(keypair, None, None)
    }

    /// Create a new network manager with optional relay configuration
    pub fn with_relay(
        keypair: Keypair,
        bootstrap_relay_peer: Option<PeerId>,
        relay_addr: Option<Multiaddr>,
    ) -> Result<Self> {
        let local_peer_id = PeerId::from(keypair.public());

        // Build transport + relay behaviour
        let (transport, relay_behaviour) = build_transport(&keypair, local_peer_id)?;

        // Create behaviour
        let behaviour = MePassaBehaviour::new(local_peer_id, &keypair, relay_behaviour)?;

        // Create swarm
        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            SwarmConfig::with_tokio_executor()
                .with_idle_connection_timeout(Duration::from_secs(60)),
        );

        // Create connection manager with default retry policy
        let connection_manager = ConnectionManager::new(RetryPolicy::default());

        // Create relay manager
        let relay_manager = RelayManager::new(bootstrap_relay_peer, relay_addr);

        Ok(Self {
            swarm,
            local_peer_id,
            connection_manager,
            relay_manager,
            message_handler: None,
            pending_kad_get: HashMap::new(),
            last_published_addr: None,
            nat_detector: NatDetector::new(),
            prefer_relay: false,
            group_manager: None,
            #[cfg(any(feature = "voip", feature = "video"))]
            voip_signaling_sender: None,
        })
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Set message handler for processing incoming messages
    pub fn set_message_handler(&mut self, handler: std::sync::Arc<MessageHandler>) {
        self.message_handler = Some(handler);
    }

    /// Set group manager for handling GossipSub group messages
    pub fn set_group_manager(&mut self, manager: Arc<GroupManager>) {
        self.group_manager = Some(manager);
    }

    /// Set VoIP signaling sender for forwarding inbound signals
    #[cfg(any(feature = "voip", feature = "video"))]
    pub fn set_voip_signaling_sender(
        &mut self,
        sender: mpsc::UnboundedSender<(PeerId, crate::voip::signaling::SignalingMessage)>,
    ) {
        self.voip_signaling_sender = Some(sender);
    }

    /// Start listening on a multiaddr
    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm
            .listen_on(addr)
            .map_err(|e| MePassaError::Network(format!("Failed to listen: {}", e)))?;

        Ok(())
    }

    /// Get all current listening addresses
    pub fn listening_addresses(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Check if a peer is connected
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.swarm.is_connected(peer_id)
    }

    /// Subscribe to a GossipSub topic
    pub fn subscribe_gossipsub(
        &mut self,
        topic: &libp2p::gossipsub::IdentTopic,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(topic)
            .map_err(|e| MePassaError::Network(format!("GossipSub subscribe failed: {}", e)))?;
        Ok(())
    }

    /// Unsubscribe from a GossipSub topic
    pub fn unsubscribe_gossipsub(
        &mut self,
        topic: &libp2p::gossipsub::IdentTopic,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(topic)
            .map_err(|e| MePassaError::Network(format!("GossipSub unsubscribe failed: {}", e)))?;
        Ok(())
    }

    /// Publish a GossipSub message
    pub fn publish_gossipsub(
        &mut self,
        topic: &libp2p::gossipsub::IdentTopic,
        payload: Vec<u8>,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), payload)
            .map_err(|e| MePassaError::Network(format!("GossipSub publish failed: {}", e)))?;
        Ok(())
    }

    /// Dial a peer with automatic relay fallback
    pub fn dial(&mut self, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        if self.prefer_relay {
            tracing::info!("🔁 NAT suggests relay-first, attempting relay to {}", peer_id);
            if let Ok(()) = self.dial_via_relay(peer_id) {
                return Ok(());
            }
        }

        // Check if we should try relay based on connection history
        if self.connection_manager.should_try_relay(&peer_id) {
            tracing::info!("🔄 Attempting relay connection to {}", peer_id);
            return self.dial_via_relay(peer_id);
        }

        // Try direct connection first
        tracing::debug!("📞 Attempting direct connection to {} at {}", peer_id, addr);
        match self.swarm.dial(addr.clone()) {
            Ok(_) => {
                // Connection initiated, will track result in events
                Ok(())
            }
            Err(e) => {
                tracing::warn!("⚠️ Direct dial failed to {}: {}", peer_id, e);
                self.connection_manager.record_failure(peer_id);

                // Try relay if available and we should fallback
                if self.connection_manager.should_try_relay(&peer_id) {
                    tracing::info!("🔄 Falling back to relay for {}", peer_id);
                    self.dial_via_relay(peer_id)
                } else {
                    Err(MePassaError::Network(format!("Failed to dial {}: {}", peer_id, e)))
                }
            }
        }
    }

    /// Dial a peer via relay
    fn dial_via_relay(&mut self, peer_id: PeerId) -> Result<()> {
        if let Some(circuit_addr) = self.relay_manager.circuit_addr(&peer_id) {
            tracing::info!("🌉 Dialing {} via relay circuit", peer_id);
            self.swarm
                .dial(circuit_addr)
                .map_err(|e| MePassaError::Network(format!("Failed to dial via relay: {}", e)))?;
            Ok(())
        } else {
            Err(MePassaError::Network(
                "No relay configuration available".to_string(),
            ))
        }
    }

    /// Add a peer to the DHT
    pub fn add_peer_to_dht(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    /// Publish our reachable address in the DHT
    pub fn publish_own_address(&mut self, addr: Multiaddr) {
        if self.last_published_addr.as_ref() == Some(&addr) {
            return;
        }

        let key = Self::addr_record_key(&self.local_peer_id);
        let record = Record {
            key,
            value: addr.to_string().into_bytes(),
            publisher: Some(self.local_peer_id),
            expires: None,
        };

        match self
            .swarm
            .behaviour_mut()
            .kademlia
            .put_record(record, Quorum::One)
        {
            Ok(query_id) => {
                tracing::info!("📌 Published address in DHT (query_id: {:?})", query_id);
                self.last_published_addr = Some(addr);
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to publish address in DHT: {:?}", e);
            }
        }
    }

    /// Resolve a peer address via DHT
    pub fn resolve_peer_address(&mut self, peer_id: PeerId) -> oneshot::Receiver<Option<Multiaddr>> {
        tracing::info!("🔍 DHT lookup requested for peer {}", peer_id);
        let key = Self::addr_record_key(&peer_id);
        let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);
        let (tx, rx) = oneshot::channel();
        self.pending_kad_get.insert(query_id, tx);
        rx
    }

    fn addr_record_key(peer_id: &PeerId) -> RecordKey {
        let key = format!("mepassa:addr:{}", peer_id);
        RecordKey::new(&key)
    }

    fn is_routable_addr(addr: &Multiaddr) -> bool {
        let addr_str = addr.to_string();
        !(addr_str.contains("/127.0.0.1/")
            || addr_str.contains("/::1/")
            || addr_str.contains("/ip4/0.0.0.0/")
            || addr_str.contains("/ip6/::/"))
    }

    /// Bootstrap the DHT
    pub fn bootstrap(&mut self) -> Result<()> {
        tracing::info!("🌐 Starting DHT bootstrap...");
        self.swarm
            .behaviour_mut()
            .kademlia
            .bootstrap()
            .map_err(|e| MePassaError::Network(format!("Failed to bootstrap DHT: {}", e)))?;

        Ok(())
    }

    /// Get connected peers count
    pub fn connected_peers(&self) -> usize {
        self.swarm.connected_peers().count()
    }

    /// Get connection state for a peer
    pub fn connection_state(
        &self,
        peer_id: &PeerId,
    ) -> super::connection::ConnectionState {
        self.connection_manager.get_state(peer_id)
    }

    /// Check if relay is available
    pub fn has_relay(&self) -> bool {
        self.relay_manager.has_reservation()
    }

    /// Attempt to reserve relay slot
    pub fn reserve_relay_slot(&mut self) -> Result<()> {
        if let Some(relay_peer) = self.relay_manager.bootstrap_relay_peer {
            if let Some(relay_addr) = &self.relay_manager.relay_addr {
                tracing::info!("🔗 Requesting relay reservation from {}", relay_peer);

                // Connect to relay first if not connected
                self.add_peer_to_dht(relay_peer, relay_addr.clone());

                let listen_addr = relay_addr
                    .clone()
                    .with(libp2p::multiaddr::Protocol::P2p(relay_peer))
                    .with(libp2p::multiaddr::Protocol::P2pCircuit);

                self.swarm
                    .listen_on(listen_addr)
                    .map_err(|e| MePassaError::Network(format!("Failed to listen via relay: {}", e)))?;

                // Mark reservation as pending until relay client confirms it.
                self.relay_manager.mark_reservation_pending();

                Ok(())
            } else {
                Err(MePassaError::Network("No relay address configured".to_string()))
            }
        } else {
            Err(MePassaError::Network("No relay peer configured".to_string()))
        }
    }

    /// Send a message to a peer
    pub fn send_message(&mut self, peer_id: PeerId, message: crate::protocol::Message) -> Result<()> {
        let request_id = self
            .swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, message);

        tracing::info!("Sent message to {} (request_id: {:?})", peer_id, request_id);
        Ok(())
    }

    /// Send an ACK response to a peer
    pub fn send_ack(
        &mut self,
        channel: libp2p::request_response::ResponseChannel<crate::protocol::Message>,
        ack_message: crate::protocol::Message,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .request_response
            .send_response(channel, ack_message)
            .map_err(|e| MePassaError::Network(format!("Failed to send ACK: {:?}", e)))?;

        Ok(())
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Send a VoIP signaling message to a peer
    pub fn send_voip_signal(
        &mut self,
        peer_id: PeerId,
        signal: crate::voip::signaling::SignalingMessage,
    ) -> Result<()> {
        let request_id = self
            .swarm
            .behaviour_mut()
            .voip_signaling
            .send_request(&peer_id, signal.clone());

        tracing::info!("📞 Sent VoIP signal to {} (request_id: {:?}): {:?}", peer_id, request_id, signal);
        Ok(())
    }

    #[cfg(feature = "voip")]
    /// Send a VoIP signaling response to a peer
    pub fn send_voip_response(
        &mut self,
        channel: libp2p::request_response::ResponseChannel<crate::voip::signaling::SignalingMessage>,
        response: crate::voip::signaling::SignalingMessage,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .voip_signaling
            .send_response(channel, response)
            .map_err(|e| MePassaError::Network(format!("Failed to send VoIP response: {:?}", e)))?;

        Ok(())
    }

    /// Run the event loop (blocking)
    pub async fn run(&mut self) -> Result<()> {
        loop {
            select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await?;
                }
            }
        }
    }

    /// Poll for one event and process it (non-blocking)
    /// Returns true if an event was processed, false if no events pending
    pub async fn poll_once(&mut self) -> Result<bool> {
        use futures::future::poll_fn;
        use std::task::Poll;

        let event = poll_fn(|cx| {
            match self.swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(event)) => Poll::Ready(Some(event)),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Ready(None), // Return immediately if no events
            }
        }).await;

        if let Some(event) = event {
            self.handle_event(event).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Handle swarm events
    async fn handle_event(&mut self, event: SwarmEvent<MePassaBehaviourEvent>) -> Result<()> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("Listening on {}", address);
                if Self::is_routable_addr(&address) {
                    self.publish_own_address(address);
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                let addr = endpoint.get_remote_address();
                tracing::info!("✅ Connected to {} at {}", peer_id, addr);

                // Determine connection type and record success
                let connection_type = if addr.to_string().contains("p2p-circuit") {
                    ConnectionType::Relayed
                } else {
                    // TODO: Detect if connection was upgraded via DCUtR
                    ConnectionType::Direct
                };

                self.connection_manager
                    .record_success(peer_id, connection_type);
            }
            SwarmEvent::ConnectionClosed {
                peer_id, cause, ..
            } => {
                tracing::info!("Disconnected from {}: {:?}", peer_id, cause);
            }
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }
            SwarmEvent::IncomingConnection { local_addr, send_back_addr, .. } => {
                tracing::info!("🔗 Incoming connection from {} to {}", send_back_addr, local_addr);
            }
            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                tracing::warn!("⚠️ Incoming connection error from {} to {}: {:?}", send_back_addr, local_addr, error);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                tracing::warn!("⚠️ Outgoing connection error to {:?}: {:?}", peer_id, error);
            }
            SwarmEvent::Dialing { peer_id, .. } => {
                tracing::info!("📞 Dialing peer: {:?}", peer_id);
            }
            _ => {
                tracing::trace!("Other swarm event received");
            }
        }

        Ok(())
    }

    /// Handle behaviour-specific events
    async fn handle_behaviour_event(&mut self, event: MePassaBehaviourEvent) -> Result<()> {
        match event {
            MePassaBehaviourEvent::Kademlia(kad_event) => {
                match kad_event {
                    kad::Event::OutboundQueryProgressed { id, result, .. } => {
                        if let Some(tx) = self.pending_kad_get.remove(&id) {
                            let addr_opt = match result {
                                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                                    let value = record.record.value;
                                    match std::str::from_utf8(&value) {
                                        Ok(addr_str) => addr_str.parse::<Multiaddr>().ok(),
                                        Err(_) => None,
                                    }
                                }
                                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. })) => None,
                                kad::QueryResult::GetRecord(Err(_)) => None,
                                _ => None,
                            };
                            let _ = tx.send(addr_opt);
                        } else {
                            tracing::debug!("Kademlia event: {:?}", result);
                        }
                    }
                    _ => {
                        tracing::debug!("Kademlia event: {:?}", kad_event);
                    }
                }
            }
            MePassaBehaviourEvent::Mdns(mdns_event) => {
                match mdns_event {
                    libp2p::mdns::Event::Discovered(peers) => {
                        for (peer_id, addr) in peers {
                            tracing::info!("mDNS discovered peer: {} at {}", peer_id, addr);
                            self.add_peer_to_dht(peer_id, addr);
                        }
                    }
                    libp2p::mdns::Event::Expired(peers) => {
                        for (peer_id, _) in peers {
                            tracing::info!("mDNS peer expired: {}", peer_id);
                        }
                    }
                }
            }
            MePassaBehaviourEvent::Identify(identify_event) => {
                match identify_event {
                    libp2p::identify::Event::Received { info, .. } => {
                        let observed_addr = info.observed_addr;
                        tracing::info!("🧭 Observed external address: {}", observed_addr);
                        if Self::is_routable_addr(&observed_addr) {
                            self.swarm.add_external_address(observed_addr.clone());
                            self.nat_detector.add_observed_address(observed_addr.clone());
                            self.publish_own_address(observed_addr);

                            if self.nat_detector.should_use_relay() && !self.prefer_relay {
                                tracing::info!("🌐 NAT suggests relay-first strategy");
                                self.prefer_relay = true;
                                let _ = self.reserve_relay_slot();
                            }
                        }
                    }
                    _ => {
                        tracing::debug!("Identify event: {:?}", identify_event);
                    }
                }
            }
            MePassaBehaviourEvent::Ping(ping_event) => {
                tracing::trace!("Ping event: {:?}", ping_event);
            }
            MePassaBehaviourEvent::Gossipsub(gossipsub_event) => {
                match gossipsub_event {
                    libp2p::gossipsub::Event::Message { message, .. } => {
                        if let Some(group_manager) = &self.group_manager {
                            if let Err(e) = group_manager
                                .handle_gossipsub_message(&message.topic.clone(), message)
                                .await
                            {
                                tracing::warn!("Failed to handle group message: {}", e);
                            }
                        }
                    }
                    _ => {
                        tracing::debug!("GossipSub event: {:?}", gossipsub_event);
                    }
                }
            }
            MePassaBehaviourEvent::RequestResponse(rr_event) => {
                match rr_event {
                    libp2p::request_response::Event::Message { peer, message } => {
                        match message {
                            libp2p::request_response::Message::Request {
                                request_id,
                                request,
                                channel,
                            } => {
                                tracing::info!(
                                    "📨 Received request from {}: {} (request_id: {:?})",
                                    peer,
                                    request.id,
                                    request_id
                                );

                                // Process message through handler
                                if let Some(handler) = self.message_handler.clone() {
                                    let message_type = MessageType::try_from(request.r#type)
                                        .unwrap_or(MessageType::Unspecified);

                                    // Special case: MediaRequest triggers chunked responses
                                    let mut pending_chunks = Vec::new();
                                    if message_type == MessageType::MediaRequest {
                                        if let Some(Payload::MediaRequest(ref media_request)) = request.payload {
                                            match handler.build_media_chunks(peer, media_request).await {
                                                Ok(chunks) => {
                                                    pending_chunks = chunks;
                                                }
                                                Err(e) => {
                                                    tracing::error!("❌ Failed to build media chunks: {}", e);
                                                }
                                            }
                                        }
                                    }

                                    match handler.handle_incoming_message(peer, request).await {
                                        Ok(ack) => {
                                            tracing::info!("✅ Processed message {}, sending ACK", ack.message_id);
                                            let response = Message {
                                                id: Uuid::new_v4().to_string(),
                                                sender_peer_id: self.local_peer_id.to_string(),
                                                recipient_peer_id: peer.to_string(),
                                                timestamp: Utc::now().timestamp_millis(),
                                                r#type: MessageType::Ack as i32,
                                                payload: Some(Payload::Ack(ack)),
                                            };
                                            if let Err(e) = self.send_ack(channel, response) {
                                                tracing::error!("❌ Failed to send ACK: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("❌ Failed to process message: {}", e);
                                        }
                                    }

                                    if !pending_chunks.is_empty() {
                                        for chunk in pending_chunks {
                                            let _ = self.send_message(peer, chunk);
                                        }
                                    }
                                } else {
                                    tracing::warn!("⚠️ No message handler configured, message will be dropped");
                                }
                            }
                            libp2p::request_response::Message::Response {
                                request_id,
                                response,
                            } => {
                                tracing::info!(
                                    "✅ Received ACK response from {}: {} (request_id: {:?})",
                                    peer,
                                    response.id,
                                    request_id
                                );

                                // Process ACK through handler
                                if let Some(ref handler) = self.message_handler {
                                    // Extract ACK from response payload
                                    if let Some(crate::protocol::pb::message::Payload::Ack(ack)) = response.payload {
                                        let handler = Arc::clone(handler);
                                        tokio::spawn(async move {
                                            if let Err(e) = handler.handle_outgoing_ack(ack).await {
                                                tracing::error!("❌ Failed to process ACK: {}", e);
                                            }
                                        });
                                    } else {
                                        tracing::warn!("⚠️ Response is not an ACK message");
                                    }
                                } else {
                                    tracing::debug!("No message handler configured for ACK processing");
                                }
                            }
                        }
                    }
                    libp2p::request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        tracing::warn!(
                            "Outbound request failed to {}: {:?} (request_id: {:?})",
                            peer,
                            error,
                            request_id
                        );
                    }
                    libp2p::request_response::Event::InboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        tracing::warn!(
                            "Inbound request failed from {}: {:?} (request_id: {:?})",
                            peer,
                            error,
                            request_id
                        );
                    }
                    libp2p::request_response::Event::ResponseSent { peer, request_id } => {
                        tracing::debug!("Response sent to {} (request_id: {:?})", peer, request_id);
                    }
                }
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            MePassaBehaviourEvent::VoipSignaling(voip_event) => {
                match voip_event {
                    libp2p::request_response::Event::Message { peer, message } => {
                        match message {
                            libp2p::request_response::Message::Request {
                                request_id,
                                request,
                                channel,
                            } => {
                                tracing::info!(
                                    "📞 Received VoIP signal from {}: {:?} (request_id: {:?})",
                                    peer,
                                    request,
                                    request_id
                                );
                                if let Some(sender) = &self.voip_signaling_sender {
                                    if let Err(err) = sender.send((peer, request.clone())) {
                                        tracing::warn!(
                                            "📞 Failed to forward VoIP signal to integration: {}",
                                            err
                                        );
                                    }
                                }

                                // Send ACK to complete request/response cycle
                                let ack = crate::voip::signaling::SignalingMessage::CallAccept {
                                    call_id: request.call_id().to_string(),
                                };
                                let _ = self.send_voip_response(channel, ack);
                            }
                            libp2p::request_response::Message::Response {
                                request_id,
                                response,
                            } => {
                                tracing::info!(
                                    "📞 Received VoIP response from {}: {:?} (request_id: {:?})",
                                    peer,
                                    response,
                                    request_id
                                );
                                if let Some(sender) = &self.voip_signaling_sender {
                                    if let Err(err) = sender.send((peer, response)) {
                                        tracing::warn!(
                                            "📞 Failed to forward VoIP response to integration: {}",
                                            err
                                        );
                                    }
                                }
                            }
                        }
                    }
                    libp2p::request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        tracing::warn!(
                            "📞 VoIP signal outbound failed to {}: {:?} (request_id: {:?})",
                            peer,
                            error,
                            request_id
                        );
                        tracing::warn!("📞 VoIP request to {} failed: {:?}", peer, error);
                    }
                    libp2p::request_response::Event::InboundFailure {
                        peer,
                        request_id,
                        error,
                    } => {
                        tracing::warn!(
                            "📞 VoIP signal inbound failed from {}: {:?} (request_id: {:?})",
                            peer,
                            error,
                            request_id
                        );
                    }
                    libp2p::request_response::Event::ResponseSent { peer, request_id } => {
                        tracing::debug!("📞 VoIP response sent to {} (request_id: {:?})", peer, request_id);
                    }
                }
            }
            MePassaBehaviourEvent::Dcutr(dcutr_event) => {
                match dcutr_event.result {
                    Ok(_) => {
                        self.connection_manager
                            .record_success(dcutr_event.remote_peer_id, ConnectionType::HolePunch);
                        tracing::info!(
                            "🎯 DCUtR upgrade succeeded for {}",
                            dcutr_event.remote_peer_id
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            "🎯 DCUtR upgrade failed for {}: {}",
                            dcutr_event.remote_peer_id,
                            err
                        );
                    }
                }
            }
            MePassaBehaviourEvent::Relay(relay_event) => {
                match relay_event {
                    libp2p::relay::client::Event::ReservationReqAccepted {
                        relay_peer_id, ..
                    } => {
                        tracing::info!("🔗 Relay reservation accepted by {}", relay_peer_id);
                        self.relay_manager.mark_reservation_reserved(3600);
                    }
                    libp2p::relay::client::Event::OutboundCircuitEstablished { relay_peer_id, .. } => {
                        tracing::info!("🌉 Relay circuit established via {}", relay_peer_id);
                    }
                    libp2p::relay::client::Event::InboundCircuitEstablished { src_peer_id, .. } => {
                        tracing::info!("🌉 Inbound relayed connection from {}", src_peer_id);
                    }
                }
            }
        }

        Ok(())
    }
}

// The MePassaBehaviourEvent type is auto-generated by NetworkBehaviour derive macro
use super::behaviour::MePassaBehaviourEvent;

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[tokio::test]
    async fn test_create_network_manager() {
        let keypair = identity::Keypair::generate_ed25519();
        let manager = NetworkManager::new(keypair);

        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_local_peer_id() {
        let keypair = identity::Keypair::generate_ed25519();
        let expected_peer_id = PeerId::from(keypair.public());

        let manager = NetworkManager::new(keypair).unwrap();

        assert_eq!(*manager.local_peer_id(), expected_peer_id);
    }

    #[tokio::test]
    async fn test_listen_on() {
        let keypair = identity::Keypair::generate_ed25519();
        let mut manager = NetworkManager::new(keypair).unwrap();

        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        let result = manager.listen_on(addr);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connected_peers_initially_zero() {
        let keypair = identity::Keypair::generate_ed25519();
        let manager = NetworkManager::new(keypair).unwrap();

        assert_eq!(manager.connected_peers(), 0);
    }
}
