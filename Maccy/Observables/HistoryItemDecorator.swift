import AppKit.NSWorkspace
import Defaults
import Foundation
import Observation
import Sauce
import MaccyCore

@Observable
class HistoryItemDecorator: Identifiable, Hashable, HasVisibility {
  static func == (lhs: HistoryItemDecorator, rhs: HistoryItemDecorator) -> Bool {
    return lhs.id == rhs.id
  }

  static var previewImageSize: NSSize { NSScreen.forPopup?.visibleFrame.size ?? NSSize(width: 2048, height: 1536) }
  static var thumbnailImageSize: NSSize { NSSize(width: 340, height: Defaults[.imageMaxHeight]) }

  let id = UUID()

  var title: String = ""
  var attributedTitle: AttributedString?

  var isVisible: Bool = true
  var selectionIndex: Int = -1
  var isSelected: Bool {
    return selectionIndex != -1
  }
  var shortcuts: [KeyShortcut] = []
  var isEditing: Bool = false
  var editingText: String = ""

  var application: String? {
    let universalClipboardTypes = ["com.apple.UIKit.pboardName"]
    let hasUniversal = item.contents.contains(where: { universalClipboardTypes.contains($0.contentType) })
    if hasUniversal {
      return "iCloud"
    }

    guard let bundle = item.application,
      let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle)
    else {
      return nil
    }

    return url.deletingPathExtension().lastPathComponent
  }

  var hasImage: Bool { imageData != nil }

  var previewImageGenerationTask: Task<(), Error>?
  var thumbnailImageGenerationTask: Task<(), Error>?
  var previewImage: NSImage?
  var thumbnailImage: NSImage?
  var applicationImage: ApplicationImage

  var text: String { Clipboard.shared.getPreviewableText(from: item).shortened(to: 10_000) }

  var isPinned: Bool { item.pin != nil }
  var isUnpinned: Bool { item.pin == nil }

  func hash(into hasher: inout Hasher) {
    hasher.combine(id)
    hasher.combine(title)
    hasher.combine(attributedTitle)
  }

  var item: ClipboardItem

  // Computed AppKit properties derived from ClipboardItem contents
  var imageData: Data? {
    let imageTypes = [NSPasteboard.PasteboardType.tiff, .png, .jpeg, .heic].map(\.rawValue)
    guard let content = item.contents.first(where: { imageTypes.contains($0.contentType) }),
          let value = content.value else { return nil }
    return Data(value)
  }

  var image: NSImage? {
    guard let data = imageData else { return nil }
    return NSImage(data: data)
  }

  init(_ item: ClipboardItem, shortcuts: [KeyShortcut] = []) {
    self.item = item
    self.shortcuts = shortcuts
    self.title = item.title
    self.applicationImage = ApplicationImageCache.shared.getImage(application: item.application)
  }

  @MainActor
  func ensureThumbnailImage() {
    guard image != nil else { return }
    guard thumbnailImage == nil else { return }
    guard thumbnailImageGenerationTask == nil else { return }
    thumbnailImageGenerationTask = Task { [weak self] in
      self?.generateThumbnailImage()
    }
  }

  @MainActor
  func ensurePreviewImage() {
    guard image != nil else { return }
    guard previewImage == nil else { return }
    guard previewImageGenerationTask == nil else { return }
    previewImageGenerationTask = Task { [weak self] in
      self?.generatePreviewImage()
    }
  }

  @MainActor
  func asyncGetPreviewImage() async -> NSImage? {
    if let image = previewImage {
      return image
    }
    ensurePreviewImage()
    _ = await previewImageGenerationTask?.result
    return previewImage
  }

  @MainActor
  func cleanupImages() {
    thumbnailImageGenerationTask?.cancel()
    previewImageGenerationTask?.cancel()
    thumbnailImage?.recache()
    previewImage?.recache()
    thumbnailImage = nil
    previewImage = nil
  }

  @MainActor
  private func generateThumbnailImage() {
    guard let image else { return }
    thumbnailImage = image.resized(to: HistoryItemDecorator.thumbnailImageSize)
  }

  @MainActor
  private func generatePreviewImage() {
    guard let image else { return }
    previewImage = image.resized(to: HistoryItemDecorator.previewImageSize)
  }

  @MainActor
  func sizeImages() {
    generatePreviewImage()
    generateThumbnailImage()
  }

  func highlight(_ query: String, _ ranges: [MatchRange]) {
    guard !query.isEmpty, !title.isEmpty else {
      attributedTitle = nil
      return
    }

    var attributedString = AttributedString(title.shortened(to: 500))
    for range in ranges {
      let lower = attributedString.index(attributedString.startIndex, offsetByCharacters: Int(range.start))
      let upper = attributedString.index(attributedString.startIndex, offsetByCharacters: Int(range.end))
      if lower < upper && upper <= attributedString.endIndex {
        switch Defaults[.highlightMatch] {
        case .bold:
          attributedString[lower..<upper].font = .bold(.body)()
        case .italic:
          attributedString[lower..<upper].font = .italic(.body)()
        case .underline:
          attributedString[lower..<upper].underlineStyle = .single
        default:
          attributedString[lower..<upper].backgroundColor = .findHighlightColor
          attributedString[lower..<upper].foregroundColor = .black
        }
      }
    }

    attributedTitle = attributedString
  }
}
