import SwiftUI

/// 左下角操作结果提示(高强调:整块绿/红/蓝)。
struct ToastOverlay: View {
    private let center = ToastCenter.shared

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(center.toasts) { toast in
                ToastView(toast: toast) { center.dismiss(toast.id) }
                    .transition(.asymmetric(
                        insertion: .move(edge: .leading).combined(with: .opacity),
                        removal: .opacity
                    ))
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
        .animation(.spring(response: 0.35, dampingFraction: 0.82), value: center.toasts.map(\.id))
    }
}

struct ToastView: View {
    let toast: Toast
    var onClose: () -> Void
    @State private var hover = false

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: toast.style.icon)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(.white)
            Text(toast.message)
                .font(.system(size: 13.5, weight: .semibold))
                .foregroundStyle(.white)
                .lineLimit(2)
            if hover {
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(.white.opacity(0.85))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 16).padding(.vertical, 12)
        .background(
            RoundedRectangle(cornerRadius: 9)
                .fill(toast.style.color)
                .shadow(color: toast.style.color.opacity(0.5), radius: 12, x: 0, y: 3)
        )
        .onHover { hover = $0 }
    }
}

/// 右下角进度提示(pull/push/fetch/clone 等进行中)。
struct ActivityOverlay: View {
    private let activity = ActivityCenter.shared

    var body: some View {
        Group {
            if activity.isBusy, let label = activity.current {
                HStack(spacing: 10) {
                    ProgressView().controlSize(.small).tint(Theme.bgApp)
                    Text(label)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(Theme.bgApp)
                }
                .padding(.horizontal, 16).padding(.vertical, 12)
                .background(
                    RoundedRectangle(cornerRadius: 9)
                        .fill(Theme.accentYellow)
                        .shadow(color: Theme.accentYellow.opacity(0.5), radius: 12, x: 0, y: 3)
                )
                .transition(.move(edge: .leading).combined(with: .opacity))
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
        .padding(16)
        .animation(.spring(response: 0.3, dampingFraction: 0.85), value: activity.isBusy)
    }
}
