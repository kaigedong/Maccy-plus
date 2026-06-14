import Foundation

struct PairedDeviceInfo: Codable, Identifiable, Equatable {
    var id: String {
        peerID
    }

    let peerID: String
    var nickname: String
    var icon: String
    var connectedAt: Date
    var isConnected: Bool
    var isAdmin: Bool

    /// Load paired devices from Rust core (SQLite), not UserDefaults.
    static func loadFromCore() -> [PairedDeviceInfo] {
        guard let jsonList = try? AppState.shared.history.core.getPairedPeers() else { return [] }
        return jsonList.compactMap { json in
            guard let data = json.data(using: .utf8),
                  let dict = try? JSONSerialization.jsonObject(with: data) as? [String: String],
                  let peerId = dict["peerId"],
                  let name = dict["displayName"] else { return nil }
            return PairedDeviceInfo(
                peerID: peerId,
                nickname: name,
                icon: "💻",
                connectedAt: Date(),
                isConnected: dict["isOnline"] == "true",
                isAdmin: dict["isAdmin"] == "true"
            )
        }
    }
}

struct SyncItemContent: Codable {
    let type: String
    let value: String?
}

struct SyncItem: Codable {
    let id: String
    let application: String?
    let firstCopiedAt: String
    let lastCopiedAt: String
    let numberOfCopies: Int
    let pin: String?
    let title: String
    let contents: [SyncItemContent]
    let syncTimestamp: String
    let syncSource: String
}
