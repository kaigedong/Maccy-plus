import Carbon
import Sauce

class KeyboardLayout {
    static var current: KeyboardLayout {
        KeyboardLayout()
    }

    /// Dvorak - QWERTY ⌘ (https://github.com/p0deje/Maccy/issues/482)
    /// bépo 1.1 - Azerty ⌘ (https://github.com/p0deje/Maccy/issues/520)
    var commandSwitchesToQWERTY: Bool {
        localizedName.hasSuffix("⌘")
    }

    var localizedName: String {
        if let value = TISGetInputSourceProperty(inputSource, kTISPropertyLocalizedName) {
            Unmanaged<CFString>.fromOpaque(value).takeUnretainedValue() as String
        } else {
            ""
        }
    }

    private var inputSource: TISInputSource!

    init() {
        inputSource = TISCopyCurrentKeyboardLayoutInputSource().takeUnretainedValue()
    }
}
