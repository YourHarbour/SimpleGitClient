import Foundation
import SwiftUI

struct GitFileStatus: Identifiable, Hashable {
    var id: String { (isStaged ? "staged-" : "unstaged-") + path }
    let path: String
    let status: FileChangeType
    var isStaged: Bool
    
    var fileName: String {
        (path as NSString).lastPathComponent
    }
    
    var directory: String {
        let dir = (path as NSString).deletingLastPathComponent
        return dir.isEmpty ? "" : dir + "/"
    }
}

enum FileChangeType: String, Hashable {
    case added = "A"
    case modified = "M"
    case deleted = "D"
    case renamed = "R"
    case copied = "C"
    case untracked = "?"
    case unmerged = "U"
    
    var symbol: String {
        switch self {
        case .added, .untracked: return "+"
        case .modified: return "\u{00b1}"
        case .deleted: return "\u{2212}"
        case .renamed: return "\u{2192}"
        case .copied: return "\u{2295}"
        case .unmerged: return "!"
        }
    }
    
    var color: Color {
        switch self {
        case .added, .untracked: return Theme.accentGreen
        case .modified: return Color(hex: "#e6994a")
        case .deleted: return Theme.accentRed
        case .renamed: return Theme.accentBlue
        case .copied: return Theme.accentBlue
        case .unmerged: return Theme.accentRed
        }
    }
}
