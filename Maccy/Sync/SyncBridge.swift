import AppKit
import Defaults
import Foundation

@MainActor
class SyncBridge {
  static let shared = SyncBridge()

  private var syncHandle: OpaquePointer?
  private var isStarted = false
  /// Cache peer_id → display_name from peer_discovered events for saving on pairing_complete
  private var peerDisplayNames: [String: String] = [:]

  private init() {}

  var isEnabled: Bool { Defaults[.syncEnabled] }

  func start() {
    guard !isStarted else { return }
    guard isEnabled else { return }

    let deviceName = Defaults[.syncDeviceName]
    let deviceID = Defaults[.syncDeviceID]
    NSLog("[Sync] start: name=\(deviceName)")

    guard let handle = maccy_sync_create(deviceName, deviceID) else {
      NSLog("[Sync] start: create failed")
      return
    }
    syncHandle = handle

    // Register single unified callback — read C string immediately
    maccy_sync_on_event(handle) { eventJSON in
      let json = String(cString: eventJSON!)
      DispatchQueue.main.async {
        SyncBridge.shared.handleEvent(json)
      }
    }

    let result = maccy_sync_start(handle)
    if result != MACCY_SYNC_OK {
      NSLog("[Sync] start: failed with \(result)")
      maccy_sync_destroy(handle)
      syncHandle = nil
      return
    }

    _ = maccy_sync_start_discovery(handle)
    isStarted = true
    NSLog("[Sync] started")
  }

  func stop() {
    guard isStarted, let handle = syncHandle else { return }
    _ = maccy_sync_stop_discovery(handle)
    _ = maccy_sync_stop(handle)
    maccy_sync_destroy(handle)
    syncHandle = nil
    isStarted = false
  }

  // ── Thin wrappers — all logic is in Rust ────────────────────────

  func addPeerAddress(_ address: String) {
    guard isStarted, let handle = syncHandle else { return }
    address.withCString { ptr in
      _ = maccy_sync_add_peer_address(handle, "", ptr)
    }
  }

  func refreshDiscovery() {
    guard isStarted, let handle = syncHandle else { return }
    _ = maccy_sync_stop_discovery(handle)
    _ = maccy_sync_start_discovery(handle)
  }

  func broadcastNewItem(_ item: HistoryItem) {
    guard isStarted, let handle = syncHandle else { return }
    guard let json = serializeItem(item) else { return }
    json.withCString { ptr in _ = maccy_sync_broadcast_item(handle, ptr) }
  }

  func broadcastDeletion(_ syncID: UUID) {
    guard isStarted, let handle = syncHandle else { return }
    syncID.uuidString.withCString { ptr in _ = maccy_sync_broadcast_deletion(handle, ptr) }
  }

  func broadcastUpdate(_ item: HistoryItem) {
    guard isStarted, let handle = syncHandle else { return }
    guard let json = serializeItem(item) else { return }
    json.withCString { ptr in _ = maccy_sync_broadcast_update(handle, ptr) }
  }

  func requestPairing(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in _ = maccy_sync_request_pairing(handle, ptr) }
  }

  func acceptPairing(peerID: String, pin: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { pid in pin.withCString { p in _ = maccy_sync_accept_pairing(handle, pid, p) } }
  }

  func rejectPairing(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in _ = maccy_sync_reject_pairing(handle, ptr) }
  }

  func unpair(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in _ = maccy_sync_unpair(handle, ptr) }
  }

  // ── Event handling — parse JSON from Rust ───────────────────────

  private func handleEvent(_ json: String) {
    guard let data = json.data(using: .utf8),
          let evt = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let type = evt["type"] as? String else { return }

    switch type {
    case "peer_discovered":
      if let peer = evt["peer"] as? [String: Any],
         let peerID = peer["peer_id"] as? String,
         let name = peer["display_name"] as? String,
         !name.isEmpty {
        peerDisplayNames[peerID] = name
        let connected = peer["is_connected"] as? Bool ?? false
        if connected {
          NotificationCenter.default.post(name: NSNotification.Name("showSyncSettings"), object: nil)
          Notifier.notify(body: "\(name) connected", sound: .knock)
        }
        NotificationCenter.default.post(name: .syncPeerDiscovered, object: nil, userInfo: peer)
      }
    case "peer_lost":
      if let peerID = evt["peer_id"] as? String {
        NotificationCenter.default.post(name: .syncPeerLost, object: nil, userInfo: ["peerID": peerID])
      }
    case "pairing_request":
      NotificationCenter.default.post(name: .syncPairingRequest, object: nil, userInfo: [
        "peerID": evt["peer_id"] as? String ?? "",
        "displayName": evt["display_name"] as? String ?? "",
        "pin": evt["pin"] as? String ?? "",
      ])
    case "pairing_complete":
      let peerID = evt["peer_id"] as? String ?? ""
      let success = evt["success"] as? Bool ?? false
      if success, !peerID.isEmpty {
        let displayName = peerDisplayNames[peerID] ?? peerID
        let newDevice = PairedDeviceInfo(
          peerID: peerID,
          nickname: displayName,
          icon: "💻",
          connectedAt: Date(),
          isConnected: true
        )
        var devices = PairedDeviceInfo.all
        if let idx = devices.firstIndex(where: { $0.peerID == peerID }) {
          devices[idx].nickname = displayName
          devices[idx].isConnected = true
        } else {
          devices.append(newDevice)
        }
        PairedDeviceInfo.all = devices
      }
      NotificationCenter.default.post(name: .syncPairingComplete, object: nil, userInfo: [
        "peerID": peerID,
        "success": success,
      ])
    case "item_received":
      if let itemJSON = evt["item_json"] as? String {
        NotificationCenter.default.post(name: .syncItemReceived, object: nil, userInfo: ["itemJSON": itemJSON])
      }
    case "item_deleted":
      if let itemID = evt["item_id"] as? String {
        NotificationCenter.default.post(name: .syncItemDeleted, object: nil, userInfo: ["itemID": itemID])
      }
    case "item_updated":
      if let itemJSON = evt["item_json"] as? String {
        NotificationCenter.default.post(name: .syncItemUpdated, object: nil, userInfo: ["itemJSON": itemJSON])
      }
    case "error":
      let msg = evt["message"] as? String ?? "Unknown error"
      NSLog("[Sync] error: \(msg)")
      NotificationCenter.default.post(name: .syncError, object: nil, userInfo: [
        "code": evt["code"] as? Int ?? 0,
        "message": msg,
      ])
    case "listening":
      NSLog("[Sync] listening on \(evt["address"] as? String ?? "?")")
    default:
      break
    }
  }

  // ── Serialization (temporary — will move to Rust later) ─────────

  private func serializeItem(_ item: HistoryItem) -> String? {
    let contents = item.contents.map { content in
      SyncItemContent(type: content.type, value: content.value?.base64EncodedString())
    }
    let syncItem = SyncItem(
      id: item.syncID.uuidString,
      application: item.application,
      firstCopiedAt: ISO8601DateFormatter().string(from: item.firstCopiedAt),
      lastCopiedAt: ISO8601DateFormatter().string(from: item.lastCopiedAt),
      numberOfCopies: item.numberOfCopies,
      pin: item.pin,
      title: item.title,
      contents: contents,
      syncTimestamp: ISO8601DateFormatter().string(from: item.syncTimestamp),
      syncSource: Defaults[.syncDeviceID]
    )
    return try? String(data: JSONEncoder().encode(syncItem), encoding: .utf8)
  }
}

extension Notification.Name {
  static let syncPeerDiscovered = Notification.Name("syncPeerDiscovered")
  static let syncPeerLost = Notification.Name("syncPeerLost")
  static let syncPairingRequest = Notification.Name("syncPairingRequest")
  static let syncPairingComplete = Notification.Name("syncPairingComplete")
  static let syncItemReceived = Notification.Name("syncItemReceived")
  static let syncItemDeleted = Notification.Name("syncItemDeleted")
  static let syncItemUpdated = Notification.Name("syncItemUpdated")
  static let syncError = Notification.Name("syncError")
}
