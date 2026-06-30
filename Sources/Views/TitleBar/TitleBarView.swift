import SwiftUI

/// 顶部窗口 + 标签栏 — spec §4 (高 ~44px)
struct TitleBarView: View {
    @Environment(AppViewModel.self) private var appVM

    var body: some View {
        HStack(spacing: 0) {
            // 让出 macOS 红绿灯按钮空间 + 与第一个 tab 的间距
            Color.clear.frame(width: 92, height: Theme.titleBarHeight)

            tabsArea

            // + 新建标签(落地页:Open / Clone)
            titleBarButton(icon: "plus", tooltip: "New Tab") { appVM.newTab() }

            Spacer()

            // ⚙️ 设置 → 凭据
            titleBarButton(icon: "gearshape", tooltip: "Settings & Credentials") {
                appVM.showCredentialsSheet = true
            }
            .padding(.trailing, 12)
        }
        .frame(height: Theme.titleBarHeight)
        .background(Theme.bgApp)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
    }

    private var tabsArea: some View {
        HStack(spacing: 0) {
            ForEach(appVM.tabs) { tab in
                TabItemView(tab: tab, isActive: tab.id == appVM.activeTabId)
            }
        }
    }

    private func titleBarButton(icon: String, tooltip: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundStyle(Theme.textSecondary)
                .frame(width: 32, height: 32)
                .contentShape(Rectangle())
        }
        .buttonStyle(ToolbarButtonStyle())
        .help(tooltip)
    }
}

// MARK: - Tab item (hover 高亮)

struct TabItemView: View {
    let tab: RepoTab
    let isActive: Bool
    @Environment(AppViewModel.self) private var appVM
    @State private var hover = false

    var body: some View {
        HStack(spacing: 6) {
            if tab.viewModel != nil {
                Image(systemName: "arrow.triangle.branch")
                    .font(.system(size: 11))
                    .foregroundStyle(isActive ? Theme.textPrimary : Theme.textSecondary)
            }
            Text(tab.name)
                .font(Theme.tabFont)
                .foregroundStyle(isActive ? Theme.textPrimary : Theme.textSecondary)
                .lineLimit(1)

            Button(action: { appVM.closeTab(tab.id) }) {
                Image(systemName: "xmark")
                    .font(.system(size: 9, weight: .semibold))
                    .foregroundStyle(Theme.textMuted)
                    .frame(width: 16, height: 16)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .opacity(isActive || hover ? 1 : 0.4)
        }
        .padding(.horizontal, 14)
        .frame(maxHeight: .infinity)   // 满高,底部贴住分割线
        .background(
            UnevenRoundedRectangle(topLeadingRadius: 6, bottomLeadingRadius: 0,
                                   bottomTrailingRadius: 0, topTrailingRadius: 6)
                .fill(isActive ? Theme.selectedTab : (hover ? Theme.bgHover : Color.clear))
        )
        .contentShape(Rectangle())
        .onHover { hover = $0 }
        .onTapGesture { appVM.selectTab(tab.id) }
        .contextMenu {
            Button("Close Tab") { appVM.closeTab(tab.id) }
            Button("Close Other Tabs") { appVM.closeAllOtherTabs(except: tab.id) }
        }
    }
}
