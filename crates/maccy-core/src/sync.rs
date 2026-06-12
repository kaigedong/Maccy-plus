/// Sync module placeholder.
/// Will be implemented after maccy-sync is refactored to expose a library API.
/// For now, sync functionality is stubbed out.

use crate::model::{ClipboardItem, CoreError};

pub struct SyncEngine {
    // Will hold maccy-sync handle when integrated
}

impl SyncEngine {
    pub fn new(_device_name: &str, _device_id: &str) -> Result<Self, CoreError> {
        Ok(SyncEngine {})
    }

    pub fn start(&self) -> Result<(), CoreError> {
        // TODO: integrate with maccy-sync
        Ok(())
    }

    pub fn stop(&self) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn broadcast_item(&self, _item: &ClipboardItem) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn broadcast_deletion(&self, _item_id: &str) -> Result<(), CoreError> {
        Ok(())
    }

    pub fn broadcast_update(&self, _item: &ClipboardItem) -> Result<(), CoreError> {
        Ok(())
    }
}
