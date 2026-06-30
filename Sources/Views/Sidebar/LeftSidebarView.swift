import SwiftUI

/// 左侧边栏 — spec §6 (展开 ~260px / 折叠 ~48px)
struct LeftSidebarView: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var filterText = ""
    @State private var expanded: Set<String> = ["LOCAL", "REMOTE"]

    var body: some View {
        Group {
            if appVM.sidebarCollapsed { collapsedView } else { expandedView }
        }
        .background(Theme.bgPanel)
        .overlay(alignment: .trailing) { Rectangle().fill(Theme.border).frame(width: 1) }
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
                        section("WORKTREES", icon: "rectangle.split.3x1", count: repo.worktrees.count) {
                            ForEach(repo.worktrees) { w in
                                simpleRow(icon: "rectangle.split.3x1", text: w.branch ?? w.name, color: Theme.textSecondary)
                            }
                        }
                        section("CLOUD PATCHES", icon: "bookmark", count: 0) { EmptyView() }
                        section("PULL REQUESTS", icon: "arrow.triangle.pull", count: 0) { EmptyView() }
                        section("ISSUES", icon: "list.bullet", count: nil) { EmptyView() }
                        section("TEAMS", icon: "person.2", count: nil) { EmptyView() }
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
                railIcon("rectangle.split.3x1", count: repo.worktrees.count)
                railIcon("bookmark", count: 0)
                railIcon("arrow.triangle.pull", count: 0)
                railIcon("list.bullet", count: nil)
                railIcon("person.2", count: nil)
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
        Button(action: {
            if !branch.isCurrent && branch.isLocal {
                Task { try? await repo.checkoutBranch(branch.name) }
            }
        }) {
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
            if branch.isLocal && !branch.isCurrent {
                Button("Checkout") { Task { try? await repo.checkoutBranch(branch.name) } }
                Button("Delete", role: .destructive) { Task { try? await repo.deleteBranch(branch.name) } }
            }
        }
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
