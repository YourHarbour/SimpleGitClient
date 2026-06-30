import SwiftUI
import AppKit

/// 让 SwiftUI 内容铺满到窗口顶部(标题栏区域),使 macOS 红绿灯按钮与标签栏处于同一行,
/// 并在标签栏高度内把红绿灯垂直居中。
struct WindowAccessor: NSViewRepresentable {
    var titleBarHeight: CGFloat

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async { configure(view.window) }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async { configure(nsView.window) }
    }

    private func configure(_ window: NSWindow?) {
        guard let window else { return }
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.styleMask.insert(.fullSizeContentView)
        window.isMovableByWindowBackground = false

        // 红绿灯按钮容器(NSTitlebarView)在非翻转坐标系中。把它整体在 titleBarHeight 内垂直居中:
        // 让按钮中心位于距窗口顶部 titleBarHeight/2 处。
        guard let close = window.standardWindowButton(.closeButton),
              let titlebar = close.superview else { return }
        let buttonHeight = close.frame.height
        // titlebar 坐标:y 向上为正,容器顶部 = titlebar.bounds.height。
        // 期望按钮顶部距窗口顶部 = (titleBarHeight - buttonHeight)/2。
        let desiredTopInset = (titleBarHeight - buttonHeight) / 2
        let newOriginY = titlebar.bounds.height - desiredTopInset - buttonHeight
        for type in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            if let button = window.standardWindowButton(type) {
                button.setFrameOrigin(NSPoint(x: button.frame.origin.x, y: newOriginY))
            }
        }
    }
}
