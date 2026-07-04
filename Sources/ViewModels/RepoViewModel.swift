import SwiftUI
import Observation

enum FileListViewMode { case path, tree }

@Observable
class RepoViewModel {
    let gitService: GitService
    var currentBranch: String = ""
    var branches: [GitBranch] = []
    var tags: [GitTag] = []
    var stagedFiles: [GitFileStatus] = []
    var unstagedFiles: [GitFileStatus] = []
    var stashCount: Int = 0
    var graphViewModel = GraphViewModel()
    var diffViewModel = DiffViewModel()
    var isLoading = false

    // 中央区状态
    var showDiff = false
    var selectedFilePath: String?
    var selectedFileStaged = false

    // 提交选中(非 nil → 右侧显示提交详情面板)
    var selectedCommitHash: String?
    var selectedCommitFiles: [GitFileStatus] = []
    var selectedCommit: GitCommit?

    // 提交编辑
    var commitSummary = ""
    var commitDescription = ""
    var commitSignOff = false
    var commitAllowEmpty = false

    // 文件列表视图模式
    var fileViewMode: FileListViewMode = .path

    private var fileWatcher: FileWatcherService?
    private var externalRefreshTask: Task<Void, Never>?

    // Undo/Redo:仅针对「提交」。redoStack 保存被 Undo 掉的提交 hash(可 reset --soft 回去)。
    private(set) var redoStack: [String] = []
    var headHasParent = false          // HEAD 是否有父提交 → 决定 Undo 是否可用
    var canUndo: Bool { headHasParent }
    var canRedo: Bool { !redoStack.isEmpty }

    var totalChanges: Int { stagedFiles.count + unstagedFiles.count }
    var localBranches: [GitBranch] { branches.filter { $0.isLocal } }
    var remoteBranches: [GitBranch] { branches.filter { $0.isRemote } }

    var commitButtonState: CommitButtonState {
        let hasSummary = !commitSummary.trimmingCharacters(in: .whitespaces).isEmpty
        if commitAllowEmpty && hasSummary { return .ready }
        if stagedFiles.isEmpty { return .noStagedFiles }
        else if !hasSummary { return .noMessage }
        else { return .ready }
    }

    init(gitService: GitService) {
        self.gitService = gitService
        setupFileWatcher()
    }

    private func setupFileWatcher() {
        guard let path = gitService.repoPath else { return }
        // 监听整个仓库根(含 .git/):在别的程序里 commit / checkout / fetch 等都会写 .git,
        // 触发这里的回调,从而自动刷新提交图与分支,而不仅仅是工作区状态。
        fileWatcher = FileWatcherService(path: path) { [weak self] in
            Task { @MainActor [weak self] in
                self?.scheduleExternalRefresh()
            }
        }
        fileWatcher?.start()
    }

    /// 外部改动 → 去抖后做一次「安静」刷新(不切 isLoading,避免中央区闪烁)。
    /// FSEvents 本身已按 1s 合并事件,这里再叠加 250ms 去抖,避免连续写盘触发多次刷新。
    @MainActor
    private func scheduleExternalRefresh() {
        externalRefreshTask?.cancel()
        externalRefreshTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 250_000_000)
            if Task.isCancelled { return }
            await refreshStatus()
            await refreshBranches()
            await refreshTags()
            await refreshLog()
            await refreshStashCount()
            await refreshHeadState()
        }
    }

    func cleanup() {
        externalRefreshTask?.cancel()
        fileWatcher?.stop()
    }

    @MainActor
    func refresh() async {
        isLoading = true
        defer { isLoading = false }
        await refreshStatus()
        await refreshBranches()
        await refreshTags()
        await refreshLog()
        await refreshStashCount()
        await refreshHeadState()
    }

    @MainActor
    func refreshHeadState() async {
        headHasParent = await gitService.headHasParent()
    }

    @MainActor
    func refreshStatus() async {
        do {
            let statuses = try await gitService.getStatus()
            stagedFiles = statuses.filter { $0.isStaged }
            unstagedFiles = statuses.filter { !$0.isStaged }
            graphViewModel.changeCount = totalChanges
        } catch { print("[GitPilot] Status error: \(error)") }
    }

    @MainActor
    func refreshBranches() async {
        do {
            branches = try await gitService.getBranches()
            currentBranch = try await gitService.getCurrentBranch()
        } catch { print("[GitPilot] Branches error: \(error)") }
    }

    @MainActor
    func refreshTags() async {
        tags = (try? await gitService.getTags()) ?? []
    }

    @MainActor
    func refreshLog() async {
        do {
            let commits = try await gitService.getLog()
            graphViewModel.updateCommits(commits)
        } catch { print("[GitPilot] Log error: \(error)") }
    }

    @MainActor
    func refreshStashCount() async {
        stashCount = (try? await gitService.stashList().count) ?? 0
    }

    // MARK: - Staging

    @MainActor func stageFile(_ path: String) async {
        do { try await gitService.stageFile(path); await refreshStatus(); await reloadDiffIfNeeded(path: path) }
        catch { showErr("Stage", error) }
    }
    @MainActor func unstageFile(_ path: String) async {
        do { try await gitService.unstageFile(path); await refreshStatus(); await reloadDiffIfNeeded(path: path) }
        catch { showErr("Unstage", error) }
    }
    @MainActor func stageAll() async {
        do { try await gitService.stageAll(); await refreshStatus() } catch { showErr("Stage all", error) }
    }
    @MainActor func unstageAll() async {
        do { try await gitService.unstageAll(); await refreshStatus() } catch { showErr("Unstage all", error) }
    }

    /// 当前在 Diff 中查看的文件被 stage/unstage 后,刷新该文件的 diff 并跟随其暂存态
    @MainActor private func reloadDiffIfNeeded(path: String) async {
        guard showDiff, selectedFilePath == path else { return }
        let nowStaged = stagedFiles.contains { $0.path == path }
        let stillUnstaged = unstagedFiles.contains { $0.path == path }
        // 若文件只剩单边状态,跟随它;否则保持原侧
        if nowStaged && !stillUnstaged { selectedFileStaged = true }
        else if stillUnstaged && !nowStaged { selectedFileStaged = false }
        await diffViewModel.loadDiff(gitService: gitService, file: path, staged: selectedFileStaged)
    }

    @MainActor
    func performCommit() async {
        guard commitButtonState == .ready else { return }
        let summary = commitSummary
        var message = commitSummary
        let body = commitDescription.trimmingCharacters(in: .whitespacesAndNewlines)
        if !body.isEmpty { message += "\n\n" + body }
        do {
            try await gitService.commit(message: message,
                                         signOff: commitSignOff, allowEmpty: commitAllowEmpty)
            commitSummary = ""
            commitDescription = ""
            redoStack.removeAll()   // 新提交后,之前被「撤销」的提交不再可重做
            await refresh()
            ToastCenter.shared.show("Committed: \(summary)", style: .success)
        } catch { showErr("Commit", error) }
    }

    // MARK: - Undo / Redo(提交级)

    /// 撤销上次提交:reset --soft HEAD~1 —— 提交内容回到暂存区,记录被撤销的提交以便重做。
    @MainActor
    func undoLastCommit() async {
        guard headHasParent else { return }
        do {
            let old = try await gitService.execute(["rev-parse", "HEAD"])
                .trimmingCharacters(in: .whitespacesAndNewlines)
            try await gitService.resetSoft(to: "HEAD~1")
            redoStack.append(old)
            await refresh()
            ToastCenter.shared.show("Undid last commit — changes kept in staging", style: .info)
        } catch { showErr("Undo", error) }
    }

    /// 重做最近一次被撤销的提交:reset --soft 回那个(悬空的)提交对象。
    @MainActor
    func redoCommit() async {
        guard let hash = redoStack.last else { return }
        do {
            try await gitService.resetSoft(to: hash)
            redoStack.removeLast()
            await refresh()
            ToastCenter.shared.show("Redid commit", style: .info)
        } catch { showErr("Redo", error) }
    }

    // MARK: - Remote / branches

    @MainActor func pull(rebase: Bool = false) async throws {
        try await ActivityCenter.shared.track("Pulling…") {
            try await gitService.pull(rebase: rebase); await refresh()
        }
        ToastCenter.shared.show("Pulled from origin", style: .success)
    }
    @MainActor func push() async throws {
        try await ActivityCenter.shared.track("Pushing…") {
            try await gitService.push(); await refresh()
        }
        ToastCenter.shared.show("Pushed to origin", style: .success)
    }

    /// 解析 origin 的 host / 内嵌用户名(用于 push 鉴权弹窗)。
    @MainActor
    func remoteHostInfo() async -> (host: String, username: String)? {
        guard let url = try? await gitService.getRemoteURL() else { return nil }
        let parsed = GitService.parseRemote(url)
        return (parsed.host, parsed.username ?? "git")
    }

    /// 存 token:写入系统钥匙串(git 之后自动鉴权)+ 可选写入 App 钥匙串。push/pull/fetch 共用。
    @MainActor
    func storeToken(host: String, username: String, token: String, remember: Bool) async throws {
        try await gitService.approveCredential(host: host, username: username, token: token)
        if remember {
            try? KeychainService.shared.saveCredential(
                GitCredential(host: host, username: username, token: token, createdAt: Date()))
        }
    }

    /// 存 token 后重试 push。成功后以后自动鉴权。
    @MainActor
    func storeTokenAndPush(host: String, username: String, token: String, remember: Bool) async throws {
        try await storeToken(host: host, username: username, token: token, remember: remember)
        try await ActivityCenter.shared.track("Pushing…") {
            try await gitService.push()
            await refresh()
        }
        ToastCenter.shared.show(remember ? "Pushed — token saved for \(host)" : "Pushed to \(host)", style: .success)
    }

    /// 存 token 后重试 pull(pull() 内部已有进度条 + 成功提示)。
    @MainActor
    func storeTokenAndPull(host: String, username: String, token: String, remember: Bool, rebase: Bool) async throws {
        try await storeToken(host: host, username: username, token: token, remember: remember)
        try await pull(rebase: rebase)
    }

    /// 存 token 后重试 fetch(fetch() 内部已有进度条 + 成功提示)。
    @MainActor
    func storeTokenAndFetch(host: String, username: String, token: String, remember: Bool) async throws {
        try await storeToken(host: host, username: username, token: token, remember: remember)
        try await fetch()
    }
    @MainActor func pushSetUpstream() async throws {
        try await gitService.pushSetUpstream(branch: currentBranch); await refresh()
    }
    @MainActor func fetch() async throws {
        try await ActivityCenter.shared.track("Fetching…") {
            try await gitService.fetch(); await refresh()
        }
        ToastCenter.shared.show("Fetched from remotes", style: .success)
    }
    @MainActor func stash(message: String? = nil) async throws {
        try await ActivityCenter.shared.track("Stashing…") {
            try await gitService.stash(message: message); await refresh()
        }
        ToastCenter.shared.show("Changes stashed", style: .success)
    }
    @MainActor func stashPop() async throws {
        try await ActivityCenter.shared.track("Applying stash…") {
            try await gitService.stashPop(); await refresh()
        }
        ToastCenter.shared.show("Stash applied", style: .success)
    }
    @MainActor func checkoutBranch(_ name: String) async throws {
        redoStack.removeAll()   // 切分支后,重做目标可能已跨分支,不再安全
        try await ActivityCenter.shared.track("Switching to \(name)…") {
            try await gitService.checkout(branch: name); await refresh()
        }
        ToastCenter.shared.show("Switched to \(name)", style: .success)
    }
    @MainActor func createBranch(_ name: String) async throws {
        try await gitService.createBranch(name: name); await refresh()
        ToastCenter.shared.show("Created branch \(name)", style: .success)
    }
    @MainActor func deleteBranch(_ name: String, force: Bool = false) async throws {
        try await gitService.deleteBranch(name: name, force: force); await refreshBranches()
        ToastCenter.shared.show("Deleted branch \(name)", style: .info)
    }

    // MARK: - Diff (working tree)

    @MainActor
    func showFileDiff(_ path: String, staged: Bool) async {
        selectedCommitHash = nil          // 离开提交详情态
        selectedCommit = nil
        selectedFilePath = path
        selectedFileStaged = staged
        showDiff = true
        graphViewModel.selectedRowID = "WIP"
        await diffViewModel.loadDiff(gitService: gitService, file: path, staged: staged)
    }

    @MainActor func closeDiff() {
        showDiff = false
        selectedFilePath = nil
    }

    // MARK: - Commit selection

    @MainActor
    func selectCommitRow(_ hash: String) async {
        showDiff = false
        selectedFilePath = nil
        selectedCommitHash = hash
        graphViewModel.selectedRowID = hash
        selectedCommit = graphViewModel.baseRows.first(where: { $0.id == hash })?.commit
        selectedCommitFiles = (try? await gitService.getChangedFilesForCommit(hash: hash)) ?? []
    }

    @MainActor
    func selectWIPRow() {
        showDiff = false
        selectedFilePath = nil
        selectedCommitHash = nil
        selectedCommit = nil
        selectedCommitFiles = []
        graphViewModel.selectedRowID = "WIP"
    }

    @MainActor
    func showCommitFileDiff(hash: String, path: String) async {
        selectedFilePath = path
        showDiff = true
        await diffViewModel.loadCommitDiff(gitService: gitService, hash: hash, file: path)
    }

    // MARK: - Discard

    @MainActor
    func discardAllChanges() async {
        do {
            try await gitService.unstageAll()
            try await gitService.discardAllChanges()
            try await gitService.cleanUntracked()
            await refreshStatus()
            ToastCenter.shared.show("Discarded all changes", style: .info)
        } catch { showErr("Discard", error) }
    }

    @MainActor
    func discardFile(_ path: String) async {
        do {
            try await gitService.discardFile(path)
            await refreshStatus()
            if selectedFilePath == path { closeDiff() }
        } catch { showErr("Discard file", error) }
    }

    @MainActor
    func addToGitignore(_ pattern: String) async {
        do {
            try gitService.appendToGitignore(pattern)
            await refreshStatus()
            ToastCenter.shared.show("Added to .gitignore: \(pattern)", style: .success)
        } catch { showErr("Update .gitignore", error) }
    }

    // MARK: - Errors

    var lastError: String?
    var showError = false
    @MainActor private func showErr(_ ctx: String, _ error: Error) {
        ToastCenter.shared.show("\(ctx) failed: \(error.localizedDescription)", style: .error)
        print("[GitPilot] \(ctx) error: \(error)")
    }
}

enum CommitButtonState {
    case noStagedFiles, noMessage, ready
    var buttonText: String {
        switch self {
        case .noStagedFiles: return "Stage Changes to Commit"
        case .noMessage: return "Type a Message to Commit"
        case .ready: return "Commit"
        }
    }
    var isEnabled: Bool { self == .ready }
}
