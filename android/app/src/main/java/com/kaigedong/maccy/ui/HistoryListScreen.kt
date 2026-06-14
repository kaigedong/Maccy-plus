package com.kaigedong.maccy.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material.icons.outlined.PushPin
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.kaigedong.maccy.ClipboardItem
import com.kaigedong.maccy.ClipboardService
import com.kaigedong.maccy.HistoryViewModel
import com.kaigedong.maccy.LogManager

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryListScreen(viewModel: HistoryViewModel = viewModel()) {
    val context = LocalContext.current
    val items by viewModel.items.collectAsState()
    var searchQuery by remember { mutableStateOf("") }
    var showLogs by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        LogManager.i("Maccy", "App started, initializing...")
        viewModel.initialize(context)
        LogManager.i("Maccy", "History initialized, ${items.size} items loaded")

        // Start clipboard polling
        val clipboardService = ClipboardService(context)
        clipboardService.startPolling { item ->
            LogManager.d("Clipboard", "New clip: ${item.title.take(80)}")
            viewModel.addItem(item)
        }
    }

    // Log screen overlay
    if (showLogs) {
        LogScreen(onBack = { showLogs = false })
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Maccy") },
                actions = {
                    IconButton(onClick = { showLogs = true }) {
                        Icon(Icons.Filled.BugReport, "Logs")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Search bar
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { query ->
                    searchQuery = query
                    if (query.isNotEmpty()) {
                        viewModel.search(query)
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                placeholder = { Text("Search clipboard history...") },
                singleLine = true,
                leadingIcon = { Icon(Icons.Filled.Search, "Search") },
                trailingIcon = {
                    if (searchQuery.isNotEmpty()) {
                        IconButton(onClick = { searchQuery = "" }) {
                            Icon(Icons.Filled.Clear, "Clear")
                        }
                    }
                }
            )

            if (items.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            Icons.Filled.ContentPaste,
                            contentDescription = null,
                            modifier = Modifier.size(48.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            "Clipboard is empty",
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            "Copy something to see it here",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f)
                        )
                    }
                }
            } else {
                // History list
                LazyColumn(
                    modifier = Modifier.fillMaxSize()
                ) {
                    val displayItems = items

                    items(displayItems, key = { it.id }) { item ->
                        HistoryItemRow(
                            item = item,
                            onCopy = {
                                val service = ClipboardService(context)
                                service.copyToClipboard(item)
                                LogManager.d("Maccy", "Copied: ${item.title.take(80)}")
                            },
                            onDelete = {
                                viewModel.deleteItem(item.id)
                                LogManager.d("Maccy", "Deleted: ${item.title.take(80)}")
                            },
                            onTogglePin = {
                                viewModel.togglePin(item.id)
                                LogManager.d("Maccy", "Toggled pin: ${item.id.take(8)}")
                            }
                        )
                        HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
                    }
                }
            }
        }
    }
}

@Composable
fun HistoryItemRow(
    item: ClipboardItem,
    onCopy: () -> Unit,
    onDelete: () -> Unit,
    onTogglePin: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onCopy() }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = item.title.ifEmpty { "(no title)" },
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                item.application?.let { app ->
                    Text(
                        text = app,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                if (item.pin != null) {
                    if (item.application != null) {
                        Text(
                            " · ",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Icon(
                        Icons.Filled.PushPin,
                        contentDescription = "Pinned",
                        modifier = Modifier.size(12.dp),
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
            }
        }

        IconButton(onClick = onTogglePin) {
            Icon(
                imageVector = if (item.pin != null) Icons.Filled.PushPin else Icons.Outlined.PushPin,
                contentDescription = if (item.pin != null) "Unpin" else "Pin",
                tint = if (item.pin != null) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        IconButton(onClick = onCopy) {
            Icon(
                Icons.Filled.ContentCopy,
                contentDescription = "Copy",
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        IconButton(onClick = onDelete) {
            Icon(
                Icons.Filled.Delete,
                contentDescription = "Delete",
                tint = MaterialTheme.colorScheme.error
            )
        }
    }
}
