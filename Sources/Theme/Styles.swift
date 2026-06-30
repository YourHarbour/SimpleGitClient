import SwiftUI

// MARK: - 工具栏 / 图标按钮(hover 浮起、按下反馈)

struct ToolbarButtonStyle: ButtonStyle {
    var cornerRadius: CGFloat = 6

    func makeBody(configuration: Configuration) -> some View {
        StyleBody(configuration: configuration, cornerRadius: cornerRadius)
    }

    private struct StyleBody: View {
        let configuration: ButtonStyleConfiguration
        let cornerRadius: CGFloat
        @Environment(\.isEnabled) private var isEnabled
        @State private var hover = false

        var body: some View {
            configuration.label
                .background(RoundedRectangle(cornerRadius: cornerRadius).fill(background))
                .scaleEffect(configuration.isPressed && isEnabled ? 0.94 : 1)
                .onHover { hover = isEnabled ? $0 : false }
                .animation(.easeOut(duration: 0.12), value: hover)
                .animation(.easeOut(duration: 0.10), value: configuration.isPressed)
        }

        private var background: Color {
            guard isEnabled else { return .clear }
            if configuration.isPressed { return Theme.bgElevated }
            return hover ? Theme.bgHover : .clear
        }
    }
}

// MARK: - 主操作按钮(填充色,hover 提亮、按下变暗)

struct PrimaryButtonStyle: ButtonStyle {
    var enabledColor: Color = Theme.commitGreen
    var disabledColor: Color = Theme.accentGreenDeep
    var cornerRadius: CGFloat = 5

    func makeBody(configuration: Configuration) -> some View {
        StyleBody(configuration: configuration, enabledColor: enabledColor,
             disabledColor: disabledColor, cornerRadius: cornerRadius)
    }

    private struct StyleBody: View {
        let configuration: ButtonStyleConfiguration
        let enabledColor: Color
        let disabledColor: Color
        let cornerRadius: CGFloat
        @Environment(\.isEnabled) private var isEnabled
        @State private var hover = false

        var body: some View {
            configuration.label
                .background(RoundedRectangle(cornerRadius: cornerRadius).fill(fill))
                .brightness(isEnabled && hover && !configuration.isPressed ? 0.06 : 0)
                .onHover { hover = $0 }
                .animation(.easeOut(duration: 0.12), value: hover)
                .animation(.easeOut(duration: 0.10), value: configuration.isPressed)
        }

        private var fill: Color {
            guard isEnabled else { return disabledColor }
            return configuration.isPressed ? enabledColor.opacity(0.82) : enabledColor
        }
    }
}

// MARK: - 描边按钮(Stage / Unstage 等)

struct OutlineButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        StyleBody(configuration: configuration)
    }

    private struct StyleBody: View {
        let configuration: ButtonStyleConfiguration
        @State private var hover = false

        var body: some View {
            configuration.label
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(Theme.accentGreen)
                .padding(.horizontal, 8).padding(.vertical, 3)
                .background(
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Theme.accentGreen.opacity(configuration.isPressed ? 0.24 : (hover ? 0.16 : 0.08)))
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 4)
                        .stroke(Theme.accentGreen.opacity(hover ? 0.85 : 0.5), lineWidth: 1)
                )
                .onHover { hover = $0 }
                .animation(.easeOut(duration: 0.12), value: hover)
        }
    }
}

// MARK: - 工具栏下拉选择器(名字 + 右侧箭头,自定义视觉 + 不可见原生 Menu 提供下拉)

struct DropdownSelector<Items: View>: View {
    let title: String
    @ViewBuilder var items: () -> Items
    @State private var hover = false

    var body: some View {
        HStack(spacing: 5) {
            Text(title)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)
                .lineLimit(1)
            Image(systemName: "chevron.down")
                .font(.system(size: 8, weight: .semibold))
                .foregroundStyle(Theme.textSecondary)
        }
        .padding(.horizontal, 6).padding(.vertical, 3)
        .background(RoundedRectangle(cornerRadius: 5).fill(hover ? Theme.bgHover : Color.clear))
        .overlay {
            // 不可见的原生 Menu(铺满)提供下拉行为;视觉完全由上面的 HStack 控制,箭头稳定在右侧。
            Menu { items() } label: { Color.clear.contentShape(Rectangle()) }
                .menuStyle(.borderlessButton)
                .opacity(0.02)
        }
        .onHover { hover = $0 }
        .animation(.easeOut(duration: 0.12), value: hover)
    }
}

// MARK: - 段控按钮(Path/Tree、File/Diff View 等),带 hover

struct SegmentButton: View {
    let title: String
    var icon: String? = nil
    let active: Bool
    let action: () -> Void
    @State private var hover = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 4) {
                if let icon { Image(systemName: icon).font(.system(size: 10)) }
                Text(title).font(.system(size: 12, weight: .medium))
            }
            .foregroundStyle(active ? Theme.textPrimary : Theme.textSecondary)
            .padding(.horizontal, 12).padding(.vertical, 4)
            .background(
                RoundedRectangle(cornerRadius: 5)
                    .fill(active ? Theme.bgElevated : (hover ? Theme.bgHover : Color.clear))
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hover = $0 }
        .animation(.easeOut(duration: 0.12), value: hover)
    }
}
