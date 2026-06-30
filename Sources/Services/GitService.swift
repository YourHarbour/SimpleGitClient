import Foundation
import Observation

enum GitError: LocalizedError {
    case notARepository
    case commandFailed(String)
    case parseError(String)
    case authenticationRequired
    case networkError(String)
    
    var errorDescription: String? {
        switch self {
        case .notARepository: return "Not a git repository"
        case .commandFailed(let msg): return "Git command failed: \(msg)"
        case .parseError(let msg): return "Failed to parse git output: \(msg)"
        case .authenticationRequired: return "Authentication required"
        case .networkError(let msg): return "Network error: \(msg)"
        }
    }
}

@Observable
class GitService {
    private(set) var repoPath: String?
    private(set) var isLoading = false
    
    init(repoPath: String? = nil) {
        self.repoPath = repoPath
    }
    
    func execute(_ arguments: [String], at path: String? = nil) async throws -> String {
        let workDir = path ?? repoPath
        guard let workDir else {
            throw GitError.notARepository
        }

        return try await withCheckedThrowingContinuation { continuation in
            // 整个进程的运行 + 读管道放到后台队列,避免阻塞调用线程。
            DispatchQueue.global(qos: .userInitiated).async {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
                process.arguments = arguments
                process.currentDirectoryURL = URL(fileURLWithPath: workDir)

                let stdoutPipe = Pipe()
                let stderrPipe = Pipe()
                process.standardOutput = stdoutPipe
                process.standardError = stderrPipe
                process.standardInput = FileHandle.nullDevice

                var env = ProcessInfo.processInfo.environment
                env["GIT_TERMINAL_PROMPT"] = "0"
                env["LC_ALL"] = "en_US.UTF-8"
                process.environment = env

                do {
                    try process.run()
                } catch {
                    continuation.resume(throwing: GitError.commandFailed(error.localizedDescription))
                    return
                }

                // 关键:并发读取 stdout/stderr。若等进程退出后再读,大输出(> 管道 64KB)
                // 会写满管道使进程阻塞、永不退出 → 死锁。
                var stdoutData = Data()
                var stderrData = Data()
                let group = DispatchGroup()
                let ioQueue = DispatchQueue(label: "simplegitclient.git.io", attributes: .concurrent)
                group.enter()
                ioQueue.async { stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile(); group.leave() }
                group.enter()
                ioQueue.async { stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile(); group.leave() }
                process.waitUntilExit()
                group.wait()

                let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
                let stderr = String(data: stderrData, encoding: .utf8) ?? ""

                if process.terminationStatus == 0 {
                    continuation.resume(returning: stdout)
                } else {
                    let lower = stderr.lowercased()
                    if lower.contains("authentication") || lower.contains("could not read username")
                        || lower.contains("could not read password") || lower.contains("invalid username or password")
                        || lower.contains("403") {
                        continuation.resume(throwing: GitError.authenticationRequired)
                    } else if stderr.contains("Could not resolve host") || stderr.contains("unable to access") {
                        continuation.resume(throwing: GitError.networkError(stderr.trimmingCharacters(in: .whitespacesAndNewlines)))
                    } else {
                        continuation.resume(throwing: GitError.commandFailed(stderr.trimmingCharacters(in: .whitespacesAndNewlines)))
                    }
                }
            }
        }
    }
    
    func setRepoPath(_ path: String) {
        self.repoPath = path
    }

    /// 像 execute 一样运行 git,但把 `input` 写入 stdin(用于 `git credential approve` 等)。
    func executeWithStdin(_ arguments: [String], input: String, at path: String? = nil) async throws -> String {
        let workDir = path ?? repoPath
        guard let workDir else { throw GitError.notARepository }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = arguments
        process.currentDirectoryURL = URL(fileURLWithPath: workDir)

        let stdinPipe = Pipe(), stdoutPipe = Pipe(), stderrPipe = Pipe()
        process.standardInput = stdinPipe
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        var env = ProcessInfo.processInfo.environment
        env["GIT_TERMINAL_PROMPT"] = "0"
        env["LC_ALL"] = "en_US.UTF-8"
        process.environment = env

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    try process.run()
                    if let data = input.data(using: .utf8) {
                        stdinPipe.fileHandleForWriting.write(data)
                    }
                    stdinPipe.fileHandleForWriting.closeFile()
                } catch {
                    continuation.resume(throwing: GitError.commandFailed(error.localizedDescription))
                    return
                }

                var outData = Data(); var errData = Data()
                let group = DispatchGroup()
                let ioQueue = DispatchQueue(label: "simplegitclient.git.io2", attributes: .concurrent)
                group.enter(); ioQueue.async { outData = stdoutPipe.fileHandleForReading.readDataToEndOfFile(); group.leave() }
                group.enter(); ioQueue.async { errData = stderrPipe.fileHandleForReading.readDataToEndOfFile(); group.leave() }
                process.waitUntilExit()
                group.wait()

                let out = String(data: outData, encoding: .utf8) ?? ""
                let err = String(data: errData, encoding: .utf8) ?? ""
                if process.terminationStatus == 0 {
                    continuation.resume(returning: out)
                } else {
                    continuation.resume(throwing: GitError.commandFailed(err.trimmingCharacters(in: .whitespacesAndNewlines)))
                }
            }
        }
    }
    
    func isGitRepository(at path: String) async -> Bool {
        do {
            _ = try await execute(["rev-parse", "--git-dir"], at: path)
            return true
        } catch {
            return false
        }
    }
    
    func getRepoName() async -> String {
        guard let path = repoPath else { return "Unknown" }
        return (path as NSString).lastPathComponent
    }
    
    // MARK: - Status
    
    func getStatus() async throws -> [GitFileStatus] {
        let output = try await execute(["status", "--porcelain=v2", "--untracked-files=all"])
        return parseStatusV2(output)
    }
    
    private func parseStatusV2(_ output: String) -> [GitFileStatus] {
        var files: [GitFileStatus] = []
        let lines = output.components(separatedBy: "\n").filter { !$0.isEmpty }
        
        for line in lines {
            if line.hasPrefix("1 ") {
                let parts = line.split(separator: " ", maxSplits: 8)
                guard parts.count >= 9 else { continue }
                let xy = String(parts[1])
                let path = String(parts[8])
                let indexStatus = xy.first ?? "."
                let workTreeStatus = xy.last ?? "."
                
                if indexStatus != "." && indexStatus != "?" {
                    let status = parseChangeType(indexStatus)
                    files.append(GitFileStatus(path: path, status: status, isStaged: true))
                }
                
                if workTreeStatus != "." && workTreeStatus != "?" {
                    let status = parseChangeType(workTreeStatus)
                    if !files.contains(where: { $0.path == path && !$0.isStaged }) {
                        files.append(GitFileStatus(path: path, status: status, isStaged: false))
                    }
                }
            } else if line.hasPrefix("? ") {
                let path = String(line.dropFirst(2))
                files.append(GitFileStatus(path: path, status: .untracked, isStaged: false))
            } else if line.hasPrefix("2 ") {
                let parts = line.split(separator: " ", maxSplits: 9)
                guard parts.count >= 10 else { continue }
                let pathPart = String(parts[9])
                let paths = pathPart.split(separator: "\t")
                let newPath = paths.count > 0 ? String(paths[0]) : pathPart
                let xy = String(parts[1])
                let indexStatus = xy.first ?? "."
                if indexStatus == "R" {
                    files.append(GitFileStatus(path: newPath, status: .renamed, isStaged: true))
                } else if indexStatus == "C" {
                    files.append(GitFileStatus(path: newPath, status: .copied, isStaged: true))
                }
            } else if line.hasPrefix("u ") {
                let parts = line.split(separator: " ", maxSplits: 10)
                guard parts.count >= 11 else { continue }
                let path = String(parts[10])
                files.append(GitFileStatus(path: path, status: .unmerged, isStaged: false))
            }
        }
        return files
    }
    
    private func parseChangeType(_ char: Character) -> FileChangeType {
        switch char {
        case "A": return .added
        case "M": return .modified
        case "D": return .deleted
        case "R": return .renamed
        case "C": return .copied
        case "?": return .untracked
        case "U": return .unmerged
        default: return .modified
        }
    }
    
    // MARK: - Log
    
    func getLog(maxCount: Int = 100) async throws -> [GitCommit] {
        let format = "%H%n%h%n%an%n%ae%n%aI%n%P%n%D%n%s%n%b%n---END---"
        // --topo-order: 保证父提交永远排在子提交之后(lane 分配算法的前提),
        // 同时让分支不会交错显示。
        let output = try await execute([
            "log", "--format=\(format)", "--max-count=\(maxCount)", "--all",
            "--topo-order"
        ])
        return parseLog(output)
    }
    
    private func parseLog(_ output: String) -> [GitCommit] {
        var commits: [GitCommit] = []
        let blocks = output.components(separatedBy: "---END---")
        let dateFormatter = ISO8601DateFormatter()
        dateFormatter.formatOptions = [.withInternetDateTime]
        
        for block in blocks {
            let trimmed = block.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let lines = trimmed.components(separatedBy: "\n")
            guard lines.count >= 8 else { continue }
            
            let hash = lines[0]
            let shortHash = lines[1]
            let author = lines[2]
            let email = lines[3]
            let dateStr = lines[4]
            let parentStr = lines[5]
            let refsStr = lines[6]
            let message = lines[7]
            let body = lines.count > 8 ? lines[8...].joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines) : ""
            
            let date = dateFormatter.date(from: dateStr) ?? Date()
            let parents = parentStr.isEmpty ? [] : parentStr.split(separator: " ").map(String.init)
            let refs = parseRefs(refsStr)
            
            commits.append(GitCommit(
                id: hash, shortHash: shortHash, message: message, body: body,
                author: author, authorEmail: email, date: date,
                parentHashes: parents, refs: refs
            ))
        }
        return commits
    }
    
    private func parseRefs(_ refsStr: String) -> [GitRef] {
        guard !refsStr.isEmpty else { return [] }
        var refs: [GitRef] = []
        let parts = refsStr.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        for part in parts {
            var name = part
            var isHead = false
            if name.hasPrefix("HEAD -> ") {
                name = String(name.dropFirst(8))
                isHead = true
            }
            if name == "HEAD" { continue }
            let type: GitRef.RefType
            if name.hasPrefix("tag: ") {
                name = String(name.dropFirst(5))
                type = .tag
            } else if name.contains("/") {
                type = .remoteBranch
            } else {
                type = .localBranch
            }
            refs.append(GitRef(id: name, name: name, type: type, isHead: isHead))
        }
        return refs
    }
    
    // MARK: - Branches
    
    func getBranches() async throws -> [GitBranch] {
        let output = try await execute(["branch", "-a", "--format=%(refname:short)\t%(objectname:short)\t%(subject)\t%(upstream:short)\t%(HEAD)"])
        var branches: [GitBranch] = []
        for line in output.components(separatedBy: "\n") where !line.isEmpty {
            let parts = line.split(separator: "\t", maxSplits: 4).map(String.init)
            guard !parts.isEmpty else { continue }
            let name = parts[0]
            let hash = parts.count > 1 ? parts[1] : nil
            let msg = parts.count > 2 ? parts[2] : nil
            let tracking = parts.count > 3 && !parts[3].isEmpty ? parts[3] : nil
            let isCurrent = parts.count > 4 && parts[4].contains("*")
            let isRemote = name.hasPrefix("origin/") || name.contains("/")
            branches.append(GitBranch(
                name: name, isLocal: !isRemote, isRemote: isRemote,
                isCurrent: isCurrent, trackingBranch: tracking,
                lastCommitHash: hash, lastCommitMessage: msg
            ))
        }
        return branches
    }
    
    func getCurrentBranch() async throws -> String {
        let output = try await execute(["branch", "--show-current"])
        return output.trimmingCharacters(in: .whitespacesAndNewlines)
    }
    
    func checkout(branch: String) async throws {
        _ = try await execute(["switch", branch])
    }
    
    func createBranch(name: String) async throws {
        _ = try await execute(["switch", "-c", name])
    }
    
    func deleteBranch(name: String, force: Bool = false) async throws {
        _ = try await execute(["branch", force ? "-D" : "-d", name])
    }
    
    // MARK: - Staging
    
    func stageFile(_ path: String) async throws {
        _ = try await execute(["add", "--", path])
    }
    
    func unstageFile(_ path: String) async throws {
        _ = try await execute(["restore", "--staged", "--", path])
    }
    
    func stageAll() async throws {
        _ = try await execute(["add", "-A"])
    }
    
    func unstageAll() async throws {
        _ = try await execute(["reset", "HEAD"])
    }
    
    func discardAllChanges() async throws {
        _ = try await execute(["checkout", "--", "."])
    }

    /// 丢弃单个文件改动:已跟踪文件还原到 HEAD,未跟踪文件直接删除。
    func discardFile(_ path: String) async throws {
        _ = try? await execute(["restore", "--staged", "--", path])
        do {
            _ = try await execute(["restore", "--", path])
        } catch {
            _ = try await execute(["clean", "-fdq", "--", path])
        }
    }

    /// 删除所有未跟踪文件/目录(配合 discardAllChanges 实现"丢弃全部")。
    func cleanUntracked() async throws {
        _ = try await execute(["clean", "-fdq"])
    }

    /// 往仓库根的 .gitignore 追加一条规则(已存在则跳过;文件不存在则创建)。
    func appendToGitignore(_ pattern: String) throws {
        guard let repoPath else { throw GitError.notARepository }
        let trimmed = pattern.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        let ignorePath = (repoPath as NSString).appendingPathComponent(".gitignore")
        var content = (try? String(contentsOfFile: ignorePath, encoding: .utf8)) ?? ""
        // 去重(忽略首尾空白)
        let existing = content.components(separatedBy: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
        if existing.contains(trimmed) { return }
        if !content.isEmpty && !content.hasSuffix("\n") { content += "\n" }
        content += trimmed + "\n"
        try content.write(toFile: ignorePath, atomically: true, encoding: .utf8)
    }
    
    // MARK: - Diff
    
    func getDiff(file: String?, staged: Bool = false) async throws -> String {
        var args = ["diff"]
        if staged { args.append("--staged") }
        if let file = file { args.append(contentsOf: ["--", file]) }
        return try await execute(args)
    }
    
    func getDiffForCommit(hash: String) async throws -> String {
        return try await execute(["diff", "\(hash)~1", hash])
    }

    /// 读取工作区文件内容(用于未跟踪文件的 diff 合成 / File View)。
    func readWorkingFile(_ path: String) throws -> String {
        guard let repoPath else { throw GitError.notARepository }
        let full = (repoPath as NSString).appendingPathComponent(path)
        return try String(contentsOfFile: full, encoding: .utf8)
    }
    
    func getFileDiffForCommit(hash: String, file: String) async throws -> String {
        return try await execute(["diff", "\(hash)~1", hash, "--", file])
    }

    /// 某个提交里某文件的完整内容(用于提交详情的 File View)。
    func getFileAtCommit(hash: String, file: String) async throws -> String {
        return try await execute(["show", "\(hash):\(file)"])
    }
    
    func getChangedFilesForCommit(hash: String) async throws -> [GitFileStatus] {
        let output = try await execute(["diff-tree", "--no-commit-id", "-r", "--name-status", hash])
        var files: [GitFileStatus] = []
        for line in output.components(separatedBy: "\n") where !line.isEmpty {
            let parts = line.split(separator: "\t", maxSplits: 1)
            guard parts.count == 2 else { continue }
            let status = parseChangeType(parts[0].first ?? "M")
            files.append(GitFileStatus(path: String(parts[1]), status: status, isStaged: false))
        }
        return files
    }
    
    // MARK: - Commit
    
    func commit(message: String, amend: Bool = false, signOff: Bool = false, allowEmpty: Bool = false) async throws {
        var args = ["commit", "-m", message]
        if amend { args.append("--amend") }
        if signOff { args.append("--signoff") }
        if allowEmpty { args.append("--allow-empty") }
        _ = try await execute(args)
    }
    
    // MARK: - Remote
    
    func pull(rebase: Bool = false) async throws {
        var args = ["pull"]
        if rebase { args.append("--rebase") }
        _ = try await execute(args)
    }
    
    func push() async throws {
        _ = try await execute(["push"])
    }
    
    func pushSetUpstream(remote: String = "origin", branch: String) async throws {
        _ = try await execute(["push", "-u", remote, branch])
    }
    
    func fetch(all: Bool = true) async throws {
        var args = ["fetch"]
        if all { args.append("--all") }
        _ = try await execute(args)
    }
    
    func clone(url: String, to path: String) async throws {
        _ = try await execute(["clone", url, path], at: (path as NSString).deletingLastPathComponent)
    }

    // MARK: - Remote URL / Credentials

    func getRemoteURL(remote: String = "origin") async throws -> String {
        try await execute(["remote", "get-url", remote])
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// 从远程 URL 解析出 host 与(可能内嵌的)用户名。支持 https 与 scp 风格。
    static func parseRemote(_ url: String) -> (host: String, username: String?) {
        if let schemeRange = url.range(of: "://") {
            var rest = String(url[schemeRange.upperBound...])
            var user: String?
            if let at = rest.firstIndex(of: "@") {
                user = String(rest[..<at])
                rest = String(rest[rest.index(after: at)...])
            }
            let host = rest.split(whereSeparator: { $0 == "/" || $0 == ":" }).first.map(String.init) ?? rest
            return (host, user)
        } else if url.contains("@"), url.contains(":") {
            // git@host:path.git
            let parts = url.split(separator: "@", maxSplits: 1)
            let user = parts.count > 1 ? String(parts[0]) : nil
            let afterAt = String(parts.last ?? "")
            let host = afterAt.split(separator: ":").first.map(String.init) ?? afterAt
            return (host, user)
        }
        return (url, nil)
    }

    /// 确保当前仓库启用 osxkeychain 凭据助手(push 时由系统钥匙串自动提供 token)。
    func ensureCredentialHelper() async {
        _ = try? await execute(["config", "credential.helper", "osxkeychain"])
    }

    /// 把 token 写入系统钥匙串(`git credential approve`),之后 git 自身的 push/pull 即可自动鉴权。
    func approveCredential(host: String, username: String, token: String) async throws {
        await ensureCredentialHelper()
        let input = "protocol=https\nhost=\(host)\nusername=\(username)\npassword=\(token)\n\n"
        _ = try await executeWithStdin(["credential", "approve"], input: input)
    }

    /// 不依赖具体仓库地写入凭据(用于设置页直接添加 token)。使用全局 osxkeychain。
    func approveCredentialGlobal(host: String, username: String, token: String) async throws {
        let home = NSHomeDirectory()
        _ = try? await execute(["config", "--global", "credential.helper", "osxkeychain"], at: home)
        let input = "protocol=https\nhost=\(host)\nusername=\(username)\npassword=\(token)\n\n"
        _ = try await executeWithStdin(["credential", "approve"], input: input, at: home)
    }
    
    // MARK: - Stash
    
    func stash(message: String? = nil) async throws {
        var args = ["stash", "push"]
        if let message = message { args.append(contentsOf: ["-m", message]) }
        _ = try await execute(args)
    }
    
    func stashPop() async throws {
        _ = try await execute(["stash", "pop"])
    }
    
    func stashList() async throws -> [String] {
        let output = try await execute(["stash", "list"])
        return output.components(separatedBy: "\n").filter { !$0.isEmpty }
    }
    
    // MARK: - Worktrees

    func getWorktrees() async throws -> [GitWorktree] {
        let output = try await execute(["worktree", "list", "--porcelain"])
        var result: [GitWorktree] = []
        var path: String?
        var branch: String?
        func flush() {
            if let p = path {
                let ref = branch.map { $0.replacingOccurrences(of: "refs/heads/", with: "") }
                result.append(GitWorktree(path: p, branch: ref, isMain: result.isEmpty))
            }
            path = nil; branch = nil
        }
        for line in output.components(separatedBy: "\n") {
            if line.hasPrefix("worktree ") {
                flush()
                path = String(line.dropFirst("worktree ".count))
            } else if line.hasPrefix("branch ") {
                branch = String(line.dropFirst("branch ".count))
            } else if line.isEmpty {
                flush()
            }
        }
        flush()
        return result
    }

    // MARK: - Tags
    
    func getTags() async throws -> [GitTag] {
        let output = try await execute(["tag", "-l", "--format=%(refname:short)\t%(objectname:short)\t%(contents:subject)"])
        var tags: [GitTag] = []
        for line in output.components(separatedBy: "\n") where !line.isEmpty {
            let parts = line.split(separator: "\t", maxSplits: 2).map(String.init)
            guard parts.count >= 2 else { continue }
            tags.append(GitTag(
                name: parts[0], commitHash: parts[1],
                message: parts.count > 2 ? parts[2] : nil,
                isAnnotated: parts.count > 2 && !parts[2].isEmpty
            ))
        }
        return tags
    }
}
