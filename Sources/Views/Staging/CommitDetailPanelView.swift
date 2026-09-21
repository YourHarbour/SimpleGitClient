import SwiftUI

struct CommitDetailPanelView: View {
    @Environment(AppViewModel.self) private var appVM
    
    var body: some View {
        if let repo = appVM.activeRepo, let commit = repo.selectedCommit {
            VStack(alignment: .leading, spacing: 0) {
                // Header
                HStack {
                    Button(action: {
                        repo.selectWIPRow()
                    }) {
                        Image(systemName: "xmark")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(Theme.textSecondary)
                            .frame(width: 24, height: 24)
                            .background(Theme.bgElevated)
                            .clipShape(Circle())
                    }
                    .buttonStyle(.plain)
                    
                    Spacer()
                    
                    Text(commit.shortHash)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(Theme.textMuted)
                        .textSelection(.enabled)
                        .contextMenu {
                            Button("Copy SHA") { Clipboard.copy(commit.id, label: "Copied SHA") }
                            Button("Copy Short SHA") { Clipboard.copy(commit.shortHash, label: "Copied SHA") }
                        }
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 12)
                .background(Theme.bgPanel)
                
                // Commit Info
                VStack(alignment: .leading, spacing: 12) {
                    Text(commit.message)
                        .font(.system(size: 14, weight: .bold))
                        .foregroundStyle(Theme.textPrimary)
                        .contextMenu {
                            Button("Copy Subject") { Clipboard.copy(commit.message, label: "Copied subject") }
                            Button("Copy Commit Message") {
                                Clipboard.copy(commit.fullMessage, label: "Copied commit message")
                            }
                        }
                    
                    if !commit.body.isEmpty {
                        Text(commit.body)
                            .font(.system(size: 12))
                            .foregroundStyle(Theme.textSecondary)
                            .fixedSize(horizontal: false, vertical: true)
                            .contextMenu {
                                Button("Copy Description") { Clipboard.copy(commit.body, label: "Copied description") }
                                Button("Copy Commit Message") {
                                    Clipboard.copy(commit.fullMessage, label: "Copied commit message")
                                }
                            }
                    }
                    
                    HStack(spacing: 8) {
                        Circle()
                            .fill(Theme.bgElevated)
                            .frame(width: 28, height: 28)
                            .overlay(
                                Text(String(commit.author.prefix(1)).uppercased())
                                    .font(.system(size: 11, weight: .bold))
                                    .foregroundStyle(Theme.textSecondary)
                            )
                        
                        VStack(alignment: .leading, spacing: 2) {
                            Text(commit.author)
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(Theme.textPrimary)
                                .contextMenu {
                                    Button("Copy Author") {
                                        Clipboard.copy("\(commit.author) <\(commit.authorEmail)>", label: "Copied author")
                                    }
                                    Button("Copy Email") {
                                        Clipboard.copy(commit.authorEmail, label: "Copied email")
                                    }
                                }
                            Text(commit.date.formatted(date: .abbreviated, time: .shortened))
                                .font(.system(size: 11))
                                .foregroundStyle(Theme.textMuted)
                                .copyable("Date", commit.date.formatted(date: .abbreviated, time: .standard))
                        }
                    }
                }
                .padding(.horizontal, 16)
                .padding(.bottom, 16)
                // 提交详情里的文字全部可以拖选 + ⌘C
                .textSelection(.enabled)
                
                Divider().background(Theme.border)
                
                // Changed Files List
                HStack {
                    Text("\(repo.selectedCommitFiles.count) changed files")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundStyle(Theme.textSecondary)
                    Spacer()
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
                .background(Theme.bgElevated)
                
                ScrollView {
                    VStack(spacing: 0) {
                        ForEach(repo.selectedCommitFiles) { file in
                            CommitFileRowView(file: file, repo: repo)
                        }
                    }
                    .padding(.vertical, 4)
                }
                
                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.bgPanel)
            .overlay(alignment: .leading) {
                Rectangle().fill(Theme.border).frame(width: 1)
            }
        } else {
            EmptyView()
        }
    }
}

struct CommitFileRowView: View {
    let file: GitFileStatus
    let repo: RepoViewModel
    
    @State private var isHovered = false
    
    var body: some View {
        Button(action: {
            if let hash = repo.selectedCommitHash {
                Task { await repo.showCommitFileDiff(hash: hash, path: file.path) }
            }
        }) {
            HStack(spacing: 8) {
                Text(file.status.symbol)
                    .font(.system(size: 11, weight: .bold, design: .monospaced))
                    .foregroundStyle(file.status.color)
                    .frame(width: 16)
                
                Text(file.fileName)
                    .font(Theme.listFont)
                    .foregroundStyle(Theme.textPrimary)
                    .lineLimit(1)
                
                Text(file.directory)
                    .font(Theme.listFont)
                    .foregroundStyle(Theme.textMuted)
                    .lineLimit(1)
                
                Spacer()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .background(
                isHovered ? Theme.bgHover :
                (repo.selectedFilePath == file.path ? Theme.selectedRow : Color.clear)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { isHovered = $0 }
        .contextMenu {
            Button("Copy Path") { Clipboard.copy(file.path, label: "Copied path") }
            Button("Copy File Name") { Clipboard.copy(file.fileName, label: "Copied file name") }
        }
    }
}
