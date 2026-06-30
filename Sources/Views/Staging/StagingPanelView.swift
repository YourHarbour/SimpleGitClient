import SwiftUI

/// 右侧暂存 / 提交面板 — spec §8
struct StagingPanelView: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var unstagedCollapsed = false
    @State private var stagedCollapsed = false
    @State private var showDiscardConfirm = false

    var body: some View {
        if let repo = appVM.activeRepo {
            VStack(spacing: 0) {
                panelHeader(repo)        // §8.1
                viewModeToolbar(repo)    // §8.2

                ScrollView {
                    LazyVStack(spacing: 0) {   // lazy:大量未跟踪文件时也流畅
                        // §8.3 Unstaged
                        fileSection(
                            title: "Unstaged Files",
                            files: repo.unstagedFiles,
                            isStaged: false,
                            collapsed: $unstagedCollapsed,
                            repo: repo
                        )
                        // §8.4 Staged
                        fileSection(
                            title: "Staged Files",
                            files: repo.stagedFiles,
                            isStaged: true,
                            collapsed: $stagedCollapsed,
                            repo: repo
                        )
                    }
                    .padding(.bottom, 8)
                }

                CommitComposerView()     // §8.5
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.bgPanel)
            .overlay(alignment: .leading) { Rectangle().fill(Theme.border).frame(width: 1) }
            .confirmationDialog(
                "Discard all changes?",
                isPresented: $showDiscardConfirm,
                titleVisibility: .visible
            ) {
                Button("Discard All Changes", role: .destructive) {
                    Task { await repo.discardAllChanges() }
                }
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This permanently deletes all uncommitted changes in the working tree. This cannot be undone.")
            }
        }
    }

    // MARK: - §8.1 Panel header

    private func panelHeader(_ repo: RepoViewModel) -> some View {
        HStack(spacing: 12) {
            Button(action: { showDiscardConfirm = true }) {
                Image(systemName: "trash")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.textPrimary)
                    .frame(width: 28, height: 28)
                    .background(RoundedRectangle(cornerRadius: 6).fill(Theme.accentRed))
            }
            .buttonStyle(.plain)
            .help("Discard all changes")
            .disabled(repo.totalChanges == 0)
            .opacity(repo.totalChanges == 0 ? 0.4 : 1)

            Spacer()

            HStack(spacing: 5) {
                Text("\(repo.totalChanges) file changes on")
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.textSecondary)
                Text(repo.currentBranch.isEmpty ? "main" : repo.currentBranch)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.accentTeal)
                    .padding(.horizontal, 7)
                    .padding(.vertical, 2)
                    .background(Capsule().fill(Theme.accentTeal.opacity(0.15)))
            }
        }
        .padding(.horizontal, 14)
        .frame(height: 48)
        .background(Theme.bgPanel)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
    }

    // MARK: - §8.2 View mode toolbar

    private func viewModeToolbar(_ repo: RepoViewModel) -> some View {
        HStack(spacing: 0) {
            Spacer()

            HStack(spacing: 0) {
                SegmentButton(title: "Path", icon: "text.alignleft",
                              active: repo.fileViewMode == .path) { repo.fileViewMode = .path }
                SegmentButton(title: "Tree", icon: "list.bullet.indent",
                              active: repo.fileViewMode == .tree) { repo.fileViewMode = .tree }
            }
            .padding(2)
            .background(RoundedRectangle(cornerRadius: 6).fill(Theme.bgApp))
        }
        .padding(.horizontal, 12)
        .frame(height: 36)
    }

    // MARK: - File section

    @ViewBuilder
    private func fileSection(title: String, files: [GitFileStatus], isStaged: Bool,
                             collapsed: Binding<Bool>, repo: RepoViewModel) -> some View {
        VStack(spacing: 0) {
            // Section header
            HStack(spacing: 6) {
                Button(action: { collapsed.wrappedValue.toggle() }) {
                    HStack(spacing: 6) {
                        Image(systemName: collapsed.wrappedValue ? "chevron.right" : "chevron.down")
                            .font(.system(size: 9, weight: .semibold))
                            .foregroundStyle(Theme.textMuted)
                        Text(title)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(Theme.textPrimary)
                        Text("(\(files.count))")
                            .font(.system(size: 13))
                            .foregroundStyle(Theme.textSecondary)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)

                Spacer()

                if !files.isEmpty {
                    Button(isStaged ? "Unstage All Changes" : "Stage All Changes") {
                        Task { isStaged ? await repo.unstageAll() : await repo.stageAll() }
                    }
                    .buttonStyle(OutlineButtonStyle())
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)

            // Rows
            if !collapsed.wrappedValue {
                if repo.fileViewMode == .path {
                    ForEach(files) { file in
                        FileRowView(file: file, isStaged: isStaged, indent: 0, repo: repo)
                    }
                } else {
                    let tree = FileTree.build(from: files)
                    ForEach(tree) { node in
                        FileTreeRows(node: node, isStaged: isStaged, indent: 0, repo: repo)
                    }
                }
            }
        }
    }
}

// MARK: - File row (Path mode / leaf)

struct FileRowView: View {
    let file: GitFileStatus
    let isStaged: Bool
    let indent: CGFloat
    let repo: RepoViewModel
    @State private var isHovered = false

    private var isSelected: Bool { repo.selectedFilePath == file.path && repo.showDiff }

    var body: some View {
        HStack(spacing: 8) {
            Text(file.status.symbol)
                .font(.system(size: 13, weight: .bold))
                .foregroundStyle(file.status.color)
                .frame(width: 14)

            HStack(spacing: 0) {
                if !file.directory.isEmpty {
                    Text(file.directory)
                        .foregroundStyle(Theme.textMuted)
                }
                Text(file.fileName)
                    .foregroundStyle(Theme.textPrimary)
            }
            .font(Theme.listFont)
            .lineLimit(1)
            .truncationMode(.middle)

            Spacer(minLength: 8)

            if isHovered || isSelected {
                Button(isStaged ? "Unstage File" : "Stage File") {
                    Task { isStaged ? await repo.unstageFile(file.path) : await repo.stageFile(file.path) }
                }
                .buttonStyle(OutlineButtonStyle())
            }
        }
        .padding(.leading, 14 + indent)
        .padding(.trailing, 12)
        .frame(height: 30)
        .background(isSelected ? Theme.selectedRow : (isHovered ? Theme.bgHover : Color.clear))
        .contentShape(Rectangle())
        .onHover { isHovered = $0 }
        .onTapGesture { Task { await repo.showFileDiff(file.path, staged: isStaged) } }
        .contextMenu {
            Button(isStaged ? "Unstage File" : "Stage File") {
                Task { isStaged ? await repo.unstageFile(file.path) : await repo.stageFile(file.path) }
            }
            Divider()
            Button("Ignore this file") {
                Task { await repo.addToGitignore("/" + file.path) }
            }
            if !ignoreDir.isEmpty {
                Button("Ignore folder “\(ignoreDir)/”") {
                    Task { await repo.addToGitignore("/" + ignoreDir + "/") }
                }
            }
            if !ignoreExt.isEmpty {
                Button("Ignore all “*.\(ignoreExt)”") {
                    Task { await repo.addToGitignore("*." + ignoreExt) }
                }
            }
            Divider()
            Button("Discard Changes", role: .destructive) {
                Task { await repo.discardFile(file.path) }
            }
        }
    }

    private var ignoreDir: String { (file.path as NSString).deletingLastPathComponent }
    private var ignoreExt: String { (file.path as NSString).pathExtension }
}

// MARK: - Tree rendering

struct FileTreeRows: View {
    let node: FileTreeNode
    let isStaged: Bool
    let indent: CGFloat
    let repo: RepoViewModel

    var body: some View {
        if let file = node.file {
            FileRowView(file: file, isStaged: isStaged, indent: indent, repo: repo)
        } else {
            VStack(spacing: 0) {
                HStack(spacing: 6) {
                    Image(systemName: "folder")
                        .font(.system(size: 10))
                        .foregroundStyle(Theme.accentTeal)
                    Text(node.name)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textSecondary)
                    Spacer()
                }
                .padding(.leading, 14 + indent)
                .padding(.trailing, 12)
                .frame(height: 26)

                ForEach(node.children) { child in
                    FileTreeRows(node: child, isStaged: isStaged, indent: indent + 16, repo: repo)
                }
            }
        }
    }
}

