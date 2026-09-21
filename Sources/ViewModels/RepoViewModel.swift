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
    private var autoFetchTask: Task<Void, Never>?

    /// 最近一次(自动或手动)fetch 成功的时间,给工具栏显示用。
    private(set) var lastFetchDate: Date?

    // Undo/Redo:仅针对「提交」。redoStack 保存被 Undo 掉的提交 hash(可 reset --soft 回去)。
    private(set) var redoStack: [String] = []
    var headHasParent = false          // HEAD 是否有父提交 → 决定 Undo 是否可用
    var canUndo: Bool { headHasParent }
    var canRedo: Bool { !redoStack.isEmpty }

    var totalChanges: Int { stagedFiles.count + unstagedFiles.count }
    var localBranches: [GitBranch] { branches.filter { $0.isLocal } }
    var remoteBranches: [GitBranch] { branches.filter { $0.isRemote } }
    var currentLocalBranch: GitBranch? {
        localBranches.first { $0.isCurrent } ?? localBranches.first { $0.name == currentBranch }
    }

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
        restartAutoFetch()
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
        autoFetchTask?.cancel()
        fileWatcher?.stop()
    }

    // MARK: - 后台自动 fetch

    /// 后台定时 fetch —— 别的 Git 客户端(GitKraken / Fork / GitHub Desktop)都这么做:
    /// 远程的 ahead/behind 不靠手动点 Fetch 才更新。静默执行:不弹进度条、不弹 toast、
    /// 出错(离线 / 缺 token)直接吞掉,下一轮再试。
    func restartAutoFetch() {
        autoFetchTask?.cancel()
        guard AppSettings.autoFetchEnabled else { return }
        autoFetchTask = Task { @MainActor [weak self] in
            // 开仓库的头几秒让首次 refresh 先跑完
            try? await Task.sleep(nanoseconds: 5_000_000_000)
            while !Task.isCancelled {
                guard let self else { return }
                await self.autoFetchOnce()
                let interval = AppSettings.autoFetchInterval
                try? await Task.sleep(nanoseconds: UInt64(interval * 1_000_000_000))
            }
        }
    }

    @MainActor
    private func autoFetchOnce() async {
        guard AppSettings.autoFetchEnabled else { return }
        guard await gitService.hasRemote() else { return }
        do {
            try await gitService.fetch()
            lastFetchDate = Date()
            await refreshBranches()
            await refreshLog()
        } catch {
            // 离线 / 未授权:静默跳过,不打扰用户
        }
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
        try await ActivityCenter.shared.track("Fetching & pulling…") {
            try await gitService.pull(rebase: rebase); await refresh()
        }
        lastFetchDate = Date()
        ToastCenter.shared.show("Pulled from origin", style: .success)
    }
    @MainActor func push() async throws {
        let result = try await pushCurrentBranch()
        let shouldSetUpstream = result.setUpstream
        let branchName = result.branchName
        if shouldSetUpstream, let branchName {
            ToastCenter.shared.show("Pushed \(branchName) and set upstream", style: .success)
        } else {
            ToastCenter.shared.show("Pushed to origin", style: .success)
        }
    }

    @MainActor
    private func pushCurrentBranch() async throws -> (setUpstream: Bool, branchName: String?) {
        let branch = currentLocalBranch
        let branchName = branch?.name
        let shouldSetUpstream = branch?.trackingBranch == nil && !(branchName ?? "").isEmpty
        try await ActivityCenter.shared.track("Pushing…") {
            if shouldSetUpstream, let branchName {
                try await gitService.pushSetUpstream(branch: branchName)
            } else {
                try await gitService.push()
            }
            await refresh()
        }
        return (shouldSetUpstream, branchName)
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
        _ = try await pushCurrentBranch()
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
        lastFetchDate = Date()
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
    /// 检出一个远程分支。本地已有同名分支 → 直接切过去;否则建一个跟踪它的本地分支。
    /// (远程分支本身是只读引用,直接 `switch origin/x` 会进 detached HEAD,不是用户想要的。)
    @MainActor func checkoutRemoteBranch(_ branch: GitBranch) async throws {
        let local = branch.displayName
        let existing = localBranches.first { $0.name == local }
        if existing?.isCurrent == true {
            ToastCenter.shared.show("Already on \(local)", style: .info)
            return
        }
        redoStack.removeAll()
        try await ActivityCenter.shared.track("Switching to \(local)…") {
            if existing != nil {
                try await gitService.checkout(branch: local)
            } else {
                try await gitService.checkoutTracking(remote: branch.name, local: local)
            }
            await refresh()
        }
        ToastCenter.shared.show(
            existing != nil ? "Switched to \(local)" : "Created \(local) tracking \(branch.name)",
            style: .success)
    }

    @MainActor func createBranch(_ name: String) async throws {
        try await gitService.createBranch(name: name); await refresh()
        ToastCenter.shared.show("Created branch \(name)", style: .success)
    }
    @MainActor func deleteBranch(_ name: String, force: Bool = false) async throws {
        try await gitService.deleteBranch(name: name, force: force); await refreshBranches()
        ToastCenter.shared.show("Deleted branch \(name)", style: .info)
    }

    /// 删除分支的「有反馈」版本。`git branch -d` 在分支未完全合并时会失败 —— 视图层过去用
    /// `try?` 吞掉了错误,点 Delete 完全没反应。现在:未合并 → 交回给视图弹二次确认(强删),
    /// 其它失败 → 红 toast。
    @MainActor
    func deleteBranchChecked(_ name: String, force: Bool = false) async -> BranchDeleteOutcome {
        do {
            try await deleteBranch(name, force: force)
            return .deleted
        } catch {
            let msg = error.localizedDescription.lowercased()
            if !force, msg.contains("not fully merged") {
                return .needsForce
            }
            showErr("Delete branch", error)
            return .failed
        }
    }

    /// 跑一个可能抛错的 git 动作,失败时弹红 toast。视图里凡是 `try? await repo.xxx()`
    /// 都该换成这个 —— 否则 git 拒绝时按钮看起来像坏了(切分支有本地改动、建分支名重复等)。
    @MainActor
    @discardableResult
    func run(_ label: String, _ op: () async throws -> Void) async -> Bool {
        do { try await op(); return true } catch { showErr(label, error); return false }
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
    /// 把规则写进 .gitignore。`trackedFile` 非空表示这条规则是从一个「已跟踪」的文件上发起的 ——
    /// 此时 .gitignore 不会生效(git 只忽略未跟踪的文件),要额外提醒用户去「Stop Tracking」。
    func addToGitignore(_ pattern: String, trackedFile: String? = nil) async {
        do {
            try gitService.appendToGitignore(pattern)
            await refreshStatus()
            if let trackedFile {
                let name = (trackedFile as NSString).lastPathComponent
                ToastCenter.shared.show(
                    "Added \(pattern) to .gitignore — but \(name) is already tracked, so it keeps showing up. Use “Stop Tracking” on it.",
                    style: .info)
            } else {
                ToastCenter.shared.show("Added to .gitignore: \(pattern)", style: .success)
            }
        } catch { showErr("Update .gitignore", error) }
    }

    /// git rm --cached:停止跟踪但保留本地文件。留下一个「已暂存的删除」,提交后才真正生效。
    @MainActor
    func untrackFile(_ path: String) async {
        do {
            try await gitService.untrack(path)
            await refresh()
            let name = (path as NSString).lastPathComponent
            ToastCenter.shared.show("Stopped tracking \(name) — commit the staged removal to finish", style: .success)
        } catch { showErr("Stop tracking", error) }
    }

    /// 文件在磁盘上的绝对路径(复制路径 / 打开用)。
    func absolutePath(of path: String) -> String {
        guard let repoPath = gitService.repoPath else { return path }
        return (repoPath as NSString).appendingPathComponent(path)
    }

    // MARK: - Errors

    var lastError: String?
    var showError = false
    @MainActor private func showErr(_ ctx: String, _ error: Error) {
        ToastCenter.shared.show("\(ctx) failed: \(error.localizedDescription)", style: .error)
        print("[GitPilot] \(ctx) error: \(error)")
    }
}

enum BranchDeleteOutcome { case deleted, needsForce, failed }

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
