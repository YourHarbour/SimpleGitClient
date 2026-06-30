import Foundation

/// 文件树节点 —— 用于 Staging 面板的 "Tree" 视图模式。
struct FileTreeNode: Identifiable {
    let id: String          // 完整路径(目录或文件)
    let name: String        // 该层显示名
    var file: GitFileStatus? // 非 nil 表示叶子文件
    var children: [FileTreeNode]
}

enum FileTree {
    static func build(from files: [GitFileStatus]) -> [FileTreeNode] {
        let root = Builder(name: "", path: "")
        for file in files.sorted(by: { $0.path < $1.path }) {
            let parts = file.path.split(separator: "/").map(String.init)
            root.insert(parts: parts, file: file, prefix: "")
        }
        return root.materialize().children
    }

    /// 构造期使用的引用类型,便于按路径分组。
    private final class Builder {
        let name: String
        let path: String
        var file: GitFileStatus?
        private var childMap: [String: Builder] = [:]
        private var childOrder: [String] = []

        init(name: String, path: String) {
            self.name = name
            self.path = path
        }

        func insert(parts: [String], file: GitFileStatus, prefix: String) {
            guard let head = parts.first else { return }
            let childPath = prefix.isEmpty ? head : prefix + "/" + head
            let child: Builder
            if let existing = childMap[head] {
                child = existing
            } else {
                child = Builder(name: head, path: childPath)
                childMap[head] = child
                childOrder.append(head)
            }
            if parts.count == 1 {
                child.file = file
            } else {
                child.insert(parts: Array(parts.dropFirst()), file: file, prefix: childPath)
            }
        }

        func materialize() -> FileTreeNode {
            // 目录在前、文件在后,各自字母序
            let nodes = childOrder.map { childMap[$0]!.materialize() }
            let sorted = nodes.sorted { a, b in
                let aDir = a.file == nil, bDir = b.file == nil
                if aDir != bDir { return aDir && !bDir }
                return a.name.localizedCaseInsensitiveCompare(b.name) == .orderedAscending
            }
            return FileTreeNode(id: path.isEmpty ? "__root__" : path,
                                name: name, file: file, children: sorted)
        }
    }
}
