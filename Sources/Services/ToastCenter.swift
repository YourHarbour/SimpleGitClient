import SwiftUI
import Observation

enum ToastStyle: Equatable {
    case success, error, info

    var icon: String {
        switch self {
        case .success: return "checkmark.circle.fill"
        case .error: return "exclamationmark.triangle.fill"
        case .info: return "info.circle.fill"
        }
    }
    var color: Color {
        switch self {
        case .success: return Theme.accentGreen
        case .error: return Theme.accentRed
        case .info: return Theme.accentBlue
        }
    }
}

struct Toast: Identifiable {
    let id = UUID()
    let message: String
    let style: ToastStyle
}

/// 左下角操作结果提示中心。任意线程调用 show(),内部在主线程更新并自动消失。
@Observable
final class ToastCenter {
    static let shared = ToastCenter()
    private(set) var toasts: [Toast] = []

    func show(_ message: String, style: ToastStyle = .info) {
        DispatchQueue.main.async {
            let toast = Toast(message: message, style: style)
            self.toasts.append(toast)
            if self.toasts.count > 4 { self.toasts.removeFirst(self.toasts.count - 4) }
            let duration: Double = style == .error ? 4.0 : 2.6
            DispatchQueue.main.asyncAfter(deadline: .now() + duration) {
                self.toasts.removeAll { $0.id == toast.id }
            }
        }
    }

    func dismiss(_ id: UUID) {
        toasts.removeAll { $0.id == id }
    }
}
