import Defaults
import Sparkle

@Observable
class SoftwareUpdater {
  var automaticallyChecksForUpdates = false {
    didSet {
      updater.automaticallyChecksForUpdates = automaticallyChecksForUpdates
    }
  }

  private var updater: SPUUpdater
  private var automaticallyChecksForUpdatesObservation: NSKeyValueObservation?

  private lazy var updaterController = SPUStandardUpdaterController(
    startingUpdater: true,
    updaterDelegate: self,
    userDriverDelegate: nil
  )

  init() {
    updater = updaterController.updater
    automaticallyChecksForUpdatesObservation = updater.observe(
      \.automaticallyChecksForUpdates,
      options: [.initial, .new, .old]
    ) { [unowned self] updater, change in
      guard change.newValue != change.oldValue else {
        return
      }

      self.automaticallyChecksForUpdates = updater.automaticallyChecksForUpdates
    }
  }

  func checkForUpdates() {
    updater.checkForUpdates()
  }
}

extension SoftwareUpdater: SPUUpdaterDelegate {
  func feedURLString(for updater: SPUUpdater) -> String? {
    if Defaults[.betaUpdates] {
      return "https://github.com/kaigedong/Maccy-plus/releases/download/latest-beta/appcast-beta.xml"
    }
    // Return nil to use the default SUFeedURL from Info.plist
    return nil
  }
}
