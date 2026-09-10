import SwiftUI

enum Palette {
    static let notch = Color(hex: NotchColors.notch)
    static let card = Color(hex: NotchColors.card)
    static let ringTrack = Color(hex: NotchColors.ringTrack)
    static let barTrack = Color(hex: NotchColors.barTrack)
    static let ample = Color(hex: NotchColors.ample)
    static let watch = Color(hex: NotchColors.watch)
    static let critical = Color(hex: NotchColors.critical)
    static let textPrimary = Color(hex: NotchColors.textPrimary)
    static let textSecondary = Color(hex: NotchColors.textSecondary)
}

extension Color {
    init(hex: UInt32) {
        self.init(
            .sRGB,
            red:   Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue:  Double(hex & 0xFF) / 255,
            opacity: 1
        )
    }
}

private struct NotchReduceTransparencyKey: EnvironmentKey {
    static let defaultValue: Bool = false
}

extension EnvironmentValues {
    /// True when macOS Accessibility "Reduce Transparency" is enabled in system settings,
    /// or explicitly overridden via `.environment(\.notchReduceTransparency, ...)`.
    var notchReduceTransparency: Bool {
        get { self[NotchReduceTransparencyKey.self] || self.accessibilityReduceTransparency }
        set { self[NotchReduceTransparencyKey.self] = newValue }
    }
}

