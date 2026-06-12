use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::runtime::Runtime;

use crate::error::ErrorCode;
use crate::types::*;

type CallbackFn<T> = Arc<Mutex<Option<Box<dyn Fn(T) + Send + Sync>>>>;

pub type SharedState = Arc<std::sync::Mutex<SyncState>>;

pub struct SyncState {
    pub runtime: Runtime,
    pub command_tx: mpsc::UnboundedSender<SyncCommand>,
    pub device_id: String,
    pub device_name: String,

    pub on_peer_discovered: CallbackFn<PeerInfo>,
    pub on_peer_lost: CallbackFn<String>,
    pub on_pairing_request: CallbackFn<(String, String, String)>,
    pub on_pairing_complete: CallbackFn<(String, bool)>,
    pub on_item_received: CallbackFn<String>,
    pub on_item_deleted: CallbackFn<String>,
    pub on_item_updated: CallbackFn<String>,
    pub on_error: CallbackFn<(i32, String)>,
}

impl SyncState {
    pub fn new(device_name: &str, device_id: &str) -> Result<Self, ErrorCode> {
        let runtime = Runtime::new().map_err(|_| ErrorCode::Init)?;
        let (command_tx, _command_rx) = mpsc::unbounded_channel();

        Ok(Self {
            runtime,
            command_tx,
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            on_peer_discovered: Arc::new(Mutex::new(None)),
            on_peer_lost: Arc::new(Mutex::new(None)),
            on_pairing_request: Arc::new(Mutex::new(None)),
            on_pairing_complete: Arc::new(Mutex::new(None)),
            on_item_received: Arc::new(Mutex::new(None)),
            on_item_deleted: Arc::new(Mutex::new(None)),
            on_item_updated: Arc::new(Mutex::new(None)),
            on_error: Arc::new(Mutex::new(None)),
        })
    }

    pub fn emit_peer_discovered(&self, peer: PeerInfo) {
        if let Some(cb) = self.on_peer_discovered.lock().as_ref() {
            cb(peer);
        }
    }

    pub fn emit_peer_lost(&self, peer_id: String) {
        if let Some(cb) = self.on_peer_lost.lock().as_ref() {
            cb(peer_id);
        }
    }

    pub fn emit_pairing_request(&self, peer_id: String, display_name: String, pin: String) {
        if let Some(cb) = self.on_pairing_request.lock().as_ref() {
            cb((peer_id, display_name, pin));
        }
    }

    pub fn emit_pairing_complete(&self, peer_id: String, success: bool) {
        if let Some(cb) = self.on_pairing_complete.lock().as_ref() {
            cb((peer_id, success));
        }
    }

    pub fn emit_item_received(&self, item_json: String) {
        if let Some(cb) = self.on_item_received.lock().as_ref() {
            cb(item_json);
        }
    }

    pub fn emit_item_deleted(&self, item_id: String) {
        if let Some(cb) = self.on_item_deleted.lock().as_ref() {
            cb(item_id);
        }
    }

    pub fn emit_item_updated(&self, item_json: String) {
        if let Some(cb) = self.on_item_updated.lock().as_ref() {
            cb(item_json);
        }
    }

    pub fn emit_error(&self, code: ErrorCode, message: String) {
        if let Some(cb) = self.on_error.lock().as_ref() {
            cb((code as i32, message));
        }
    }
}

#[derive(Debug)]
pub enum SyncCommand {
    StartListening,
    StopListening,
    StartDiscovery,
    StopDiscovery,
    RequestPairing { peer_id: String },
    AcceptPairing { peer_id: String, pin: String },
    RejectPairing { peer_id: String },
    BroadcastItem { item_json: String },
    BroadcastDeletion { item_id: String },
    BroadcastUpdate { item_json: String },
    AddPeerAddress { peer_id: String, address: String },
    Unpair { peer_id: String },
    Shutdown,
}
