import SwiftUI

/// 左侧边栏 — spec §6 (展开 ~260px / 折叠 ~48px)
struct LeftSidebarView: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var filterText = ""
    @State private var expanded: Set<String> = ["LOCAL", "REMOTE"]
    /// 非 nil = 这个分支未完全合并,正在问用户要不要强删。
    @State private var forceDeleteTarget: String?

    var body: some View {
        Group {
            if appVM.sidebarCollapsed { collapsedView } else { expandedView }
        }
        .background(Theme.bgPanel)
        .overlay(alignment: .trailing) { Rectangle().fill(Theme.border).frame(width: 1) }
        .confirmationDialog(
            "Delete “\(forceDeleteTarget ?? "")”?",
            isPresented: Binding(get: { forceDeleteTarget != nil },
                                 set: { if !$0 { forceDeleteTarget = nil } }),
            titleVisibility: .visible
        ) {
            Button("Force Delete", role: .destructive) {
                if let name = forceDeleteTarget {
                    Task { _ = await appVM.activeRepo?.deleteBranchChecked(name, force: true) }
                }
                forceDeleteTarget = nil
            }
            Button("Cancel", role: .cancel) { forceDeleteTarget = nil }
        } message: {
            Text("“\(forceDeleteTarget ?? "")” isn't fully merged. Force-deleting it loses the commits that exist only on this branch.")
        }
    }

    // MARK: - §6.1 Expanded

    private var expandedView: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 折叠按钮
            HStack {
                collapseButton(icon: "chevron.left") { appVM.sidebarCollapsed = true }
                Spacer()
            }
            .padding(.horizontal, 12).padding(.top, 10).padding(.bottom, 8)

            // 过滤框
            HStack(spacing: 6) {
                TextField("Filter (⌘ + Option + f)", text: $filterText)
                    .textFieldStyle(.plain)
                    .font(Theme.listFont)
                    .foregroundStyle(Theme.textPrimary)
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 11)).foregroundStyle(Theme.textMuted)
            }
            .padding(.horizontal, 8).padding(.vertical, 6)
            .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
            .padding(.horizontal, 12).padding(.bottom, 8)

            // 分组列表
            ScrollView {
                VStack(spacing: 0) {
                    if let repo = appVM.activeRepo {
                        section("LOCAL", icon: "desktopcomputer", count: repo.localBranches.count) {
                            ForEach(filtered(repo.localBranches)) { b in branchRow(b, repo: repo) }
                        }
                        section("REMOTE", icon: "cloud", count: repo.remoteBranches.count) {
                            ForEach(filtered(repo.remoteBranches)) { b in branchRow(b, repo: repo) }
                        }
                        section("TAGS", icon: "tag", count: repo.tags.count) {
                            ForEach(repo.tags.filter { filterText.isEmpty || $0.name.localizedCaseInsensitiveContains(filterText) }) { t in
                                simpleRow(icon: "tag", text: t.name, color: Theme.accentBlue)
                            }
                        }
                    }
                }
                .padding(.top, 2)
            }
        }
        .frame(maxHeight: .infinity)
    }

    // MARK: - §6.2 Collapsed icon rail

    private var collapsedView: some View {
        VStack(spacing: 2) {
            collapseButton(icon: "chevron.right") { appVM.sidebarCollapsed = false }
                .padding(.vertical, 8)
            if let repo = appVM.activeRepo {
                railIcon("desktopcomputer", count: repo.localBranches.count)
                railIcon("cloud", count: repo.remoteBranches.count)
                railIcon("tag", count: repo.tags.count)
            }
            Spacer()
        }
        .frame(width: Theme.sidebarCollapsedWidth)
        .frame(maxHeight: .infinity)
    }

    // MARK: - Components

    private func collapseButton(icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(Theme.textSecondary)
                .frame(width: 28, height: 28)
                .background(Circle().fill(Theme.bgElevated))
        }
        .buttonStyle(.plain)
    }

    @ViewBuilder
    private func section<Content: View>(_ title: String, icon: String, count: Int?,
                                        @ViewBuilder content: () -> Content) -> some View {
        let isOpen = expanded.contains(title)
        VStack(spacing: 0) {
            Button(action: { toggle(title) }) {
                HStack(spacing: 8) {
                    Image(systemName: isOpen ? "chevron.down" : "chevron.right")
                        .font(.system(size: 9, weight: .semibold)).foregroundStyle(Theme.textMuted).frame(width: 10)
                    Image(systemName: icon).font(.system(size: 11)).foregroundStyle(Theme.textSecondary).frame(width: 16)
                    Text(title).font(Theme.sectionFont).foregroundStyle(Theme.textSecondary)
                    Spacer()
                    if let count { Text("\(count)").font(.system(size: 11)).foregroundStyle(Theme.textMuted) }
                }
                .padding(.horizontal, 12).padding(.vertical, 7)
                .contentShape(Rectangle())
            }
            .buttonStyle(HoverRowStyle())

            if isOpen { content() }
        }
    }

    private func branchRow(_ branch: GitBranch, repo: RepoViewModel) -> some View {
        Button(action: { checkout(branch, repo: repo) }) {
            HStack(spacing: 8) {
                Color.clear.frame(width: 18)
                Image(systemName: branch.isCurrent ? "checkmark" : "arrow.triangle.branch")
                    .font(.system(size: 10, weight: branch.isCurrent ? .bold : .regular))
                    .foregroundStyle(branch.isCurrent ? Theme.accentTeal : Theme.textMuted)
                    .frame(width: 14)
                Text(branch.displayName)
                    .font(Theme.listFont)
                    .foregroundStyle(branch.isCurrent ? Theme.textPrimary : Theme.textSecondary)
                    .lineLimit(1)
                Spacer()
            }
            .padding(.vertical, 4).padding(.horizontal, 12)
            .contentShape(Rectangle())
        }
        .buttonStyle(HoverRowStyle())
        .contextMenu {
            if !branch.isCurrent {
                // 远程分支没有本地同名分支时,检出等于「建立跟踪分支」—— 标题写清楚。
                Button(checkoutLabel(branch, repo: repo)) { checkout(branch, repo: repo) }
                Divider()
            }
            Button("Copy Branch Name") { Clipboard.copy(branch.name, label: "Copied branch name") }
            if branch.isLocal && !branch.isCurrent {
                Divider()
                Button("Delete", role: .destructive) {
                    Task {
                        // 未合并时 git 会拒绝 -d;弹确认框问是否 -D,而不是静默失败。
                        if await repo.deleteBranchChecked(branch.name) == .needsForce {
                            forceDeleteTarget = branch.name
                        }
                    }
                }
            }
        }
    }

    /// 点分支行 = 切过去。本地分支直接 switch;远程分支建/切同名本地跟踪分支。
    private func checkout(_ branch: GitBranch, repo: RepoViewModel) {
        guard !branch.isCurrent else { return }
        Task {
            await repo.run("Checkout") {
                branch.isLocal
                    ? try await repo.checkoutBranch(branch.name)
                    : try await repo.checkoutRemoteBranch(branch)
            }
        }
    }

    private func checkoutLabel(_ branch: GitBranch, repo: RepoViewModel) -> String {
        guard branch.isRemote else { return "Checkout" }
        let hasLocal = repo.localBranches.contains { $0.name == branch.displayName }
        return hasLocal ? "Checkout \u{201c}\(branch.displayName)\u{201d}"
                        : "Checkout as Local Branch \u{201c}\(branch.displayName)\u{201d}"
    }

    private func simpleRow(icon: String, text: String, color: Color) -> some View {
        HStack(spacing: 8) {
            Color.clear.frame(width: 18)
            Image(systemName: icon).font(.system(size: 10)).foregroundStyle(color).frame(width: 14)
            Text(text).font(Theme.listFont).foregroundStyle(Theme.textSecondary).lineLimit(1)
            Spacer()
        }
        .padding(.vertical, 4).padding(.horizontal, 12)
    }

    private func railIcon(_ icon: String, count: Int?) -> some View {
        ZStack(alignment: .bottomTrailing) {
            Image(systemName: icon)
                .font(.system(size: 15)).foregroundStyle(Theme.textSecondary)
                .frame(width: 36, height: 32)
            if let count, count > 0 {
                Text("\(count)")
                    .font(.system(size: 9, weight: .bold)).foregroundStyle(Theme.textPrimary)
                    .padding(.horizontal, 3).padding(.vertical, 1)
                    .background(Capsule().fill(Theme.accentTeal))
                    .offset(x: 4, y: 0)
            }
        }
    }

    // MARK: - Helpers

    private func toggle(_ title: String) {
        if expanded.contains(title) { expanded.remove(title) } else { expanded.insert(title) }
    }

    private func filtered(_ branches: [GitBranch]) -> [GitBranch] {
        filterText.isEmpty ? branches : branches.filter { $0.name.localizedCaseInsensitiveContains(filterText) }
    }
}

// MARK: - Hover row style

struct HoverRowStyle: ButtonStyle {
    @State private var hovered = false
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(hovered ? Theme.bgHover : Color.clear)
            .onHover { hovered = $0 }
    }
}
