import SwiftUI
import AppKit

/// 主布局 — 按 spec §3 四大横向区域
struct MainLayout: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var toastCenter = ToastCenter.shared
    @State private var activityCenter = ActivityCenter.shared
    @State private var sidebarWidth: CGFloat = 260
    @State private var stagingWidth: CGFloat = 460

    var body: some View {
        // 让 MainLayout 观察 toast / 进度变化,变化时重建底部 overlay
        let _ = toastCenter.toasts.count
        let _ = activityCenter.items.count
        return VStack(spacing: 0) {
            TitleBarView()      // ① §4

            if appVM.activeRepo != nil {
                ToolbarView()   // ② §5
                mainWorkArea    // ③ §6 §7 §8
            } else {
                LandingView()   // New Tab 落地页
            }
        }
        .background(Theme.bgApp)
        .background(WindowAccessor(titleBarHeight: Theme.titleBarHeight))
        .ignoresSafeArea(.container, edges: .top)
        .overlay(alignment: .bottomLeading) { ToastOverlay() }
        .overlay(alignment: .bottomTrailing) { ActivityOverlay() }
        .task { await appVM.bootstrap() }
        .alert("Error", isPresented: Binding(
            get: { appVM.showError }, set: { appVM.showError = $0 }
        )) {
            Button("OK") { appVM.showError = false }
        } message: {
            Text(appVM.errorMessage ?? "Unknown error")
        }
        .alert("Git Error", isPresented: Binding(
            get: { appVM.activeRepo?.showError ?? false },
            set: { appVM.activeRepo?.showError = $0 }
        )) {
            Button("OK") { appVM.activeRepo?.showError = false }
        } message: {
            Text(appVM.activeRepo?.lastError ?? "Unknown error")
        }
        .sheet(isPresented: Binding(
            get: { appVM.showCredentialsSheet }, set: { appVM.showCredentialsSheet = $0 }
        )) { CredentialsSheet() }
        .sheet(isPresented: Binding(
            get: { appVM.showCloneSheet }, set: { appVM.showCloneSheet = $0 }
        )) { CloneSheet() }
        .sheet(isPresented: Binding(
            get: { appVM.showCreateBranchSheet }, set: { appVM.showCreateBranchSheet = $0 }
        )) { CreateBranchSheet() }
        .sheet(isPresented: Binding(
            get: { appVM.showAuthSheet }, set: { appVM.showAuthSheet = $0 }
        )) { AuthSheet() }
    }

    // MARK: - 三栏工作区(固定/可拖拽宽度,中央自适应,切换内容不抖动)

    private var mainWorkArea: some View {
        GeometryReader { geo in
            let total = geo.size.width
            let sidebarW = appVM.sidebarCollapsed ? Theme.sidebarCollapsedWidth : sidebarWidth
            // 保证中央区至少 ~300pt,窗口很窄时优先压缩右侧面板
            let maxStaging = max(340, total - sidebarW - 300)
            let stagingW = min(stagingWidth, maxStaging)

            HStack(spacing: 0) {
                LeftSidebarView()
                    .frame(width: sidebarW)

                if !appVM.sidebarCollapsed {
                    PanelResizeHandle(width: $sidebarWidth, range: 200...440, invert: false)
                }

                centerArea
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                PanelResizeHandle(width: $stagingWidth, range: 360...680, invert: true)

                rightPanel
                    .frame(width: stagingW)
            }
            .frame(width: total, height: geo.size.height)
        }
    }

    @ViewBuilder
    private var rightPanel: some View {
        if let repo = appVM.activeRepo, repo.selectedCommitHash != nil {
            CommitDetailPanelView()
        } else {
            StagingPanelView()
        }
    }

    @ViewBuilder
    private var centerArea: some View {
        if let repo = appVM.activeRepo {
            if repo.showDiff {
                DiffViewerView()
            } else {
                CommitGraphView()
            }
        }
    }

}

// MARK: - 面板拖拽手柄

struct PanelResizeHandle: View {
    @Binding var width: CGFloat
    let range: ClosedRange<CGFloat>
    var invert: Bool          // 右侧面板:向左拖拽 = 变宽
    @State private var startWidth: CGFloat?

    var body: some View {
        Rectangle()
            .fill(Theme.border)
            .frame(width: 1)
            .overlay(
                Rectangle()
                    .fill(Color.clear)
                    .frame(width: 9)
                    .contentShape(Rectangle())
                    .onHover { inside in
                        if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() }
                    }
                    .gesture(
                        DragGesture(minimumDistance: 1)
                            .onChanged { value in
                                if startWidth == nil { startWidth = width }
                                let base = startWidth ?? width
                                let delta = invert ? -value.translation.width : value.translation.width
                                width = min(max(base + delta, range.lowerBound), range.upperBound)
                            }
                            .onEnded { _ in startWidth = nil }
                    )
            )
    }
}
