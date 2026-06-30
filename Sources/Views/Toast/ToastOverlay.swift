import SwiftUI

/// 左下角弹出的操作结果提示。
struct ToastOverlay: View {
    @Environment(ToastCenter.self) private var center

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
                .font(.system(size: 14))
                .foregroundStyle(toast.style.color)
            Text(toast.message)
                .font(.system(size: 13))
                .foregroundStyle(Theme.textPrimary)
                .lineLimit(2)
            if hover {
                Button(action: onClose) {
                    Image(systemName: "xmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(Theme.textMuted)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .background(
            RoundedRectangle(cornerRadius: 8)
                .fill(Theme.bgElevated)
                .shadow(color: .black.opacity(0.35), radius: 10, x: 0, y: 3)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(toast.style.color.opacity(0.35), lineWidth: 1)
        )
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: 2)
                .fill(toast.style.color)
                .frame(width: 3)
                .padding(.vertical, 6)
        }
        .onHover { hover = $0 }
    }
}
