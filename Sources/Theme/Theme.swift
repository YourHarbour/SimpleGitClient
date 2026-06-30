import SwiftUI

// MARK: - Color Hex Extension

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 6: // RGB
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (255, 0, 0, 0)
        }
        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}

// MARK: - NSColor Hex Extension

extension NSColor {
    convenience init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let r, g, b: UInt64
        switch hex.count {
        case 6:
            (r, g, b) = (int >> 16, int >> 8 & 0xFF, int & 0xFF)
        default:
            (r, g, b) = (0, 0, 0)
        }
        self.init(
            srgbRed: CGFloat(r) / 255,
            green: CGFloat(g) / 255,
            blue: CGFloat(b) / 255,
            alpha: 1.0
        )
    }
}

// MARK: - Theme

/// GitPilot 主题系统 — 严格按照 gitkraken-clone-ui-spec.md §1 §2 定义
enum Theme {
    // MARK: §1 配色 Token

    /// 应用最底层背景
    static let bgApp = Color(hex: "#16181d")
    /// 各面板背景
    static let bgPanel = Color(hex: "#1b1e24")
    /// 悬浮/选中行背景
    static let bgElevated = Color(hex: "#272a31")
    /// hover 态背景
    static let bgHover = Color(hex: "#22252c")
    /// 分隔线/边框
    static let border = Color(hex: "#2c2f36")
    /// 输入框等较明显边框
    static let borderStrong = Color(hex: "#3a3e46")
    /// 主文字
    static let textPrimary = Color(hex: "#dfe2e7")
    /// 次要/说明文字
    static let textSecondary = Color(hex: "#8b909a")
    /// 更弱的文字(占位符等)
    static let textMuted = Color(hex: "#646a73")
    /// 新增/已暂存/成功(+)
    static let accentGreen = Color(hex: "#4caf50")
    /// 主操作按钮底色(暗绿,禁用态)
    static let accentGreenDeep = Color(hex: "#2f5d3f")
    /// 主提交按钮可用态(亮绿)
    static let commitGreen = Color(hex: "#3c8c52")
    /// 分支标签 main 的色调
    static let accentTeal = Color(hex: "#3fb6a8")
    /// merge 连线 / 蓝色节点
    static let accentBlue = Color(hex: "#4a90e2")
    /// 删除按钮 / removed 行
    static let accentRed = Color(hex: "#c0392b")
    /// 当前激活标签背景
    static let selectedTab = Color(hex: "#23262d")
    /// 选中行背景
    static let selectedRow = Color(hex: "#23262d")
    /// 分支橙色
    static let accentOrange = Color(hex: "#e6994a")
    /// 进度提示黄色
    static let accentYellow = Color(hex: "#e9c54a")
    // MARK: NSColor versions (for NSTextView)

    static let nsBgApp = NSColor(hex: "#16181d")
    static let nsBgPanel = NSColor(hex: "#1b1e24")
    static let nsBgElevated = NSColor(hex: "#272a31")
    static let nsTextPrimary = NSColor(hex: "#dfe2e7")
    static let nsTextSecondary = NSColor(hex: "#8b909a")
    static let nsTextMuted = NSColor(hex: "#646a73")
    static let nsAccentGreen = NSColor(hex: "#4caf50")
    static let nsAccentRed = NSColor(hex: "#c0392b")
    static let nsLineNumber = NSColor(hex: "#5b616a")

    // MARK: Diff 专用色 (spec §1)

    /// 新增行背景(added) — 半透明绿
    static let diffAddedBg = Color(red: 46/255, green: 160/255, blue: 67/255, opacity: 0.15)
    /// 新增行左侧标记栏
    static let diffAddedGutter = Color(red: 46/255, green: 160/255, blue: 67/255, opacity: 0.28)
    /// 删除行背景(removed) — 半透明红
    static let diffRemovedBg = Color(red: 192/255, green: 57/255, blue: 43/255, opacity: 0.18)
    /// 删除行左侧标记栏
    static let diffRemovedGutter = Color(red: 192/255, green: 57/255, blue: 43/255, opacity: 0.30)
    /// 行号
    static let diffLineNumber = Color(hex: "#5b616a")

    // NSColor Diff versions
    static let nsDiffAddedBg = NSColor(srgbRed: 46/255, green: 160/255, blue: 67/255, alpha: 0.15)
    static let nsDiffAddedGutter = NSColor(srgbRed: 46/255, green: 160/255, blue: 67/255, alpha: 0.28)
    static let nsDiffRemovedBg = NSColor(srgbRed: 192/255, green: 57/255, blue: 43/255, alpha: 0.18)
    static let nsDiffRemovedGutter = NSColor(srgbRed: 192/255, green: 57/255, blue: 43/255, alpha: 0.30)

    // MARK: §2 字体排版

    /// UI 文字 — 系统无衬线字体
    static let uiFont = Font.system(size: 13)
    /// 标签栏标题
    static let tabFont = Font.system(size: 13)
    /// 列表行文字
    static let listFont = Font.system(size: 13)
    /// 区段标题 (字重 600)
    static let sectionFont = Font.system(size: 13, weight: .semibold)
    /// 说明性副文字
    static let captionFont = Font.system(size: 12)
    /// 代码/Diff 等宽字体
    static let codeFont = Font.custom("SF Mono", size: 12.5)
    /// 代码 fallback
    static let codeFontFallback = Font.system(size: 12.5, design: .monospaced)
    /// 小标签字体
    static let smallLabel = Font.system(size: 11)

    // NSFont versions
    static let nsCodeFont: NSFont = {
        if let font = NSFont(name: "SF Mono", size: 12.5) {
            return font
        }
        return NSFont.monospacedSystemFont(ofSize: 12.5, weight: .regular)
    }()

    // MARK: §3 尺寸常量

    /// 顶部窗口 + 标签栏高度
    static let titleBarHeight: CGFloat = 44
    /// 工具栏高度
    static let toolbarHeight: CGFloat = 64
    /// 左侧边栏展开宽度
    static let sidebarExpandedWidth: CGFloat = 260
    /// 左侧边栏折叠宽度
    static let sidebarCollapsedWidth: CGFloat = 48
    /// 右侧暂存面板宽度
    static let stagingPanelWidth: CGFloat = 470
    /// 提交行高度
    static let commitRowHeight: CGFloat = 36
    /// 列头高度
    static let columnHeaderHeight: CGFloat = 32
    /// 圆形节点直径
    static let commitNodeDiameter: CGFloat = 22

    // MARK: 分支颜色调色板

    /// 分支颜色轮换（不含紫色）
    static let branchColors: [Color] = [
        Color(hex: "#3fb6a8"), // teal
        Color(hex: "#4a90e2"), // blue
        Color(hex: "#e6994a"), // orange
        Color(hex: "#e25555"), // red
        Color(hex: "#4caf50"), // green
        Color(hex: "#e2cf4a"), // yellow
        Color(hex: "#4ac4e2"), // cyan
        Color(hex: "#e24a8f"), // pink
    ]

    /// 获取分支颜色(按 index 轮换)
    static func branchColor(at index: Int) -> Color {
        branchColors[index % branchColors.count]
    }
}
