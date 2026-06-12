package com.kaigedong.maccy

import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import java.util.UUID

class ClipboardService(private val context: Context) {
    private val clipboardManager =
        context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    private var lastClipHash: Int = 0
    private val handler = Handler(Looper.getMainLooper())
    private var pollingRunnable: Runnable? = null

    fun startPolling(intervalMs: Long = 500L, onNewClip: (ClipboardItem) -> Unit) {
        pollingRunnable = object : Runnable {
            override fun run() {
                val clip = clipboardManager.primaryClip
                if (clip != null && clip.hashCode() != lastClipHash) {
                    lastClipHash = clip.hashCode()
                    val item = clipToItem(clip)
                    if (item != null) {
                        onNewClip(item)
                    }
                }
                handler.postDelayed(this, intervalMs)
            }
        }
        handler.post(pollingRunnable!!)
    }

    fun stopPolling() {
        pollingRunnable?.let { handler.removeCallbacks(it) }
        pollingRunnable = null
    }

    private fun clipToItem(clip: android.content.ClipData): ClipboardItem? {
        val contents = mutableListOf<ClipboardContent>()
        for (i in 0 until clip.itemCount) {
            val item = clip.getItemAt(i)
            item.text?.let { text ->
                contents.add(
                    ClipboardContent(
                        contentType = "text/plain",
                        value = text.toString().toByteArray(Charsets.UTF_8)
                    )
                )
            }
            item.uri?.let { uri ->
                contents.add(
                    ClipboardContent(
                        contentType = "text/uri-list",
                        value = uri.toString().toByteArray(Charsets.UTF_8)
                    )
                )
            }
        }
        if (contents.isEmpty()) return null

        val nowMs = System.currentTimeMillis()
        return ClipboardItem(
            id = UUID.randomUUID().toString(),
            application = null,
            firstCopiedAt = nowMs,
            lastCopiedAt = nowMs,
            numberOfCopies = 1,
            pin = null,
            title = contents.firstNotNullOfOrNull { c ->
                c.value?.toString(Charsets.UTF_8)
            }?.take(200) ?: "",
            contents = contents,
            syncTimestamp = nowMs,
            syncSource = null,
            syncDeleted = false
        )
    }

    fun copyToClipboard(item: ClipboardItem) {
        val text = item.contents.firstOrNull {
            it.contentType == "text/plain"
        }?.value?.let { String(it, Charsets.UTF_8) } ?: return

        val clip = android.content.ClipData.newPlainText("Maccy", text)
        clipboardManager.setPrimaryClip(clip)
    }
}
