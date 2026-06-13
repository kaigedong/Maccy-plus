use std::sync::{Arc, Mutex};

use crate::model::{ClipboardContent, ClipboardItem, CoreError, SearchMode, SearchResult, SortBy};
use crate::platform::ClipboardObserver;
use crate::search::SearchEngine;
use crate::sort::sort_items;
use crate::storage::Storage;

/// Central coordinator for clipboard history management.
/// Delegates to Storage for persistence, SearchEngine for queries,
/// and optionally a sync engine for P2P sync.
#[derive(uniffi::Object)]
pub struct HistoryManager {
    storage: Mutex<Storage>,
    observer: Mutex<Option<Arc<dyn ClipboardObserver>>>,
}

#[uniffi::export]
impl HistoryManager {
    #[uniffi::constructor]
    pub fn new(db_path: String) -> Result<Self, CoreError> {
        let storage = Storage::open(&db_path)?;
        Ok(HistoryManager {
            storage: Mutex::new(storage),
            observer: Mutex::new(None),
        })
    }

    /// Load all items from storage.
    pub fn load(&self) -> Result<Vec<ClipboardItem>, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.get_all_items()
    }

    /// Add a new clipboard item. Handles deduplication and size limiting.
    /// Returns the resulting item (which may be an updated existing item if deduplicated).
    pub fn add(
        &self,
        item: ClipboardItem,
        max_size: i32,
        is_unlimited: bool,
    ) -> Result<ClipboardItem, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;

        // Deduplication: check if the new item supersedes an existing one
        let existing_items = storage.get_all_items()?;
        if let Some(similar) = find_similar(&item, &existing_items) {
            // Update existing item
            let mut updated = similar.clone();
            updated.last_copied_at = item.last_copied_at;
            updated.number_of_copies += 1;
            // Merge contents if the new item has new types
            for new_content in &item.contents {
                let has_type = updated
                    .contents
                    .iter()
                    .any(|c| c.content_type == new_content.content_type);
                if !has_type {
                    updated.contents.push(new_content.clone());
                }
            }
            // Update value for existing content types
            for new_content in &item.contents {
                if let Some(existing) = updated
                    .contents
                    .iter_mut()
                    .find(|c| c.content_type == new_content.content_type)
                {
                    existing.value = new_content.value.clone();
                }
            }
            storage.update_item(&updated)?;
            return Ok(updated);
        }

        // Enforce size limit
        if !is_unlimited && max_size > 0 {
            let count = storage.count_items()?;
            if count >= max_size as i64 {
                // Delete oldest unpinned items to make room
                let all = storage.get_all_items()?;
                let unpinned: Vec<&ClipboardItem> =
                    all.iter().filter(|i| i.pin.is_none()).collect();
                let excess = (count - max_size as i64 + 1) as usize;
                for item in unpinned.iter().rev().take(excess) {
                    storage.delete_item(&item.id)?;
                }
            }
        }

        storage.insert_item(&item)?;
        Ok(item)
    }

    /// Delete an item by ID.
    pub fn delete(&self, id: String) -> Result<(), CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.delete_item(&id)
    }

    /// Delete all unpinned items.
    pub fn clear_unpinned(&self) -> Result<u64, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.delete_unpinned()
    }

    /// Delete all items (including pinned).
    pub fn clear_all(&self) -> Result<u64, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.delete_all()
    }

    /// Toggle pin on an item. If pinning, assigns the first available pin from the list.
    pub fn toggle_pin(
        &self,
        id: String,
        available_pins: Vec<String>,
    ) -> Result<ClipboardItem, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;

        let mut item = storage
            .get_item(&id)?
            .ok_or(CoreError::NotFound { id: id.clone() })?;

        if item.pin.is_some() {
            // Unpin
            item.pin = None;
        } else {
            // Pin with first available key
            item.pin = available_pins.first().cloned();
        }

        storage.update_item(&item)?;
        Ok(item)
    }

    /// Update an item's text content.
    pub fn update_item_text(
        &self,
        id: String,
        new_text: String,
    ) -> Result<ClipboardItem, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;

        let mut item = storage
            .get_item(&id)?
            .ok_or(CoreError::NotFound { id: id.clone() })?;

        // Update the string content
        let text_type = "public.utf8-plain-text";
        if let Some(content) = item
            .contents
            .iter_mut()
            .find(|c| c.content_type == text_type)
        {
            content.value = Some(new_text.as_bytes().to_vec());
        }

        item.title = new_text;
        storage.update_item(&item)?;
        Ok(item)
    }

    /// Search items.
    pub fn search(
        &self,
        query: &str,
        items: Vec<ClipboardItem>,
        mode: SearchMode,
    ) -> Vec<SearchResult> {
        SearchEngine::search(query, &items, mode)
    }

    /// Sort items.
    pub fn sort(
        &self,
        items: Vec<ClipboardItem>,
        sort_by: SortBy,
        pin_to_top: bool,
    ) -> Vec<ClipboardItem> {
        sort_items(items, sort_by, pin_to_top)
    }

    /// Register a clipboard observer for sync events.
    pub fn set_observer(&self, observer: Arc<dyn ClipboardObserver>) {
        let mut obs = self.observer.lock().unwrap();
        *obs = Some(observer);
    }

    /// Get the database file size in bytes.
    pub fn storage_size_bytes(&self, db_path: String) -> i64 {
        self.storage
            .lock()
            .map(|s| s.db_size_bytes(&db_path))
            .unwrap_or(0)
    }

    /// Get total item count.
    pub fn count(&self) -> Result<i64, CoreError> {
        let storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.count_items()
    }

    /// Migrate from SwiftData database.
    pub fn migrate_from_swiftdata(&self, swiftdata_path: String) -> Result<u64, CoreError> {
        let mut storage = self.storage.lock().map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        storage.migrate_from_swiftdata(&swiftdata_path)
    }
}

/// Find an existing item that is similar to the new one (deduplication).
/// An item is similar if all non-transient content types match in type and value.
fn find_similar<'a>(new_item: &ClipboardItem, existing: &'a [ClipboardItem]) -> Option<&'a ClipboardItem> {
    // Transient types that should be ignored during comparison
    let transient_types = [
        "com.apple.modified",
        "com.maccy.from-maccy",
        "com.apple.linkpresentation.metadata",
        "com.apple.custom-webkit-pasteboard-data",
        "com.apple.pasteboard.promised-file-url",
        "org.chromium.source-url",
        "org.chromium.source-web-custom-data",
        "com.apple.NSItemProvider.Providers",
    ];

    existing.iter().find(|existing| {
        // Check if the new item's non-transient contents are a subset of the existing item's contents
        let new_non_transient: Vec<&ClipboardContent> = new_item
            .contents
            .iter()
            .filter(|c| !transient_types.contains(&c.content_type.as_str()))
            .collect();

        if new_non_transient.is_empty() {
            return false;
        }

        new_non_transient.iter().all(|new_content| {
            existing.contents.iter().any(|existing_content| {
                existing_content.content_type == new_content.content_type
                    && existing_content.value == new_content.value
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, title: &str, types: &[(&str, &[u8])]) -> ClipboardItem {
        ClipboardItem {
            id: id.to_string(),
            application: None,
            first_copied_at: 1000,
            last_copied_at: 1000,
            number_of_copies: 1,
            pin: None,
            title: title.to_string(),
            contents: types
                .iter()
                .map(|(t, v)| ClipboardContent {
                    content_type: t.to_string(),
                    value: Some(v.to_vec()),
                })
                .collect(),
            sync_timestamp: 1000,
            sync_source: None,
            sync_deleted: false,
        }
    }

    #[test]
    fn test_add_and_load() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();
        let item = make_item("1", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        mgr.add(item, 100, false).unwrap();

        let items = mgr.load().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Hello");
    }

    #[test]
    fn test_delete() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();
        let item = make_item("1", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        mgr.add(item, 100, false).unwrap();
        mgr.delete("1".to_string()).unwrap();

        let items = mgr.load().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();

        let item1 = make_item("1", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        mgr.add(item1, 100, false).unwrap();

        let mut item2 = make_item("2", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        item2.last_copied_at = 2000;
        let result = mgr.add(item2, 100, false).unwrap();

        // Should have deduplicated (updated existing item, not added new one)
        assert_eq!(result.id, "1");
        assert_eq!(result.number_of_copies, 2);
        assert_eq!(result.last_copied_at, 2000);

        let items = mgr.load().unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_toggle_pin() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();
        let item = make_item("1", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        mgr.add(item, 100, false).unwrap();

        let pinned = mgr.toggle_pin("1".to_string(), vec!["b".to_string()]).unwrap();
        assert_eq!(pinned.pin, Some("b".to_string()));

        let unpinned = mgr.toggle_pin("1".to_string(), vec![]).unwrap();
        assert!(unpinned.pin.is_none());
    }

    #[test]
    fn test_size_limit() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();

        for i in 0..5 {
            let item = make_item(
                &format!("{}", i),
                &format!("Item {}", i),
                &[("public.utf8-plain-text", b"text")],
            );
            mgr.add(item, 3, false).unwrap();
        }

        let items = mgr.load().unwrap();
        assert!(items.len() <= 3);
    }

    #[test]
    fn test_clear_unpinned() {
        let mgr = HistoryManager::new(":memory:".to_string()).unwrap();

        let mut item1 = make_item("1", "Hello", &[("public.utf8-plain-text", b"Hello")]);
        item1.pin = Some("b".to_string());
        mgr.add(item1, 100, false).unwrap();

        let item2 = make_item("2", "World", &[("public.utf8-plain-text", b"World")]);
        mgr.add(item2, 100, false).unwrap();

        let deleted = mgr.clear_unpinned().unwrap();
        assert_eq!(deleted, 1);

        let items = mgr.load().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "1");
    }
}
