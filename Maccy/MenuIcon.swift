import AppKit
import Defaults

enum MenuIcon: String, CaseIterable, Identifiable, Defaults.Serializable {
    case maccy
    case clipboard
    case scissors
    case paperclip

    var id: Self {
        self
    }

    var image: NSImage {
        switch self {
        case .maccy:
            NSImage(named: .maccyStatusBar)!
        case .clipboard:
            NSImage(named: .clipboard)!
        case .scissors:
            NSImage(named: .scissors)!
        case .paperclip:
            NSImage(named: .paperclip)!
        }
    }
}
