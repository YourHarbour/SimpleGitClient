import SwiftUI
import Observation

struct RepoTab: Identifiable {
    let id = UUID()
    var path: String?              // nil = 落地页 / New Tab
    var name: String               // "New Tab" 或仓库名
    var viewModel: RepoViewModel?  // nil = 落地页
}

@Observable
class AppViewModel {
    var tabs: [RepoTab] = []
    var activeTabId: UUID?

    // 弹窗 / 面板状态
    var showCredentialsSheet = false   // ⚙️ 设置 → 凭据
    var showCloneSheet = false
    var showCreateBranchSheet = false
    var sidebarCollapsed = false

    // 鉴权(push / clone 共用)
    enum AuthContext: Equatable { case push; case clone(url: String, directory: String, name: String?) }
    var showAuthSheet = false
    var authHost = ""
    var authUsername = "git"
    var authContext: AuthContext = .push

    // 最近仓库(持久化)
    var recentRepos: [String] = []
    private let recentKey = "GitPilot.recentRepos"

    // 打开的 tab(持久化,用于重启后恢复)
    private let openTabsKey = "SimpleGitClient.openTabs"
    private let activeTabPathKey = "SimpleGitClient.activeTabPath"

    var errorMessage: String?
    var showError = false

    var activeTab: RepoTab? {
        tabs.first(where: { $0.id == activeTabId }) ?? tabs.first
    }
    var activeRepo: RepoViewModel? { activeTab?.viewModel }

    init() {
        recentRepos = UserDefaults.standard.stringArray(forKey: recentKey) ?? []
        newTab()   // 启动即一个 New Tab 落地页
    }

    // MARK: - Tabs

    func newTab() {
        // 注意:不在这里 persistTabs() —— New Tab 无路径,会把已保存的 openTabs 覆盖成空,
        // 导致下次启动无法恢复。落地页本身也不需要被恢复。
        let tab = RepoTab(path: nil, name: "New Tab", viewModel: nil)
        tabs.append(tab)
        activeTabId = tab.id
    }

    func selectTab(_ id: UUID) { activeTabId = id; persistTabs() }

    func closeTab(_ id: UUID) {
        guard let index = tabs.firstIndex(where: { $0.id == id }) else { return }
        tabs[index].viewModel?.cleanup()
        tabs.remove(at: index)
        if tabs.isEmpty { newTab() }
        else if activeTabId == id { activeTabId = tabs.last?.id }
        persistTabs()
    }

    /// 保存当前打开的仓库 tab 与激活 tab(供下次启动恢复)。
    private func persistTabs() {
        let paths = tabs.compactMap { $0.path }
        UserDefaults.standard.set(paths, forKey: openTabsKey)
        UserDefaults.standard.set(activeTab?.path ?? "", forKey: activeTabPathKey)
    }

    func closeAllOtherTabs(except id: UUID) {
        for closeId in tabs.filter({ $0.id != id }).map(\.id) { closeTab(closeId) }
    }

    // MARK: - Open / Clone

    func bootstrap() async {
        // 1) 显式 --open / 环境变量优先(测试 / 复现用)
        var explicit: String?
        let args = CommandLine.arguments
        if let idx = args.firstIndex(of: "--open"), idx + 1 < args.count { explicit = args[idx + 1] }
        if explicit == nil, let env = ProcessInfo.processInfo.environment["SIMPLEGITCLIENT_OPEN"], !env.isEmpty {
            explicit = env
        }
        if let explicit, !explicit.isEmpty { await openRepository(at: explicit); return }

        // 2) 恢复上次打开的 tab(过滤掉已失效的仓库,避免启动时报错)
        let saved = UserDefaults.standard.stringArray(forKey: openTabsKey) ?? []
        for path in saved where await GitService(repoPath: path).isGitRepository(at: path) {
            await openRepository(at: path)
        }
        if let activePath = UserDefaults.standard.string(forKey: activeTabPathKey),
           !activePath.isEmpty, let tab = tabs.first(where: { $0.path == activePath }) {
            activeTabId = tab.id
        }
        if tabs.isEmpty { newTab() }   // 没有可恢复的 → 保留一个 New Tab
    }

    func openRepository(at path: String) async {
        let git = GitService(repoPath: path)
        guard await git.isGitRepository(at: path) else {
            showErrorMessage("'\(path)' is not a Git repository.")
            return
        }
        if let existing = tabs.first(where: { $0.path == path }) {
            activeTabId = existing.id
            persistTabs()
            return
        }
        let name = (path as NSString).lastPathComponent
        let repoVM = RepoViewModel(gitService: git)
        await git.ensureCredentialHelper()   // 让 push 能记住 token

        // 填充当前空白(落地)tab,否则新开 tab
        if let idx = tabs.firstIndex(where: { $0.id == activeTabId }), tabs[idx].viewModel == nil {
            tabs[idx].path = path
            tabs[idx].name = name
            tabs[idx].viewModel = repoVM
        } else {
            let tab = RepoTab(path: path, name: name, viewModel: repoVM)
            tabs.append(tab)
            activeTabId = tab.id
        }
        addRecent(path)
        persistTabs()
        await repoVM.refresh()
    }

    func showOpenDialog() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.message = "Select a Git repository folder"
        panel.prompt = "Open Repository"
        if panel.runModal() == .OK, let url = panel.url {
            Task { await openRepository(at: url.path) }
        }
    }

    /// 默认文件夹名:用户给的 name 优先,否则从 URL 末段推导(去掉 .git)。
    static func cloneFolderName(url: String, name: String?) -> String {
        if let name, !name.trimmingCharacters(in: .whitespaces).isEmpty {
            return name.trimmingCharacters(in: .whitespaces)
        }
        var folder = (url as NSString).lastPathComponent
        if folder.hasSuffix(".git") { folder = String(folder.dropLast(4)) }
        return folder
    }

    func cloneRepository(url: String, to directory: String, name: String? = nil) async {
        let trimmed = url.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let folder = Self.cloneFolderName(url: trimmed, name: name)
        let target = (directory as NSString).appendingPathComponent(folder)
        let git = GitService(repoPath: nil)
        ToastCenter.shared.show("Cloning \(folder)…", style: .info)
        do {
            try await git.clone(url: trimmed, to: target)
            await openRepository(at: target)
            ToastCenter.shared.show("Cloned \(folder)", style: .success)
        } catch GitError.authenticationRequired {
            // 需要 token —— 弹鉴权框,存好后自动重试 clone
            let parsed = GitService.parseRemote(trimmed)
            authHost = parsed.host
            authUsername = parsed.username ?? "git"
            if let cred = try? KeychainService.shared.getCredential(host: parsed.host) {
                authUsername = cred.username
            }
            authContext = .clone(url: trimmed, directory: directory, name: name)
            showAuthSheet = true
        } catch {
            showErrorMessage("Clone failed: \(error.localizedDescription)")
        }
    }

    // MARK: - Recent

    private func addRecent(_ path: String) {
        recentRepos.removeAll { $0 == path }
        recentRepos.insert(path, at: 0)
        if recentRepos.count > 12 { recentRepos = Array(recentRepos.prefix(12)) }
        UserDefaults.standard.set(recentRepos, forKey: recentKey)
    }

    func removeRecent(_ path: String) {
        recentRepos.removeAll { $0 == path }
        UserDefaults.standard.set(recentRepos, forKey: recentKey)
    }

    // MARK: - Auth (push / clone)

    func beginPushAuth(for repo: RepoViewModel) async {
        authContext = .push
        if let info = await repo.remoteHostInfo() {
            authHost = info.host
            authUsername = info.username
            if let cred = try? KeychainService.shared.getCredential(host: info.host) {
                authUsername = cred.username
            }
        }
        showAuthSheet = true
    }

    func submitAuth(username: String, token: String, remember: Bool) async {
        let host = authHost
        switch authContext {
        case .push:
            guard let repo = activeRepo else { showAuthSheet = false; return }
            do {
                try await repo.storeTokenAndPush(host: host, username: username, token: token, remember: remember)
                showAuthSheet = false
            } catch {
                showErrorMessage("Push failed: \(error.localizedDescription)")
            }
        case .clone(let url, let directory, let name):
            do {
                try await GitService().approveCredentialGlobal(host: host, username: username, token: token)
                if remember {
                    try? KeychainService.shared.saveCredential(
                        GitCredential(host: host, username: username, token: token, createdAt: Date()))
                }
                showAuthSheet = false
                await cloneRepository(url: url, to: directory, name: name)   // 重试,凭据已就绪
            } catch {
                showErrorMessage("Saving token failed: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Errors

    func showErrorMessage(_ message: String) {
        errorMessage = message
        showError = true
    }
}
