use std::sync::Arc;

use base64::Engine;
use maccy_sync::{NetworkManager, SharedState, SyncCommand, SyncEvent, SyncState};
use serde::Deserialize;

use crate::model::{ClipboardContent, ClipboardItem, CoreError};
use crate::platform::ClipboardObserver;

/// P2P sync engine wrapping maccy-sync's NetworkManager.
pub struct SyncEngine {
    state: SharedState,
    #[allow(dead_code)]
    observer: Arc<dyn ClipboardObserver>,
}

impl SyncEngine {
    pub fn new(
        device_name: &str,
        device_id: &str,
        observer: Arc<dyn ClipboardObserver>,
    ) -> Result<Self, CoreError> {
        let sync_state = SyncState::new(device_name, device_id).map_err(|e| CoreError::Sync {
            message: format!("Failed to create sync state: {:?}", e),
        })?;
        let state = Arc::new(std::sync::Mutex::new(sync_state));

        // Register the single unified event callback
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
                        obs.on_peer_discovered(peer.peer_id, peer.display_name);
                    }
                    SyncEvent::PeerLost { peer_id } => {
                        obs.on_peer_lost(peer_id);
                    }
                    SyncEvent::Error { code, message } => {
                        log::error!("Sync error ({}): {}", code, message);
                    }
                    _ => {}
                }
            }
        }));

        Ok(SyncEngine { state, observer })
    }

    pub fn start(&self) -> Result<(), CoreError> {
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn the network manager in a background thread with its own tokio runtime
        let state = self.state.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(_) => return,
            };
            let mut mgr = match NetworkManager::new(command_rx, state, local_key) {
                Ok(m) => m,
                Err(_) => return,
            };
            rt.block_on(mgr.run());
        });

        // Store the command sender
        {
            let mut locked = self.state.lock().unwrap();
            locked.command_tx = command_tx;
        }

        Ok(())
    }

    pub fn stop(&self) -> Result<(), CoreError> {
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::Shutdown);
        Ok(())
    }

    pub fn broadcast_item(&self, item: &ClipboardItem) -> Result<(), CoreError> {
        let json = Self::serialize_item(item);
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::BroadcastItem { item_json: json });
        Ok(())
    }

    pub fn broadcast_deletion(&self, item_id: &str) -> Result<(), CoreError> {
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::BroadcastDeletion { item_id: item_id.to_string() });
        Ok(())
    }

    pub fn broadcast_update(&self, item: &ClipboardItem) -> Result<(), CoreError> {
        let json = Self::serialize_item(item);
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::BroadcastUpdate { item_json: json });
        Ok(())
    }

    pub fn request_pairing(&self, peer_id: &str) -> Result<(), CoreError> {
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::RequestPairing { peer_id: peer_id.to_string() });
        Ok(())
    }

    pub fn unpair(&self, peer_id: &str) -> Result<(), CoreError> {
        let state = self.state.lock().unwrap();
        let _ = state.command_tx.send(SyncCommand::Unpair { peer_id: peer_id.to_string() });
        Ok(())
    }

    // Serialization helpers

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
