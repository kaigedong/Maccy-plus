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

    private val _items = MutableStateFlow<List<ClipboardItem>>(emptyList())
    val items: StateFlow<List<ClipboardItem>> = _items

    private val _searchResults = MutableStateFlow<List<SearchResult>>(emptyList())
    val searchResults: StateFlow<List<SearchResult>> = _searchResults

    fun initialize(context: Context) {
        val dbPath = File(context.filesDir, "maccy.db").absolutePath
        core = HistoryManager(dbPath = dbPath)
        loadItems()
    }

    fun loadItems() {
        viewModelScope.launch {
            core?.let { manager ->
                _items.value = try {
                    manager.load()
                } catch (e: Exception) {
                    emptyList()
                }
            }
        }
    }

    fun addItem(item: ClipboardItem) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    manager.add(item, maxSize = 500, isUnlimited = false)
                } catch (_: Exception) {}
                loadItems()
            }
        }
    }

    fun deleteItem(id: String) {
        viewModelScope.launch {
            core?.let { manager ->
                try { manager.delete(id) } catch (_: Exception) {}
                loadItems()
            }
        }
    }

    fun togglePin(id: String) {
        viewModelScope.launch {
            core?.let { manager ->
                try {
                    manager.togglePin(id, listOf("b", "c", "d", "e", "f", "g", "h", "i", "j", "k"))
                } catch (_: Exception) {}
                loadItems()
            }
        }
    }

    fun search(query: String, mode: SearchMode = SearchMode.Mixed) {
        viewModelScope.launch {
            core?.let { manager ->
                _searchResults.value = manager.search(query, _items.value, mode)
            }
        }
    }

    override fun onCleared() {
        super.onCleared()
        core = null
    }
}
