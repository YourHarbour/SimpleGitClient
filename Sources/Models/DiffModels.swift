import Foundation

struct DiffFile: Identifiable {
    let id = UUID()
    let oldPath: String?
    let newPath: String?
    let hunks: [DiffHunk]
    let isBinary: Bool
    let fileStatus: FileChangeType
    
    var displayPath: String {
        newPath ?? oldPath ?? "unknown"
    }
    
    var linesAdded: Int {
        hunks.flatMap { $0.lines }.filter { $0.type == .added }.count
    }
    
    var linesRemoved: Int {
        hunks.flatMap { $0.lines }.filter { $0.type == .removed }.count
    }
}

struct DiffHunk: Identifiable {
    let id = UUID()
    let header: String
    let oldStart: Int
    let oldCount: Int
    let newStart: Int
    let newCount: Int
    let lines: [DiffLine]
}

struct DiffLine: Identifiable {
    let id = UUID()
    let type: DiffLineType
    let content: String
    let oldLineNumber: Int?
    let newLineNumber: Int?
}

enum DiffLineType: Hashable {
    case context
    case added
    case removed
    case hunkHeader
}
