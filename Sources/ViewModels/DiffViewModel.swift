import SwiftUI
import Observation

@Observable
class DiffViewModel {
    var diffFile: DiffFile?
    var fileContent: String?      // File View 全文
    var isLoading = false
    var viewMode: DiffViewMode = .diff
    var isStaged = false
    var filePath: String = ""
    var error: String?

    // Diff 工具栏开关
    var wrapLines = false
    var showWhitespace = false

    @MainActor
    func loadDiff(gitService: GitService, file: String, staged: Bool) async {
        isLoading = true
        filePath = file
        isStaged = staged
        error = nil
        defer { isLoading = false }
        do {
            let rawDiff = try await gitService.getDiff(file: file, staged: staged)
            if rawDiff.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
               let content = try? gitService.readWorkingFile(file) {
                // 未跟踪 / intent-to-add 文件:git diff 为空,合成"全部新增"diff
                diffFile = DiffParser.synthesizeAdded(content, filePath: file)
            } else {
                diffFile = DiffParser.parse(rawDiff, filePath: file)
            }
            fileContent = try? gitService.readWorkingFile(file)
        } catch {
            self.error = error.localizedDescription
            diffFile = nil
        }
    }

    @MainActor
    func loadCommitDiff(gitService: GitService, hash: String, file: String) async {
        isLoading = true
        filePath = file
        error = nil
        defer { isLoading = false }
        do {
            let rawDiff = try await gitService.getFileDiffForCommit(hash: hash, file: file)
            diffFile = DiffParser.parse(rawDiff, filePath: file)
            fileContent = try? await gitService.getFileAtCommit(hash: hash, file: file)
        } catch {
            self.error = error.localizedDescription
            diffFile = nil
        }
    }
}

enum DiffViewMode { case file, diff }

enum DiffParser {
    static func parse(_ raw: String, filePath: String) -> DiffFile {
        guard !raw.isEmpty else {
            return DiffFile(oldPath: filePath, newPath: filePath, hunks: [], isBinary: false, fileStatus: .modified)
        }
        if raw.contains("Binary files") {
            return DiffFile(oldPath: filePath, newPath: filePath, hunks: [], isBinary: true, fileStatus: .modified)
        }

        let lines = raw.components(separatedBy: "\n")
        var hunks: [DiffHunk] = []
        var currentHunkLines: [DiffLine] = []
        var currentHeader = ""
        var oldStart = 0, oldCount = 0, newStart = 0, newCount = 0
        var oldLine = 0, newLine = 0
        var inHunk = false

        var fileStatus: FileChangeType = .modified
        if raw.contains("new file mode") { fileStatus = .added }
        else if raw.contains("deleted file mode") { fileStatus = .deleted }
        else if raw.contains("rename from") { fileStatus = .renamed }

        func flush() {
            if inHunk && !currentHunkLines.isEmpty {
                hunks.append(DiffHunk(header: currentHeader, oldStart: oldStart, oldCount: oldCount,
                                      newStart: newStart, newCount: newCount, lines: currentHunkLines))
            }
        }

        for line in lines {
            if line.hasPrefix("@@") {
                flush()
                // 仅保留 "@@ -x,y +a,b @@" 部分,去掉尾部函数上下文
                if let close = line.range(of: " @@") {
                    currentHeader = String(line[..<close.upperBound])
                } else {
                    currentHeader = line
                }
                currentHunkLines = []
                let regex = try? NSRegularExpression(pattern: #"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@"#)
                if let match = regex?.firstMatch(in: line, range: NSRange(line.startIndex..., in: line)) {
                    oldStart = Int((line as NSString).substring(with: match.range(at: 1))) ?? 0
                    oldCount = match.range(at: 2).location != NSNotFound ? Int((line as NSString).substring(with: match.range(at: 2))) ?? 1 : 1
                    newStart = Int((line as NSString).substring(with: match.range(at: 3))) ?? 0
                    newCount = match.range(at: 4).location != NSNotFound ? Int((line as NSString).substring(with: match.range(at: 4))) ?? 1 : 1
                }
                oldLine = oldStart
                newLine = newStart
                inHunk = true
            } else if inHunk {
                if line.hasPrefix("+") {
                    currentHunkLines.append(DiffLine(type: .added, content: String(line.dropFirst()), oldLineNumber: nil, newLineNumber: newLine))
                    newLine += 1
                } else if line.hasPrefix("-") {
                    currentHunkLines.append(DiffLine(type: .removed, content: String(line.dropFirst()), oldLineNumber: oldLine, newLineNumber: nil))
                    oldLine += 1
                } else if line.hasPrefix(" ") {
                    currentHunkLines.append(DiffLine(type: .context, content: String(line.dropFirst()), oldLineNumber: oldLine, newLineNumber: newLine))
                    oldLine += 1
                    newLine += 1
                } else if line.hasPrefix("\\") {
                    // "\ No newline at end of file" — 忽略
                }
            }
        }
        flush()
        return DiffFile(oldPath: filePath, newPath: filePath, hunks: hunks, isBinary: false, fileStatus: fileStatus)
    }

    /// 为未跟踪文件合成一份"全部新增"的 diff。
    static func synthesizeAdded(_ content: String, filePath: String) -> DiffFile {
        var lines = content.components(separatedBy: "\n")
        if lines.last == "" { lines.removeLast() }   // 去掉末尾换行产生的空元素
        let diffLines = lines.enumerated().map { idx, text in
            DiffLine(type: .added, content: text, oldLineNumber: nil, newLineNumber: idx + 1)
        }
        let header = "@@ -0,0 +1,\(lines.count) @@"
        let hunk = DiffHunk(header: header, oldStart: 0, oldCount: 0, newStart: 1, newCount: lines.count, lines: diffLines)
        return DiffFile(oldPath: filePath, newPath: filePath, hunks: lines.isEmpty ? [] : [hunk], isBinary: false, fileStatus: .added)
    }
}
