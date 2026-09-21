import Foundation

struct GitCommit: Identifiable, Hashable {
    let id: String
    let shortHash: String
    let message: String
    let body: String
    let author: String
    let authorEmail: String
    let date: Date
    let parentHashes: [String]
    let refs: [GitRef]
    
    var isMerge: Bool { parentHashes.count > 1 }
    
    /// 完整提交信息(标题 + 空行 + 正文)—— 复制时用。
    var fullMessage: String {
        body.isEmpty ? message : message + "\n\n" + body
    }

    /// 「Copy Commit Info」用的多行摘要,格式贴近 `git show --stat` 的头部。
    var copyableSummary: String {
        var out = "commit \(id)\n"
        out += "Author: \(author) <\(authorEmail)>\n"
        out += "Date:   \(date.formatted(date: .abbreviated, time: .standard))\n\n"
        out += fullMessage
        return out
    }

    var truncatedBody: String {
        guard !body.isEmpty else { return "" }
        let firstLine = body.components(separatedBy: .newlines).first ?? body
        if firstLine.count > 80 {
            return String(firstLine.prefix(80)) + "\u{2026}"
        }
        return firstLine
    }
}

struct GitRef: Identifiable, Hashable {
    let id: String
    let name: String
    let type: RefType
    let isHead: Bool
    
    enum RefType: String, Hashable {
        case localBranch
        case remoteBranch
        case tag
    }
}
