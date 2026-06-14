package com.kaigedong.maccy

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.PushPin
import androidx.compose.material.icons.outlined.PushPin
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HistoryListScreen(viewModel: HistoryViewModel = viewModel()) {
    val context = LocalContext.current
    val items by viewModel.items.collectAsState()
    var searchQuery by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        viewModel.initialize(context)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Maccy") },
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
                singleLine = true
            )

            // History list
            LazyColumn(
                modifier = Modifier.fillMaxSize()
            ) {
                val displayItems = if (searchQuery.isEmpty()) items else {
                    // Show search results when query is active
                    items // TODO: use searchResults from ViewModel
                }

                items(displayItems, key = { it.id }) { item ->
                    HistoryItemRow(
                        item = item,
                        onCopy = {
                            val service = ClipboardService(context)
                            service.copyToClipboard(item)
                        },
                        onDelete = { viewModel.deleteItem(item.id) },
                        onTogglePin = { viewModel.togglePin(item.id) }
                    )
                    HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))
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
            item.application?.let { app ->
                Text(
                    text = app,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
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
