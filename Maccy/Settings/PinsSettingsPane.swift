import SwiftUI
import Defaults
import MaccyCore

struct PinsSettingsPane: View {
  @Environment(AppState.self) private var appState

  @State private var availablePins: [String] = []
  @State private var selection: String?

  private var pinnedItems: [HistoryItemDecorator] {
    appState.history.pinnedItems
  }

  var body: some View {
    VStack(alignment: .leading) {
      Table(pinnedItems, selection: $selection) {
        TableColumn(Text("Key", tableName: "PinsSettings")) { decorator in
          if let pin = decorator.item.pin {
            Text(pin)
          }
        }
        .width(60)

        TableColumn(Text("Alias", tableName: "PinsSettings")) { decorator in
          Text(decorator.title)
        }

        TableColumn(Text("Content", tableName: "PinsSettings")) { decorator in
          Text(Clipboard.shared.getText(from: decorator.item) ?? "")
            .lineLimit(1)
            .truncationMode(.tail)
        }
      }
      .onAppear {
        availablePins = History.availablePins
      }
      .onDeleteCommand {
        guard let selection else { return }
        if let item = appState.history.items.first(where: { $0.item.id == selection }) {
          appState.history.delete(item)
        }
      }

      Text("PinCustomizationDescription", tableName: "PinsSettings")
        .foregroundStyle(.gray)
        .controlSize(.small)
    }
    .frame(minWidth: 500, minHeight: 400)
    .padding()
  }
}

#Preview {
  PinsSettingsPane()
    .environment(\.locale, .init(identifier: "en"))
}
