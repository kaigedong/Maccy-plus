use serde::{Deserialize, Serialize};

/// Information about a discovered or paired peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub display_name: String,
    pub addresses: Vec<String>,
    pub is_connected: bool,
}

/// A syncable clipboard item serialized from Swift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncItem {
    pub id: String,
    pub application: Option<String>,
    pub first_copied_at: String,
    pub last_copied_at: String,
    pub number_of_copies: i64,
    pub pin: Option<String>,
    pub title: String,
    pub contents: Vec<SyncItemContent>,
    pub sync_timestamp: String,
    pub sync_source: String,
}

/// One content variant of a clipboard item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncItemContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub value: Option<String>,
}

/// Messages sent over gossipsub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    ItemAdded { item_json: String },
    ItemDeleted { id: String, timestamp: String },
    ItemUpdated { item_json: String },
    Heartbeat { device_id: String, timestamp: String },
}

/// Pairing protocol messages sent over request-response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PairingMessage {
    Request {
        session_id: String,
        device_name: String,
        device_id: String,
        public_key: Vec<u8>,
    },
    Challenge {
        session_id: String,
        pin: String,
        device_name: String,
        public_key: Vec<u8>,
    },
    Confirm {
        session_id: String,
    },
    Reject {
        session_id: String,
    },
}

/// Sync request for initial/reconnection bulk sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BulkSyncMessage {
    Request { since_timestamp: String },
    Response { items_json: String },
}

/// Gossipsub topic name.
pub const TOPIC_NAME: &str = "maccy-sync-v1";

/// Protocol name for pairing request-response.
pub const PAIRING_PROTOCOL: &str = "/maccy-sync/pairing/1";

/// Protocol name for bulk sync request-response.
pub const BULK_SYNC_PROTOCOL: &str = "/maccy-sync/bulk/1";

/// Fixed listen port for reliable reconnection.
pub const LISTEN_PORT: u16 = 31774;
