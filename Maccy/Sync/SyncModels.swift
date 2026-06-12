import Foundation

struct PairedDeviceInfo: Codable, Identifiable, Equatable {
  var id: String { peerID }
  let peerID: String
  var nickname: String
  var icon: String
  var connectedAt: Date
  var isConnected: Bool

  static var all: [PairedDeviceInfo] {
    get {
      guard let data = UserDefaults.standard.data(forKey: "syncPairedDeviceInfos"),
            let infos = try? JSONDecoder().decode([PairedDeviceInfo].self, from: data)
      else { return [] }
      return infos
    }
    set {
      let data = try? JSONEncoder().encode(newValue)
      UserDefaults.standard.set(data, forKey: "syncPairedDeviceInfos")
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
