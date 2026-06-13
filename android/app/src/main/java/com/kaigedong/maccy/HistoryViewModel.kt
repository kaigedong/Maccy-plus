package com.kaigedong.maccy

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.io.File

class HistoryViewModel : ViewModel() {
    private var core: HistoryManager? = null
    private var syncObserver: MaccyClipboardObserver? = null

    private val _items = MutableStateFlow<List<ClipboardItem>>(emptyList())
    val items: StateFlow<List<ClipboardItem>> = _items

    private val _searchResults = MutableStateFlow<List<SearchResult>>(emptyList())
    val searchResults: StateFlow<List<SearchResult>> = _searchResults

    // Sync state
    private val _peers = MutableStateFlow<List<DiscoveredPeer>>(emptyList())
    val peers: StateFlow<List<DiscoveredPeer>> = _peers

    private val _pairingRequest = MutableStateFlow<PairingRequest?>(null)
    val pairingRequest: StateFlow<PairingRequest?> = _pairingRequest

    private val _syncError = MutableStateFlow<String?>(null)
    val syncError: StateFlow<String?> = _syncError

    fun initialize(context: Context) {
        val dbPath = File(context.filesDir, "maccy.db").absolutePath
        LogManager.i("History", "Opening database at $dbPath")
        try {
            core = HistoryManager(dbPath = dbPath)
            LogManager.i("History", "HistoryManager created successfully")
            loadItems()
        } catch (e: Exception) {
            LogManager.e("History", "Failed to create HistoryManager", e)
        }
    }

    // ── Sync ─────────────────────────────────────────────────────

    fun startSync(deviceName: String, deviceId: String) {
        val core = this.core ?: return
        syncObserver = MaccyClipboardObserver(
            onItemReceived = { item ->
                LogManager.i("Sync", "Received item: ${item.title.take(80)}")
                viewModelScope.launch {
                    try {
                        core.add(item, maxSize = 500, isUnlimited = false)
                        loadItems()
                    } catch (e: Exception) {
                        LogManager.e("Sync", "Failed to add synced item", e)
                    }
                }
            },
            onItemDeleted = { itemId ->
                LogManager.d("Sync", "Remote delete: $itemId")
                viewModelScope.launch {
                    try { core.delete(itemId); loadItems() }
                    catch (e: Exception) { LogManager.e("Sync", "Failed to delete synced item", e) }
                }
            },
            onItemUpdated = { item ->
                LogManager.d("Sync", "Remote update: ${item.title.take(80)}")
                viewModelScope.launch {
                    try {
                        core.add(item, maxSize = 500, isUnlimited = false)
                        loadItems()
                    } catch (e: Exception) { LogManager.e("Sync", "Failed to update synced item", e) }
                }
            },
            onPeerDiscovered = { peerId, displayName, addresses, isConnected ->
                LogManager.i("Sync", "Peer: $displayName (connected=$isConnected)")
                val list = _peers.value.toMutableList()
                list.removeAll { it.peerId == peerId }
                list.add(DiscoveredPeer(peerId, displayName, addresses.toList(), isConnected))
                _peers.value = list
            },
            onPeerLost = { peerId ->
                _peers.value = _peers.value.filter { it.peerId != peerId }
            },
            onPairingRequest = { peerId, displayName, pin ->
                LogManager.i("Sync", "Pairing request from $displayName (pin=$pin)")
                _pairingRequest.value = PairingRequest(peerId, displayName, pin)
            },
            onPairingComplete = { peerId, success ->
                LogManager.i("Sync", "Pairing complete: peer=$peerId success=$success")
                _pairingRequest.value = null
            },
            onListening = { address ->
                LogManager.i("Sync", "Listening on $address")
            },
            onError = { code, message ->
                LogManager.e("Sync", "Error $code: $message")
                _syncError.value = "Sync error: $message"
            }
        )

        try {
            core.startSync(deviceName, deviceId, syncObserver!!)
            LogManager.i("Sync", "Sync started (via HistoryManager)")
        } catch (e: Exception) {
            LogManager.e("Sync", "Failed to start sync", e)
        }
    }

    fun stopSync() {
        try {
            core?.stopSync()
            LogManager.i("Sync", "Sync stopped")
        } catch (e: Exception) {
            LogManager.e("Sync", "Failed to stop sync", e)
        }
        syncObserver = null
    }

    fun requestPairing(peerId: String) {
        core?.syncRequestPairing(peerId)
        LogManager.i("Sync", "Pairing requested with $peerId")
    }

    fun acceptPairing(peerId: String, pin: String) {
        core?.syncAcceptPairing(peerId, pin)
        LogManager.i("Sync", "Pairing accepted: $peerId")
    }

    fun rejectPairing(peerId: String) {
        core?.syncRejectPairing(peerId)
        _pairingRequest.value = null
    }

    fun unpair(peerId: String) {
        core?.syncUnpair(peerId)
        LogManager.i("Sync", "Unpaired: $peerId")
    }

    fun addPeerAddress(address: String) {
        core?.syncAddPeerAddress(address)
        LogManager.i("Sync", "Dialing: $address")
    }

    // ── History ──────────────────────────────────────────────────

    fun loadItems() {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    _items.value = manager.load()
                } catch (e: Exception) {
                    LogManager.e("History", "Failed to load items", e)
                    _items.value = emptyList()
                }
            }
        }
    }

    fun addItem(item: ClipboardItem) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    val result = manager.add(item, maxSize = 500, isUnlimited = false)
                    // Broadcast to peers
                    manager.syncBroadcastItem(result)
                    LogManager.d("History", "Added item: ${item.id.take(8)}...")
                } catch (e: Exception) {
                    LogManager.e("History", "Failed to add item", e)
                }
                loadItems()
            }
        }
    }

    fun deleteItem(id: String) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    manager.delete(id)
                    manager.syncBroadcastDeletion(id)
                    LogManager.d("History", "Deleted item: $id")
                } catch (e: Exception) {
                    LogManager.e("History", "Failed to delete item", e)
                }
                loadItems()
            }
        }
    }

    fun togglePin(id: String) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    manager.togglePin(id, listOf("b", "c", "d", "e", "f", "g", "h", "i", "j", "k"))
                } catch (e: Exception) {
                    LogManager.e("History", "Failed to toggle pin", e)
                }
                loadItems()
            }
        }
    }

    fun search(query: String, mode: SearchMode = SearchMode.Mixed) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    _searchResults.value = manager.search(query, _items.value, mode)
                } catch (e: Exception) {
                    LogManager.e("History", "Search failed", e)
                }
            }
        }
    }

    fun clearError() {
        _syncError.value = null
    }

    fun dismissPairingRequest() {
        _pairingRequest.value = null
    }

    override fun onCleared() {
        super.onCleared()
        stopSync()
        LogManager.i("History", "ViewModel cleared")
        core = null
    }
}

// ── Sync data classes ────────────────────────────────────────────

data class DiscoveredPeer(
    val peerId: String,
    val displayName: String,
    val addresses: List<String>,
    val isConnected: Boolean
)

data class PairingRequest(
    val peerId: String,
    val displayName: String,
    val pin: String
)

// ── ClipboardObserver UniFFI implementation ──────────────────────
// After UniFFI generates the ClipboardObserverInterface, this class implements it.
// If the generated name differs, search/replace "ClipboardObserverInterface" below.

class MaccyClipboardObserver(
    private val onItemReceivedCb: (ClipboardItem) -> Unit,
    private val onItemDeletedCb: (String) -> Unit,
    private val onItemUpdatedCb: (ClipboardItem) -> Unit,
    private val onPeerDiscoveredCb: (String, String, List<String>, Boolean) -> Unit,
    private val onPeerLostCb: (String) -> Unit,
    private val onPairingRequestCb: (String, String, String) -> Unit,
    private val onPairingCompleteCb: (String, Boolean) -> Unit,
    private val onListeningCb: (String) -> Unit,
    private val onErrorCb: (Int, String) -> Unit,
) : ClipboardObserverInterface {
    override fun onItemReceived(item: ClipboardItem) = onItemReceivedCb(item)
    override fun onItemDeleted(itemId: String) = onItemDeletedCb(itemId)
    override fun onItemUpdated(item: ClipboardItem) = onItemUpdatedCb(item)
    override fun onPeerDiscovered(peerId: String, displayName: String, addresses: List<String>, isConnected: Boolean) =
        onPeerDiscoveredCb(peerId, displayName, addresses, isConnected)
    override fun onPeerLost(peerId: String) = onPeerLostCb(peerId)
    override fun onPairingRequest(peerId: String, displayName: String, pin: String) =
        onPairingRequestCb(peerId, displayName, pin)
    override fun onPairingComplete(peerId: String, success: Boolean) = onPairingCompleteCb(peerId, success)
    override fun onListening(address: String) = onListeningCb(address)
    override fun onError(code: Int, message: String) = onErrorCb(code, message)
}
