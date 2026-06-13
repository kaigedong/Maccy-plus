use std::sync::Arc;

use base64::Engine;
use maccy_sync::{NetworkManager, SharedState, SyncCommand, SyncEvent, SyncState};
use tokio::sync::mpsc;

use crate::model::{ClipboardContent, ClipboardItem, CoreError};
use crate::platform::ClipboardObserver;

/// P2P sync engine wrapping maccy-sync's NetworkManager.
/// Routes all events through the platform's `ClipboardObserver` implementation.
pub struct SyncEngine {
    state: SharedState,
    command_tx: mpsc::UnboundedSender<SyncCommand>,
    #[allow(dead_code)]
    observer: Arc<dyn ClipboardObserver>, // kept alive so the callback's clone stays valid
}

impl SyncEngine {
    /// Create and start the sync engine.
    /// Spawns a background thread with its own tokio runtime for the libp2p network.
    pub fn start(
        device_name: &str,
        device_id: &str,
        observer: Arc<dyn ClipboardObserver>,
    ) -> Result<Self, CoreError> {
        let sync_state = SyncState::new(device_name, device_id).map_err(|e| CoreError::Sync {
            message: format!("Failed to create sync state: {:?}", e),
        })?;
        let state = Arc::new(std::sync::Mutex::new(sync_state));
        let obs = observer.clone();

        // Register the unified event callback — dispatches to observer trait methods
        {
            let obs = observer.clone();
            state.lock().unwrap().on_event.lock().replace(Box::new(move |json: &str| {
                if let Ok(event) = serde_json::from_str::<SyncEvent>(json) {
                    match event {
                        SyncEvent::ItemReceived { item_json } => {
                            if let Ok(item) = Self::deserialize_item(&item_json) {
                                obs.on_item_received(item);
                            }
                        }
                        SyncEvent::ItemDeleted { item_id } => {
                            obs.on_item_deleted(item_id);
                        }
                        SyncEvent::ItemUpdated { item_json } => {
                            if let Ok(item) = Self::deserialize_item(&item_json) {
                                obs.on_item_updated(item);
                            }
                        }
                        SyncEvent::PeerDiscovered { peer } => {
                            obs.on_peer_discovered(
                                peer.peer_id,
                                peer.display_name,
                                peer.addresses,
                                peer.is_connected,
                            );
                        }
                        SyncEvent::PeerLost { peer_id } => {
                            obs.on_peer_lost(peer_id);
                        }
                        SyncEvent::PairingRequest { peer_id, display_name, pin } => {
                            obs.on_pairing_request(peer_id, display_name, pin);
                        }
                        SyncEvent::PairingComplete { peer_id, success } => {
                            obs.on_pairing_complete(peer_id, success);
                        }
                        SyncEvent::Listening { address } => {
                            obs.on_listening(address);
                        }
                        SyncEvent::Error { code, message } => {
                            obs.on_error(code, message);
                        }
                    }
                }
            }));
        }

        // Spawn the network manager in a background thread
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let net_state = state.clone();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(_) => return,
            };
            let mut mgr = match NetworkManager::new(command_rx, net_state, local_key) {
                Ok(m) => m,
                Err(_) => return,
            };
            rt.block_on(mgr.run());
        });

        // Send StartDiscovery so peers can find us
        let _ = command_tx.send(SyncCommand::StartDiscovery);

        Ok(SyncEngine {
            state,
            command_tx,
            observer: obs,
        })
    }

    /// Stop the sync engine.
    pub fn stop(&self) {
        let _ = self.command_tx.send(SyncCommand::Shutdown);
    }

    // ── Peer management ───────────────────────────────────────────

    pub fn add_peer_address(&self, address: &str) {
        let _ = self.command_tx.send(SyncCommand::AddPeerAddress { address: address.to_string() });
    }

    pub fn start_discovery(&self) {
        let _ = self.command_tx.send(SyncCommand::StartDiscovery);
    }

    pub fn stop_discovery(&self) {
        let _ = self.command_tx.send(SyncCommand::StopDiscovery);
    }

    // ── Pairing ───────────────────────────────────────────────────

    pub fn request_pairing(&self, peer_id: &str) {
        let _ = self.command_tx.send(SyncCommand::RequestPairing { peer_id: peer_id.to_string() });
    }

    pub fn accept_pairing(&self, peer_id: &str, pin: &str) {
        let _ = self.command_tx.send(SyncCommand::AcceptPairing { peer_id: peer_id.to_string(), pin: pin.to_string() });
    }

    pub fn reject_pairing(&self, peer_id: &str) {
        let _ = self.command_tx.send(SyncCommand::RejectPairing { peer_id: peer_id.to_string() });
    }

    pub fn unpair(&self, peer_id: &str) {
        let _ = self.command_tx.send(SyncCommand::Unpair { peer_id: peer_id.to_string() });
    }

    // ── Broadcast ─────────────────────────────────────────────────

    pub fn broadcast_item(&self, item: &ClipboardItem) {
        let json = Self::serialize_item(item);
        let _ = self.command_tx.send(SyncCommand::BroadcastItem { item_json: json });
    }

    pub fn broadcast_deletion(&self, item_id: &str) {
        let _ = self.command_tx.send(SyncCommand::BroadcastDeletion { item_id: item_id.to_string() });
    }

    pub fn broadcast_update(&self, item: &ClipboardItem) {
        let json = Self::serialize_item(item);
        let _ = self.command_tx.send(SyncCommand::BroadcastUpdate { item_json: json });
    }

    // ── Serialization (canonical — platforms don't duplicate this) ─

    fn serialize_item(item: &ClipboardItem) -> String {
        let sync_item = maccy_sync::SyncItem {
            id: item.id.clone(),
            application: item.application.clone(),
            first_copied_at: Self::format_timestamp(item.first_copied_at),
            last_copied_at: Self::format_timestamp(item.last_copied_at),
            number_of_copies: item.number_of_copies as i64,
            pin: item.pin.clone(),
            title: item.title.clone(),
            contents: item
                .contents
                .iter()
                .map(|c| maccy_sync::SyncItemContent {
                    content_type: c.content_type.clone(),
                    value: c.value.as_ref().map(|v| base64::engine::general_purpose::STANDARD.encode(v)),
                })
                .collect(),
            sync_timestamp: Self::format_timestamp(item.sync_timestamp),
            sync_source: item.sync_source.clone().unwrap_or_default(),
        };
        serde_json::to_string(&sync_item).unwrap_or_default()
    }

    fn deserialize_item(json: &str) -> Result<ClipboardItem, ()> {
        let sync_item: maccy_sync::SyncItem = serde_json::from_str(json).map_err(|_| ())?;
        Ok(ClipboardItem {
            id: sync_item.id,
            application: sync_item.application,
            first_copied_at: Self::parse_timestamp(&sync_item.first_copied_at),
            last_copied_at: Self::parse_timestamp(&sync_item.last_copied_at),
            number_of_copies: sync_item.number_of_copies as i32,
            pin: sync_item.pin,
            title: sync_item.title,
            contents: sync_item
                .contents
                .into_iter()
                .map(|c| ClipboardContent {
                    content_type: c.content_type,
                    value: c.value.map(|v| base64::engine::general_purpose::STANDARD.decode(v).unwrap_or_default()),
                })
                .collect(),
            sync_timestamp: Self::parse_timestamp(&sync_item.sync_timestamp),
            sync_source: if sync_item.sync_source.is_empty() { None } else { Some(sync_item.sync_source) },
            sync_deleted: false,
        })
    }

    fn format_timestamp(millis: i64) -> String {
        chrono::DateTime::from_timestamp_millis(millis)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }

    fn parse_timestamp(s: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0)
    }
}
