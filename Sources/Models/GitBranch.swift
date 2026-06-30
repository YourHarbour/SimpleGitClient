import Foundation

struct GitBranch: Identifiable, Hashable {
    var id: String { name }
    let name: String
    let isLocal: Bool
    let isRemote: Bool
    let isCurrent: Bool
    let trackingBranch: String?
    let lastCommitHash: String?
    let lastCommitMessage: String?
    
    var displayName: String {
        if isRemote {
            let parts = name.split(separator: "/", maxSplits: 1)
            if parts.count > 1 {
                return String(parts[1])
            }
        }
        return name
    }
    
    var remoteName: String? {
        guard isRemote else { return nil }
        let parts = name.split(separator: "/", maxSplits: 1)
        return parts.count > 1 ? String(parts[0]) : nil
    }
}

struct GitTag: Identifiable, Hashable {
    var id: String { name }
    let name: String
    let commitHash: String
    let message: String?
    let isAnnotated: Bool
}

struct GitWorktree: Identifiable, Hashable {
    var id: String { path }
    let path: String
    let branch: String?
    let isMain: Bool

    var name: String { (path as NSString).lastPathComponent }
}
