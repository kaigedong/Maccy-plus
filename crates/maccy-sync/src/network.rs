use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId};
use libp2p::identity::Keypair;
use libp2p::mdns;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{PeerId, SwarmBuilder};
use tokio::sync::mpsc;

use crate::error::ErrorCode;
use crate::state::{SharedState, SyncCommand};
use crate::types::*;

const PAIRING_TOPIC: &str = "maccy-sync-pairing-v1";

#[derive(NetworkBehaviour)]
pub struct MaccyBehaviour {
    pub mdns: mdns::tokio::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub identify: libp2p::identify::Behaviour,
}

pub struct NetworkManager {
    swarm: libp2p::Swarm<MaccyBehaviour>,
    command_rx: mpsc::UnboundedReceiver<SyncCommand>,
    state: SharedState,
    discovered_peers: HashMap<PeerId, PeerInfo>,
    paired_peers: HashMap<PeerId, Vec<u8>>,
    listen_port: u16,
}

impl NetworkManager {
    pub fn new(
        command_rx: mpsc::UnboundedReceiver<SyncCommand>,
        state: SharedState,
        local_key: Keypair,
    ) -> Result<Self, ErrorCode> {
        Self::build(command_rx, state, local_key, LISTEN_PORT)
    }

    /// Create with a custom listen port (for testing).
    pub fn new_with_port(
        command_rx: mpsc::UnboundedReceiver<SyncCommand>,
        state: SharedState,
        local_key: Keypair,
        port: u16,
    ) -> Result<Self, ErrorCode> {
        Self::build(command_rx, state, local_key, port)
    }

    fn build(
        command_rx: mpsc::UnboundedReceiver<SyncCommand>,
        state: SharedState,
        local_key: Keypair,
        listen_port: u16,
    ) -> Result<Self, ErrorCode> {
        let local_peer_id = PeerId::from(local_key.public());

        let mdns_config = mdns::Config {
            query_interval: Duration::from_secs(5),
            ttl: Duration::from_secs(120),
            ..mdns::Config::default()
        };
        let mdns_behaviour = mdns::tokio::Behaviour::new(mdns_config, local_peer_id)
            .map_err(|_| ErrorCode::Init)?;

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(|msg: &gossipsub::Message| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&msg.data, &mut hasher);
                MessageId::from(std::hash::Hasher::finish(&hasher).to_string())
            })
            .build()
            .map_err(|_| ErrorCode::Init)?;

        let gossipsub_behaviour = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|_| ErrorCode::Init)?;

        let identify = libp2p::identify::Behaviour::new(
            libp2p::identify::Config::new(
                PAIRING_PROTOCOL.to_string(),
                local_key.public(),
            )
            .with_agent_version(format!(
                "maccy-sync/0.1.0/{}",
                state.lock().unwrap().device_name,
            )),
        );

        let behaviour = MaccyBehaviour {
            mdns: mdns_behaviour,
            gossipsub: gossipsub_behaviour,
            identify,
        };

        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|_| ErrorCode::Init)?
            .with_behaviour(|_| behaviour)
            .map_err(|_| ErrorCode::Init)?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(60))
            })
            .build();

        Ok(Self {
            swarm,
            command_rx,
            state,
            discovered_peers: HashMap::new(),
            paired_peers: HashMap::new(),
            listen_port,
        })
    }

    pub async fn run(&mut self) {
        let sync_topic = IdentTopic::new(TOPIC_NAME);
        let _ = self.swarm.behaviour_mut().gossipsub.subscribe(&sync_topic);

        let pairing_topic = IdentTopic::new(PAIRING_TOPIC);
        let _ = self.swarm.behaviour_mut().gossipsub.subscribe(&pairing_topic);

        let listen_addr: libp2p::Multiaddr =
            format!("/ip4/0.0.0.0/tcp/{}", self.listen_port).parse().unwrap();
        if self.swarm.listen_on(listen_addr).is_err() {
            self.emit_error(ErrorCode::Network, "Failed to listen on port".into());
            return;
        }

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
                Some(command) = self.command_rx.recv() => {
                    if matches!(command, SyncCommand::Shutdown) {
                        break;
                    }
                    self.handle_command(command).await;
                }
            }
        }
    }

    fn state_emit(&self, event: SyncEvent) {
        let state = self.state.lock().unwrap();
        state.emit(event);
    }

    fn emit_error(&self, code: ErrorCode, msg: String) {
        let state = self.state.lock().unwrap();
        state.emit_error(code, msg);
    }

    // ── Swarm events ──────────────────────────────────────────────

    async fn handle_swarm_event(&mut self, event: SwarmEvent<MaccyBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(MaccyBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, addr) in peers {
                    let info = PeerInfo {
                        peer_id: peer_id.to_string(),
                        display_name: String::new(),
                        addresses: vec![addr.to_string()],
                        is_connected: false,
                    };
                    self.discovered_peers.insert(peer_id, info.clone());
                    self.state_emit(SyncEvent::PeerDiscovered { peer: info });
                    let _ = self.swarm.dial(peer_id);
                }
            }
            SwarmEvent::Behaviour(MaccyBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _) in peers {
                    if let Some(info) = self.discovered_peers.remove(&peer_id) {
                        self.state_emit(SyncEvent::PeerLost { peer_id: info.peer_id });
                    }
                }
            }
            SwarmEvent::Behaviour(MaccyBehaviourEvent::Identify(
                libp2p::identify::Event::Received { peer_id, info, .. },
            )) => {
                let device_name = info
                    .agent_version
                    .split('/')
                    .last()
                    .unwrap_or("Unknown")
                    .to_string();
                log::info!("Identified {} as {}", peer_id, device_name);
                let observed_addr = info.observed_addr.to_string();
                let listen_addrs: Vec<String> = info.listen_addrs.iter().map(|a| a.to_string()).collect();

                if let Some(peer_info) = self.discovered_peers.get_mut(&peer_id) {
                    peer_info.display_name = device_name;
                    if !listen_addrs.is_empty() {
                        peer_info.addresses = listen_addrs;
                    } else if !observed_addr.is_empty() {
                        peer_info.addresses = vec![observed_addr];
                    }
                    let updated = peer_info.clone();
                    self.state_emit(SyncEvent::PeerDiscovered { peer: updated });
                } else {
                    let peer_info = PeerInfo {
                        peer_id: peer_id.to_string(),
                        display_name: device_name,
                        addresses: if !listen_addrs.is_empty() { listen_addrs } else { vec![observed_addr] },
                        is_connected: true,
                    };
                    self.discovered_peers.insert(peer_id, peer_info.clone());
                    self.state_emit(SyncEvent::PeerDiscovered { peer: peer_info });
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                log::info!("Connection established with {}", peer_id);
                if let Some(peer_info) = self.discovered_peers.get_mut(&peer_id) {
                    peer_info.is_connected = true;
                    let info = peer_info.clone();
                    self.state_emit(SyncEvent::PeerDiscovered { peer: info });
                } else {
                    let info = PeerInfo {
                        peer_id: peer_id.to_string(),
                        display_name: peer_id.to_string(),
                        addresses: vec![],
                        is_connected: true,
                    };
                    self.discovered_peers.insert(peer_id, info.clone());
                    self.state_emit(SyncEvent::PeerDiscovered { peer: info });
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                log::warn!("Connection closed with {}", peer_id);
                if let Some(peer_info) = self.discovered_peers.get_mut(&peer_id) {
                    peer_info.is_connected = false;
                    let info = peer_info.clone();
                    self.state_emit(SyncEvent::PeerDiscovered { peer: info });
                }
            }
            SwarmEvent::Behaviour(MaccyBehaviourEvent::Gossipsub(
                gossipsub::Event::Message { message, propagation_source, .. },
            )) => {
                let topic = message.topic.as_str();
                if topic == PAIRING_TOPIC {
                    if let Ok(pairing_msg) = serde_json::from_slice::<PairingMessage>(&message.data) {
                        self.handle_pairing_message(propagation_source, pairing_msg).await;
                    }
                } else if topic == TOPIC_NAME {
                    if self.paired_peers.contains_key(&propagation_source) {
                        if let Ok(sync_msg) = serde_json::from_slice::<SyncMessage>(&message.data) {
                            self.handle_sync_message(sync_msg);
                        }
                    }
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
                self.state_emit(SyncEvent::Listening { address: address.to_string() });
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                log::error!("Outgoing connection error: {:?} ({:?})", peer_id, error);
                self.emit_error(ErrorCode::Network, format!("Connection failed: {:?}", error));
            }
            SwarmEvent::IncomingConnectionError { error, .. } => {
                log::error!("Incoming connection error: {:?}", error);
            }
            SwarmEvent::ListenerError { error, .. } => {
                log::error!("Listener error: {:?}", error);
            }
            _ => {}
        }
    }

    // ── Sync / pairing message handlers ───────────────────────────

    fn handle_sync_message(&self, msg: SyncMessage) {
        match msg {
            SyncMessage::ItemAdded { item_json } => {
                self.state_emit(SyncEvent::ItemReceived { item_json });
            }
            SyncMessage::ItemDeleted { id, .. } => {
                self.state_emit(SyncEvent::ItemDeleted { item_id: id });
            }
            SyncMessage::ItemUpdated { item_json } => {
                self.state_emit(SyncEvent::ItemUpdated { item_json });
            }
            SyncMessage::Heartbeat { .. } => {}
        }
    }

    async fn handle_pairing_message(&mut self, peer: PeerId, msg: PairingMessage) {
        if let PairingMessage::Request { device_name, .. } = msg {
            self.state_emit(SyncEvent::PairingRequest {
                peer_id: peer.to_string(),
                display_name: device_name,
                pin: "000000".to_string(),
            });
        }
    }

    // ── Command handlers ──────────────────────────────────────────

    async fn handle_command(&mut self, command: SyncCommand) {
        match command {
            SyncCommand::BroadcastItem { item_json } => {
                let msg = SyncMessage::ItemAdded { item_json };
                self.broadcast_sync_message(msg);
            }
            SyncCommand::BroadcastDeletion { item_id } => {
                let msg = SyncMessage::ItemDeleted {
                    id: item_id,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                self.broadcast_sync_message(msg);
            }
            SyncCommand::BroadcastUpdate { item_json } => {
                let msg = SyncMessage::ItemUpdated { item_json };
                self.broadcast_sync_message(msg);
            }
            SyncCommand::StartDiscovery | SyncCommand::StopDiscovery => {}
            SyncCommand::RequestPairing { peer_id } => {
                if let Ok(peer) = peer_id.parse::<PeerId>() {
                    let (device_name, device_id) = {
                        let state = self.state.lock().unwrap();
                        (state.device_name.clone(), state.device_id.clone())
                    };
                    let request = PairingMessage::Request {
                        session_id: uuid::Uuid::new_v4().to_string(),
                        device_name,
                        device_id,
                        public_key: vec![],
                    };
                    if let Ok(data) = serde_json::to_vec(&request) {
                        let topic = IdentTopic::new(PAIRING_TOPIC);
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, data);
                    }
                    self.paired_peers.insert(peer, vec![]);
                }
            }
            SyncCommand::AcceptPairing { .. } | SyncCommand::RejectPairing { .. } => {}
            SyncCommand::AddPeerAddress { address } => {
                // Accept both "IP:Port" and "/ip4/.../tcp/..." formats
                let multiaddr = if address.starts_with('/') {
                    address.clone()
                } else {
                    parse_host_port_to_multiaddr(&address)
                };
                log::info!("Dialing {} (from {})", multiaddr, address);
                if let Ok(addr) = multiaddr.parse::<libp2p::Multiaddr>() {
                    match self.swarm.dial(addr.clone()) {
                        Ok(()) => log::info!("Dialing {}", addr),
                        Err(e) => {
                            log::error!("Failed to dial {}: {:?}", addr, e);
                            self.emit_error(ErrorCode::Network, format!("Failed to dial {}: {:?}", addr, e));
                        }
                    }
                } else {
                    log::error!("Invalid multiaddr: {}", multiaddr);
                    self.emit_error(ErrorCode::InvalidArg, format!("Invalid address: {}", address));
                }
            }
            SyncCommand::Unpair { peer_id } => {
                if let Ok(peer) = peer_id.parse::<PeerId>() {
                    self.paired_peers.remove(&peer);
                }
            }
            _ => {}
        }
    }

    fn broadcast_sync_message(&mut self, msg: SyncMessage) {
        if let Ok(data) = serde_json::to_vec(&msg) {
            let topic = IdentTopic::new(TOPIC_NAME);
            let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, data);
        }
    }
}

/// Parse "10.0.0.1:31774" or "[::1]:31774" into "/ip4/10.0.0.1/tcp/31774"
fn parse_host_port_to_multiaddr(input: &str) -> String {
    let input = input.trim();
    // IPv6 in brackets: [::1]:31774
    if let Some(rest) = input.strip_prefix('[') {
        if let Some(bracket_end) = rest.find("]:") {
            let host = &rest[..bracket_end];
            let port = &rest[bracket_end + 2..];
            return format!("/ip6/{}/tcp/{}", host, port);
        }
    }
    // IPv4: 10.0.0.1:31774
    if let Some(colon) = input.rfind(':') {
        let host = &input[..colon];
        let port = &input[colon + 1..];
        return format!("/ip4/{}/tcp/{}", host, port);
    }
    input.to_string()
}
