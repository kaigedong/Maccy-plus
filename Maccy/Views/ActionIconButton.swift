import SwiftUI

struct ActionIconButton: View {
  let icon: String
  let isDark: Bool
  let tooltip: String
  let action: () -> Void

  @State private var isHovered = false

  var body: some View {
    Image(systemName: icon)
      .font(.system(size: 11))
      .frame(width: 22, height: 22)
      .contentShape(Rectangle())
      .background(
        RoundedRectangle(cornerRadius: 4)
          .fill(backgroundColor)
      )
      .foregroundStyle(foregroundColor)
      .onHover { hovering in
        isHovered = hovering
      }
      .onTapGesture {
        action()
      }
      .help(tooltip)
  }

  private var backgroundColor: Color {
    guard isHovered else { return .clear }
    return isDark ? .white.opacity(0.25) : .black.opacity(0.08)
  }

  private var foregroundColor: Color {
    if isHovered {
      return isDark ? .white : .primary
    }
    return isDark ? .white.opacity(0.7) : .secondary
  }
}
