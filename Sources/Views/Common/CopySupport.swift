import SwiftUI
import AppKit

/// 全 App 统一的「复制到剪贴板」入口:写系统剪贴板 + 一个轻提示。
enum Clipboard {
    static func copy(_ text: String, label: String = "Copied") {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(text, forType: .string)
        ToastCenter.shared.show(label, style: .info)
    }
}

extension View {
    /// 给一个不可交互的文本挂「Copy …」右键菜单。
    /// 注意:行级(可点击)控件请把 Copy 项写进它自己的 .contextMenu,
    /// 否则内层菜单会吃掉外层的行菜单。
    func copyable(_ title: String, _ value: @escaping @autoclosure () -> String) -> some View {
        contextMenu {
            Button("Copy \(title)") { Clipboard.copy(value(), label: "Copied \(title.lowercased())") }
        }
    }
}

// MARK: - Diff → 可复制的 patch 文本

extension DiffLine {
    /// 还原成 unified diff 的一行(带 +/-/空格 前缀)。
    var patchLine: String {
        switch type {
        case .added: return "+" + content
        case .removed: return "-" + content
        case .context: return " " + content
        case .hunkHeader: return content
        }
    }
}

extension DiffHunk {
    var patchText: String {
        ([header] + lines.map(\.patchLine)).joined(separator: "\n") + "\n"
    }
    /// 只要改动块里「改动后」的内容(上下文 + 新增),不带 diff 前缀 —— 便于直接粘进编辑器。
    var plainText: String {
        lines.filter { $0.type == .added || $0.type == .context }
             .map(\.content).joined(separator: "\n") + "\n"
    }
}

extension DiffFile {
    var patchText: String { hunks.map(\.patchText).joined() }
}
