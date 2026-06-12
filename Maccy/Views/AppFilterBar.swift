import SwiftUI
import Defaults

struct AppFilterBar: View {
  @Environment(AppState.self) private var appState

  private var apps: [(bundleId: String, image: ApplicationImage)] {
    appState.history.sourceApps
  }

  private var pairedDevices: [PairedDeviceInfo] {
    appState.syncDevices
  }

  var body: some View {
    if !apps.isEmpty || !pairedDevices.isEmpty {
      ScrollView(.horizontal, showsIndicators: false) {
        HStack(spacing: 6) {
          ForEach(apps, id: \.bundleId) { app in
            AppFilterIcon(
              appImage: app.image,
              isExcluded: appState.history.excludedApps.contains(app.bundleId),
              isUnknown: app.bundleId.isEmpty
            ) {
              if appState.history.excludedApps.contains(app.bundleId) {
                appState.history.excludedApps.remove(app.bundleId)
              } else {
                appState.history.excludedApps.insert(app.bundleId)
              }
            }
          }

          if !apps.isEmpty && !pairedDevices.isEmpty {
            Rectangle()
              .fill(Color.primary.opacity(0.15))
              .frame(width: 1, height: 16)
          }

          ForEach(pairedDevices) { device in
            DeviceFilterIcon(
              device: device,
              isExcluded: appState.history.excludedDevices.contains(device.peerID)
            ) {
              if appState.history.excludedDevices.contains(device.peerID) {
                appState.history.excludedDevices.remove(device.peerID)
              } else {
                appState.history.excludedDevices.insert(device.peerID)
              }
            }
          }
        }
        .padding(.horizontal, 4)
      }
      .frame(height: 26)
    }
  }
}

struct AppFilterIcon: View {
  let appImage: ApplicationImage
  let isExcluded: Bool
  var isUnknown: Bool = false
  let action: () -> Void

  @State private var isHovered = false

  var body: some View {
    Image(nsImage: isUnknown ? NSImage(systemSymbolName: "app.badge.questionmark", accessibilityDescription: "Unknown")! : appImage.nsImage)
      .resizable()
      .frame(width: 18, height: 18)
      .contentShape(Rectangle())
      .opacity(isExcluded ? 0.3 : 1.0)
      .overlay(
        Group {
          if isExcluded {
            Color.red.opacity(0.3)
          } else if isHovered {
            Color.primary.opacity(0.08)
          }
        }
      )
      .clipShape(RoundedRectangle(cornerRadius: 4))
      .overlay(
        RoundedRectangle(cornerRadius: 4)
          .stroke(isExcluded ? Color.red.opacity(0.5) : .clear, lineWidth: 1)
      )
      .onHover { hovering in
        isHovered = hovering
      }
      .onTapGesture {
        action()
      }
      .help(isUnknown ? "Unknown Source" : (appImage.bundleIdentifier ?? ""))
  }
}
