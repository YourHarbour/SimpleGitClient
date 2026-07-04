import SwiftUI

/// 工具栏 — spec §5 (高 ~64px)
struct ToolbarView: View {
    @Environment(AppViewModel.self) private var appVM

    var body: some View {
        HStack(spacing: 0) {
            // 左侧: 仓库/分支选择
            leftSection

            Spacer()

            // 右侧: 操作按钮组
            rightSection
        }
        .frame(height: Theme.toolbarHeight)
        .padding(.horizontal, 16)
        .background(Theme.bgApp)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(Theme.border)
                .frame(height: 1)
        }
    }

    // MARK: - Left Section (Repository / Branch selectors)

    private var leftSection: some View {
        HStack(spacing: 24) {
            // Repository selector
            if let repo = appVM.activeRepo {
                VStack(alignment: .leading, spacing: 2) {
                    Text("repository")
                        .font(.system(size: 10))
                        .foregroundStyle(Theme.textMuted)

                    DropdownSelector(title: appVM.tabs.first(where: { $0.id == appVM.activeTabId })?.name ?? "Unknown") {
                        ForEach(appVM.tabs.filter { $0.viewModel != nil }) { tab in
                            Button(tab.name) { appVM.selectTab(tab.id) }
                        }
                        Divider()
                        Button("Open Repository…") { appVM.showOpenDialog() }
                        Button("Clone Repository…") { appVM.showCloneSheet = true }
                    }
                }

                // Branch selector
                VStack(alignment: .leading, spacing: 2) {
                    Text("branch")
                        .font(.system(size: 10))
                        .foregroundStyle(Theme.textMuted)

                    DropdownSelector(title: repo.currentBranch.isEmpty ? "main" : repo.currentBranch) {
                        ForEach(repo.localBranches) { branch in
                            Button(action: {
                                Task { try? await repo.checkoutBranch(branch.name) }
                            }) {
                                HStack {
                                    if branch.isCurrent {
                                        Image(systemName: "checkmark")
                                    }
                                    Text(branch.name)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // MARK: - Right Section (Action buttons)

    private var rightSection: some View {
        HStack(spacing: 4) {
            // Undo(撤销上次提交 → reset --soft HEAD~1)
            toolbarButton(icon: "arrow.uturn.backward", label: "Undo",
                          disabled: !(appVM.activeRepo?.canUndo ?? false)) {
                Task { await appVM.activeRepo?.undoLastCommit() }
            }

            // Redo(重做被撤销的提交)
            toolbarButton(icon: "arrow.uturn.forward", label: "Redo",
                          disabled: !(appVM.activeRepo?.canRedo ?? false)) {
                Task { await appVM.activeRepo?.redoCommit() }
            }

            toolbarDivider

            // Pull (with dropdown)
            HStack(spacing: 0) {
                toolbarButton(icon: "arrow.down.doc", label: "Pull") {
                    runRemote(.pull(rebase: false), label: "Pull") { try await $0.pull(rebase: false) }
                }

                Menu {
                    Button("Pull (merge)") {
                        runRemote(.pull(rebase: false), label: "Pull") { try await $0.pull(rebase: false) }
                    }
                    Button("Pull (rebase)") {
                        runRemote(.pull(rebase: true), label: "Pull") { try await $0.pull(rebase: true) }
                    }
                } label: {
                    Image(systemName: "chevron.down")
                        .font(.system(size: 7))
                        .foregroundStyle(Theme.textSecondary)
                        .frame(width: 16, height: 16)
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
            }

            // Push
            toolbarButton(icon: "arrow.up.doc", label: "Push") {
                runRemote(.push, label: "Push") { try await $0.push() }
            }

            toolbarDivider

            // Fetch
            toolbarButton(icon: "arrow.triangle.2.circlepath", label: "Fetch") {
                runRemote(.fetch, label: "Fetch") { try await $0.fetch() }
            }

            toolbarDivider

            // Branch
            toolbarButton(icon: "arrow.triangle.branch", label: "Branch") {
                appVM.showCreateBranchSheet = true
            }

            // Stash
            toolbarButton(icon: "tray.and.arrow.down", label: "Stash") {
                guard let repo = appVM.activeRepo else { return }
                Task {
                    try? await repo.stash()
                }
            }

            // Pop
            toolbarButton(
                icon: "tray.and.arrow.up",
                label: "Pop",
                disabled: (appVM.activeRepo?.stashCount ?? 0) == 0
            ) {
                guard let repo = appVM.activeRepo else { return }
                Task {
                    try? await repo.stashPop()
                }
            }
        }
    }

    /// 执行一个远程操作(pull/push/fetch)。若因缺 token 报 `authenticationRequired`,
    /// 弹鉴权框(存好 token 后由 submitAuth 自动重试);其它错误弹红 toast。
    private func runRemote(_ context: AppViewModel.AuthContext, label: String,
                           _ op: @escaping (RepoViewModel) async throws -> Void) {
        guard let repo = appVM.activeRepo else { return }
        Task {
            do {
                try await op(repo)
            } catch {
                if case GitError.authenticationRequired = error {
                    await appVM.beginRemoteAuth(context, for: repo)
                } else {
                    ToastCenter.shared.show("\(label) failed: \(error.localizedDescription)", style: .error)
                }
            }
        }
    }

    // MARK: - Toolbar Button Component

    private func toolbarButton(
        icon: String,
        label: String,
        disabled: Bool = false,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            VStack(spacing: 3) {
                Image(systemName: icon)
                    .font(.system(size: 16))
                Text(label)
                    .font(.system(size: 10))
            }
            .foregroundStyle(disabled ? Theme.textMuted : Theme.textSecondary)
            .frame(width: 52, height: 48)
            .contentShape(Rectangle())
        }
        .buttonStyle(ToolbarButtonStyle())
        .disabled(disabled)
    }

    private var toolbarDivider: some View {
        Rectangle()
            .fill(Theme.border)
            .frame(width: 1, height: 32)
            .padding(.horizontal, 4)
    }
}
