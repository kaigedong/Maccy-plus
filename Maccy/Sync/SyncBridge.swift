import AppKit
import Defaults
import Foundation

@MainActor
class SyncBridge {
  static let shared = SyncBridge()

  private var syncHandle: OpaquePointer?
  private var isStarted = false

  private init() {}

  var isEnabled: Bool { Defaults[.syncEnabled] }

  func start() {
    guard !isStarted else { return }
    guard isEnabled else { return }

    let deviceName = Defaults[.syncDeviceName]
    let deviceID = Defaults[.syncDeviceID]

    guard let handle = maccy_sync_create(deviceName, deviceID) else { return }
    syncHandle = handle

    registerCallbacks()

    let result = maccy_sync_start(handle)
    if result != MACCY_SYNC_OK {
      maccy_sync_destroy(handle)
      syncHandle = nil
      return
    }

    _ = maccy_sync_start_discovery(handle)
    isStarted = true
  }

  func stop() {
    guard isStarted, let handle = syncHandle else { return }
    _ = maccy_sync_stop_discovery(handle)
    _ = maccy_sync_stop(handle)
    maccy_sync_destroy(handle)
    syncHandle = nil
    isStarted = false
  }

  func broadcastNewItem(_ item: HistoryItem) {
    guard isStarted, let handle = syncHandle else { return }
    guard let json = serializeItem(item) else { return }
    json.withCString { ptr in
      _ = maccy_sync_broadcast_item(handle, ptr)
    }
  }

  func broadcastDeletion(_ syncID: UUID) {
    guard isStarted, let handle = syncHandle else { return }
    syncID.uuidString.withCString { ptr in
      _ = maccy_sync_broadcast_deletion(handle, ptr)
    }
  }

  func broadcastUpdate(_ item: HistoryItem) {
    guard isStarted, let handle = syncHandle else { return }
    guard let json = serializeItem(item) else { return }
    json.withCString { ptr in
      _ = maccy_sync_broadcast_update(handle, ptr)
    }
  }

  func requestPairing(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in
      _ = maccy_sync_request_pairing(handle, ptr)
    }
  }

  func acceptPairing(peerID: String, pin: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { pid in
      pin.withCString { p in
        _ = maccy_sync_accept_pairing(handle, pid, p)
      }
    }
  }

  func rejectPairing(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in
      _ = maccy_sync_reject_pairing(handle, ptr)
    }
  }

  func unpair(peerID: String) {
    guard isStarted, let handle = syncHandle else { return }
    peerID.withCString { ptr in
      _ = maccy_sync_unpair(handle, ptr)
    }
  }

  func addPeerAddress(address: String) {
    guard isStarted, let handle = syncHandle else { return }
    let parts = address.split(separator: ":")
    guard parts.count >= 2, let _ = UInt16(parts.last ?? "") else { return }
    let host = parts.dropLast().joined(separator: ":")
    let port = parts.last! 
    let multiaddr = "/ip4/\(host)/tcp/\(port)"
    multiaddr.withCString { addrPtr in
      _ = maccy_sync_add_peer_address(handle, "", addrPtr)
    }
  }

  func getPairedPeersJSON() -> String {
    guard isStarted, let handle = syncHandle else { return "[]" }
    guard let cStr = maccy_sync_get_paired_peers(handle) else { return "[]" }
    defer { maccy_sync_free_string(UnsafeMutablePointer(mutating: cStr)) }
    return String(cString: cStr)
  }

  private func registerCallbacks() {
    guard let handle = syncHandle else { return }

    maccy_sync_on_peer_discovered(handle) { peerID, displayName, addresses in
      DispatchQueue.main.async {
        let pid = String(cString: peerID!)
        let name = String(cString: displayName!)
        let addrs = String(cString: addresses!)
        NotificationCenter.default.post(
          name: .syncPeerDiscovered,
          object: nil,
          userInfo: ["peerID": pid, "displayName": name, "addresses": addrs]
        )
      }
    }

    maccy_sync_on_peer_lost(handle) { peerID in
      DispatchQueue.main.async {
        let pid = String(cString: peerID!)
        NotificationCenter.default.post(
          name: .syncPeerLost,
          object: nil,
          userInfo: ["peerID": pid]
        )
      }
    }

    maccy_sync_on_pairing_request(handle) { peerID, displayName, pin in
      DispatchQueue.main.async {
        let pid = String(cString: peerID!)
        let name = String(cString: displayName!)
        let p = String(cString: pin!)
        NotificationCenter.default.post(
          name: .syncPairingRequest,
          object: nil,
          userInfo: ["peerID": pid, "displayName": name, "pin": p]
        )
      }
    }

    maccy_sync_on_pairing_complete(handle) { peerID, success in
      DispatchQueue.main.async {
        let pid = String(cString: peerID!)
        NotificationCenter.default.post(
          name: .syncPairingComplete,
          object: nil,
          userInfo: ["peerID": pid, "success": success]
        )
      }
    }

    maccy_sync_on_sync_item_received(handle) { itemJSON in
      DispatchQueue.main.async {
        let json = String(cString: itemJSON!)
        NotificationCenter.default.post(
          name: .syncItemReceived,
          object: nil,
          userInfo: ["itemJSON": json]
        )
      }
    }

    maccy_sync_on_sync_item_deleted(handle) { itemID in
      DispatchQueue.main.async {
        let id = String(cString: itemID!)
        NotificationCenter.default.post(
          name: .syncItemDeleted,
          object: nil,
          userInfo: ["itemID": id]
        )
      }
    }

    maccy_sync_on_sync_item_updated(handle) { itemJSON in
      DispatchQueue.main.async {
        let json = String(cString: itemJSON!)
        NotificationCenter.default.post(
          name: .syncItemUpdated,
          object: nil,
          userInfo: ["itemJSON": json]
        )
      }
    }
  }

  private func serializeItem(_ item: HistoryItem) -> String? {
    let contents = item.contents.map { content in
      SyncItemContent(
        type: content.type,
        value: content.value?.base64EncodedString()
      )
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

    let encoder = JSONEncoder()
    return try? String(data: encoder.encode(syncItem), encoding: .utf8)
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
}
