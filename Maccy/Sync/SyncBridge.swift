import AppKit
import Defaults
import Foundation

/// macOS implementation of the `ClipboardObserver` protocol from Rust.
/// Receives sync events and routes them to the UI layer.
///
/// Architecture:
///   HistoryManager (Rust) → SyncEngine → ClipboardObserver (this class)
///       → History.shared.add() / NotificationCenter → SwiftUI
class MaccySyncObserver: ClipboardObserver {
  func onItemReceived(item: ClipboardItem) {
    Task { @MainActor in
      _ = History.shared.add(item, shouldAppend: false)
      History.shared.syncBroadcastToPeers = false // don't echo back
    }
  }

  func onItemDeleted(itemId: String) {
    Task { @MainActor in
      History.shared.deleteBySyncID(itemId)
    }
  }

  func onItemUpdated(item: ClipboardItem) {
    Task { @MainActor in
      History.shared.updateBySyncID(item)
    }
  }

  func onPeerDiscovered(peerId: String, displayName: String, addresses: [String], isConnected: Bool) {
    DispatchQueue.main.async {
      if isConnected {
        Notifier.notify(body: "\(displayName) connected", sound: .knock)
        NotificationCenter.default.post(name: NSNotification.Name("showSyncSettings"), object: nil)
      }
      NotificationCenter.default.post(name: .syncPeerDiscovered, object: nil, userInfo: [
        "peerID": peerId,
        "displayName": displayName,
        "addresses": addresses,
        "isConnected": isConnected,
      ])
    }
  }

  func onPeerLost(peerId: String) {
    DispatchQueue.main.async {
      NotificationCenter.default.post(name: .syncPeerLost, object: nil, userInfo: [
        "peerID": peerId,
      ])
    }
  }

  func onPairingRequest(peerId: String, displayName: String, pin: String) {
    DispatchQueue.main.async {
      NotificationCenter.default.post(name: .syncPairingRequest, object: nil, userInfo: [
        "peerID": peerId,
        "displayName": displayName,
        "pin": pin,
      ])
    }
  }

  func onPairingComplete(peerId: String, success: Bool) {
    DispatchQueue.main.async {
      if success {
        let displayName = SyncBridge.shared.peerDisplayNames[peerId] ?? peerId
        let newDevice = PairedDeviceInfo(
          peerID: peerId,
          nickname: displayName,
          icon: "💻",
          connectedAt: Date(),
          isConnected: true
        )
        var devices = PairedDeviceInfo.all
        if let idx = devices.firstIndex(where: { $0.peerID == peerId }) {
          devices[idx].nickname = displayName
          devices[idx].isConnected = true
        } else {
          devices.append(newDevice)
        }
        PairedDeviceInfo.all = devices
      }
      NotificationCenter.default.post(name: .syncPairingComplete, object: nil, userInfo: [
        "peerID": peerId,
        "success": success,
      ])
    }
  }

  func onListening(address: String) {
    NSLog("[Sync] listening on \(address)")
  }

  func onError(code: Int32, message: String) {
    NSLog("[Sync] error (\(code)): \(message)")
    DispatchQueue.main.async {
      NotificationCenter.default.post(name: .syncError, object: nil, userInfo: [
        "code": code,
        "message": message,
      ])
    }
  }
}

@MainActor
class SyncBridge {
  static let shared = SyncBridge()

  private(set) var peerDisplayNames: [String: String] = [:]
  private var isStarted = false

  private init() {}

  var isEnabled: Bool { Defaults[.syncEnabled] }

  func start() {
    guard !isStarted else { return }
    guard isEnabled else { return }

    let deviceName = Defaults[.syncDeviceName]
    let deviceID = Defaults[.syncDeviceID]
    NSLog("[Sync] start: name=\(deviceName)")

    let observer = MaccySyncObserver()
    do {
      try AppState.shared.history.core.startSync(
        deviceName: deviceName,
        deviceId: deviceID,
        observer: observer
      )
    } catch {
      NSLog("[Sync] start failed: \(error)")
      return
    }

    isStarted = true
    NSLog("[Sync] started (via HistoryManager)")
  }

  func stop() {
    guard isStarted else { return }
    do {
      try AppState.shared.history.core.stopSync()
    } catch {
      NSLog("[Sync] stop failed: \(error)")
    }
    isStarted = false
  }

  // ── Peer discovery events (cache names) ────────────────────────

  func recordPeerName(_ peerID: String, _ name: String) {
    peerDisplayNames[peerID] = name
  }

  // ── Thin wrappers that delegate to HistoryManager ──────────────

  func addPeerAddress(_ address: String) {
    AppState.shared.history.core.syncAddPeerAddress(address: address)
  }

  func requestPairing(peerID: String) {
    AppState.shared.history.core.syncRequestPairing(peerId: peerID)
  }

  func acceptPairing(peerID: String, pin: String) {
    AppState.shared.history.core.syncAcceptPairing(peerId: peerID, pin: pin)
  }

  func rejectPairing(peerID: String) {
    AppState.shared.history.core.syncRejectPairing(peerId: peerID)
  }

  func unpair(peerID: String) {
    AppState.shared.history.core.syncUnpair(peerId: peerID)
  }

  func broadcastNewItem(_ item: ClipboardItem) {
    AppState.shared.history.core.syncBroadcastItem(item: item)
  }

  func broadcastDeletion(_ syncID: UUID) {
    AppState.shared.history.core.syncBroadcastDeletion(itemId: syncID.uuidString)
  }

  func broadcastUpdate(_ item: ClipboardItem) {
    AppState.shared.history.core.syncBroadcastUpdate(item: item)
  }

  func refreshDiscovery() {
    AppState.shared.history.core.syncStopDiscovery()
    AppState.shared.history.core.syncStartDiscovery()
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
