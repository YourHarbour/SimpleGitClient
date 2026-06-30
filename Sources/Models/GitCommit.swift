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
