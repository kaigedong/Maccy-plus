import SwiftUI
import Defaults

struct SyncSettingsPane: View {
  @Default(.syncEnabled) private var syncEnabled
  @Default(.syncDeviceName) private var syncDeviceName
  @State private var discoveredPeers: [DiscoveredPeer] = []
  @State private var pairedDevices: [PairedDeviceInfo] = PairedDeviceInfo.all
  @State private var editingDevice: PairedDeviceInfo?
  @State private var editingNickname = ""
  @State private var editingIcon = "💻"
  @State private var showPairingDialog = false
  @State private var pairingPeerID = ""
  @State private var pairingDisplayName = ""
  @State private var pairingPin = ""

  var body: some View {
    Settings.Container(contentWidth: 450) {
      Settings.Section(label: { Text("Enable") }) {
        Toggle(isOn: $syncEnabled) {
          Text("Enable Clipboard Sync")
        }
        .onChange(of: syncEnabled) { _, newValue in
          if newValue { SyncBridge.shared.start() } else { SyncBridge.shared.stop() }
        }
      }

      Settings.Section(label: { Text("This Device") }) {
        TextField("Device Name", text: $syncDeviceName)
          .frame(width: 200)
      }

      Settings.Section(
        bottomDivider: true,
        label: { Text("Discovered Devices") }
      ) {
        discoveredDevicesContent
      }

      Settings.Section(label: { Text("Paired Devices") }) {
        pairedDevicesContent
      }
    }
    .sheet(item: $editingDevice) { device in editDeviceSheet(device) }
    .sheet(isPresented: $showPairingDialog) { pairingDialogContent }
    .onReceive(NotificationCenter.default.publisher(for: .syncPeerDiscovered)) { n in
      handlePeerDiscovered(n)
    }
    .onReceive(NotificationCenter.default.publisher(for: .syncPeerLost)) { n in
      handlePeerLost(n)
    }
    .onReceive(NotificationCenter.default.publisher(for: .syncPairingRequest)) { n in
      handlePairingRequest(n)
    }
    .onReceive(NotificationCenter.default.publisher(for: .syncPairingComplete)) { _ in
      pairedDevices = PairedDeviceInfo.all
    }
  }

  @ViewBuilder
  private var discoveredDevicesContent: some View {
    if discoveredPeers.isEmpty {
      Text("Scanning for devices...")
        .foregroundStyle(.secondary)
        .controlSize(.small)
    } else {
      ForEach(discoveredPeers) { peer in
        HStack {
          Text(peer.displayName)
          Spacer()
          Button("Pair") { SyncBridge.shared.requestPairing(peerID: peer.peerID) }
            .buttonStyle(.bordered)
            .controlSize(.small)
        }
      }
    }

    HStack {
      TextField("IP:Port", text: .constant(""))
        .frame(width: 150)
      Button("Connect") {}
        .buttonStyle(.bordered)
        .controlSize(.small)
    }
  }

  @ViewBuilder
  private var pairedDevicesContent: some View {
    if pairedDevices.isEmpty {
      Text("No paired devices")
        .foregroundStyle(.secondary)
        .controlSize(.small)
    } else {
      ForEach(pairedDevices) { device in
        pairedDeviceRow(device)
      }
    }
  }

  private func pairedDeviceRow(_ device: PairedDeviceInfo) -> some View {
    HStack {
      Text(device.icon)
      VStack(alignment: .leading) {
        Text(device.nickname).lineLimit(1)
        HStack(spacing: 4) {
          Circle()
            .fill(device.isConnected ? Color.green : Color.gray)
            .frame(width: 6, height: 6)
          Text(device.isConnected ? "Connected" : "Offline")
            .font(.caption2)
            .foregroundStyle(.secondary)
        }
      }
      Spacer()
      Button { editingDevice = device; editingNickname = device.nickname; editingIcon = device.icon }
        label: { Image(systemName: "pencil") }
        .buttonStyle(.borderless)
        .controlSize(.small)
      Button("Unpair") {
        SyncBridge.shared.unpair(peerID: device.peerID)
        pairedDevices.removeAll { $0.peerID == device.peerID }
        PairedDeviceInfo.all = pairedDevices
      }
      .buttonStyle(.bordered)
      .controlSize(.small)
    }
  }

  private func editDeviceSheet(_ device: PairedDeviceInfo) -> some View {
    VStack(spacing: 16) {
      Text("Edit Device").font(.headline)
      HStack {
        Text("Icon:")
        TextField("", text: $editingIcon).frame(width: 40).font(.title2)
      }
      HStack {
        Text("Name:")
        TextField("Nickname", text: $editingNickname).frame(width: 200)
      }
      HStack {
        Button("Cancel") { editingDevice = nil }.keyboardShortcut(.cancelAction)
        Button("Save") {
          if let idx = pairedDevices.firstIndex(where: { $0.peerID == device.peerID }) {
            pairedDevices[idx].nickname = editingNickname
            pairedDevices[idx].icon = editingIcon
            PairedDeviceInfo.all = pairedDevices
          }
          editingDevice = nil
        }
        .keyboardShortcut(.defaultAction)
      }
    }
    .padding(24)
    .frame(width: 320)
  }

  private var pairingDialogContent: some View {
    VStack(spacing: 16) {
      Text("Pairing Request").font(.headline)
      Text("Device \"\(pairingDisplayName)\" wants to sync clipboards.")
      Text("Confirm this PIN on both devices:")
      HStack(spacing: 8) {
        ForEach(Array(pairingPin.enumerated()), id: \.offset) { _, char in
          Text(String(char))
            .font(.system(.title, design: .monospaced))
            .frame(width: 32, height: 40)
            .background(Color.primary.opacity(0.05))
            .clipShape(RoundedRectangle(cornerRadius: 4))
        }
      }
      HStack {
        Button("Cancel") {
          SyncBridge.shared.rejectPairing(peerID: pairingPeerID)
          showPairingDialog = false
        }
        .keyboardShortcut(.cancelAction)
        Button("Confirm") {
          SyncBridge.shared.acceptPairing(peerID: pairingPeerID, pin: pairingPin)
          showPairingDialog = false
        }
        .keyboardShortcut(.defaultAction)
      }
    }
    .padding(24)
    .frame(width: 340)
  }

  private func handlePeerDiscovered(_ notification: NotificationCenter.Publisher.Output) {
    guard let peerID = notification.userInfo?["peerID"] as? String,
          let name = notification.userInfo?["displayName"] as? String else { return }
    if !discoveredPeers.contains(where: { $0.peerID == peerID }) {
      discoveredPeers.append(DiscoveredPeer(peerID: peerID, displayName: name))
    }
  }

  private func handlePeerLost(_ notification: NotificationCenter.Publisher.Output) {
    guard let peerID = notification.userInfo?["peerID"] as? String else { return }
    discoveredPeers.removeAll { $0.peerID == peerID }
  }

  private func handlePairingRequest(_ notification: NotificationCenter.Publisher.Output) {
    guard let peerID = notification.userInfo?["peerID"] as? String,
          let name = notification.userInfo?["displayName"] as? String,
          let pin = notification.userInfo?["pin"] as? String else { return }
    pairingPeerID = peerID
    pairingDisplayName = name
    pairingPin = pin
    showPairingDialog = true
  }
}

private struct DiscoveredPeer: Identifiable {
  let id = UUID()
  let peerID: String
  let displayName: String
}

#Preview {
  SyncSettingsPane()
    .environment(\.locale, .init(identifier: "en"))
}
