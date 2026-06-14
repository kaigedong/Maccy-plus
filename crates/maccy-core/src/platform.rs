use crate::model::ClipboardItem;

/// Callback interface that platforms implement to receive sync events.
/// UniFFI generates a protocol in Swift and an interface in Kotlin.
#[uniffi::export(with_foreign)]
pub trait ClipboardObserver: Send + Sync + std::fmt::Debug {
    /// Called when a new clipboard item is received via P2P sync.
    fn on_item_received(&self, item: ClipboardItem);

    /// Called when an item is deleted via sync.
    fn on_item_deleted(&self, item_id: String);

    /// Called when an item is updated via sync.
    fn on_item_updated(&self, item: ClipboardItem);

    /// Called when a peer is discovered on the network.
    fn on_peer_discovered(&self, peer_id: String, display_name: String);

    /// Called when a peer disconnects.
    fn on_peer_lost(&self, peer_id: String);
}
