import SwiftUI
import AppKit

/// 差异视图 — spec §7.B
struct DiffViewerView: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var currentHunk = 0

    var body: some View {
        if let repo = appVM.activeRepo {
            let dvm = repo.diffViewModel
            VStack(spacing: 0) {
                fileHeader(repo: repo, dvm: dvm)      // 文件头条
                diffToolbar(repo: repo, dvm: dvm)     // Diff 工具栏
                contentArea(repo: repo, dvm: dvm)     // 内容区
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.bgApp)
        }
    }

    // MARK: - File header (44px)

    private func fileHeader(repo: RepoViewModel, dvm: DiffViewModel) -> some View {
        let status = dvm.diffFile?.fileStatus ?? .modified
        return HStack(spacing: 8) {
            Text(status.symbol)
                .font(.system(size: 14, weight: .bold))
                .foregroundStyle(status.color)
            Text(dvm.filePath)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.textPrimary)
                .lineLimit(1)
                .truncationMode(.middle)

            Spacer()

            Text("UTF-8")
                .font(.system(size: 10))
                .foregroundStyle(Theme.textSecondary)
                .padding(.horizontal, 6).padding(.vertical, 2)
                .background(Capsule().fill(Theme.bgElevated))

            // 提交详情中的文件 diff 没有 stage 概念
            if repo.selectedCommitHash == nil {
                Button(dvm.isStaged ? "Unstage File" : "Stage File") {
                    Task {
                        dvm.isStaged ? await repo.unstageFile(dvm.filePath)
                                     : await repo.stageFile(dvm.filePath)
                    }
                }
                .buttonStyle(OutlineButtonStyle())
            }

            Button(action: { repo.closeDiff() }) {
                Image(systemName: "xmark")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(Theme.textSecondary)
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(ToolbarButtonStyle(cornerRadius: 4))
        }
        .padding(.horizontal, 14)
        .frame(height: 44)
        .background(Theme.bgPanel)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
    }

    // MARK: - Diff toolbar (40px)

    private func diffToolbar(repo: RepoViewModel, dvm: DiffViewModel) -> some View {
        let binding = Bindable(dvm)
        return HStack(spacing: 10) {
            Button(action: { openInEditor(repo: repo, path: dvm.filePath) }) {
                HStack(spacing: 4) {
                    Image(systemName: "pencil").font(.system(size: 11))
                    Text("Edit This File").font(.system(size: 12))
                }
                .foregroundStyle(Theme.textSecondary)
                .padding(.horizontal, 6).padding(.vertical, 4)
            }
            .buttonStyle(ToolbarButtonStyle(cornerRadius: 5))

            Text(dvm.isStaged ? "Staged" : "Unstaged")
                .font(.system(size: 12))
                .foregroundStyle(Theme.textMuted)

            Spacer()

            // File View / Diff View 段控
            HStack(spacing: 0) {
                SegmentButton(title: "File View", active: dvm.viewMode == .file) { dvm.viewMode = .file }
                SegmentButton(title: "Diff View", active: dvm.viewMode == .diff) { dvm.viewMode = .diff }
            }
            .padding(2)
            .background(RoundedRectangle(cornerRadius: 6).fill(Theme.bgApp))

            Spacer()

            // 改动块跳转
            iconToggle("chevron.up", help: "Previous change") { navigateHunk(-1, dvm: dvm) }
            iconToggle("chevron.down", help: "Next change") { navigateHunk(1, dvm: dvm) }

            Divider().frame(height: 18)

            // 空白字符 / 自动换行
            iconToggle("paragraphsign", help: "Show whitespace", active: dvm.showWhitespace) {
                binding.showWhitespace.wrappedValue.toggle()
            }
            iconToggle("arrow.turn.down.left", help: "Wrap lines", active: dvm.wrapLines) {
                binding.wrapLines.wrappedValue.toggle()
            }
        }
        .padding(.horizontal, 14)
        .frame(height: 40)
        .background(Theme.bgPanel)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
    }

    private func iconToggle(_ icon: String, help: String, active: Bool = false, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 12))
                .foregroundStyle(active ? Theme.accentTeal : Theme.textSecondary)
                .frame(width: 24, height: 24)
                .background(RoundedRectangle(cornerRadius: 4).fill(active ? Theme.bgElevated : Color.clear))
        }
        .buttonStyle(ToolbarButtonStyle(cornerRadius: 4))
        .help(help)
    }

    // MARK: - Content

    @ViewBuilder
    private func contentArea(repo: RepoViewModel, dvm: DiffViewModel) -> some View {
        if dvm.isLoading {
            ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let error = dvm.error {
            VStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle").font(.system(size: 28)).foregroundStyle(Theme.accentRed)
                Text(error).font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if dvm.viewMode == .file {
            fileView(dvm: dvm)
        } else if let diff = dvm.diffFile {
            if diff.isBinary {
                centeredMessage("Binary file not shown.")
            } else if diff.hunks.isEmpty {
                centeredMessage("No changes to display.")
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            ForEach(Array(diff.hunks.enumerated()), id: \.element.id) { idx, hunk in
                                DiffHunkBlock(hunk: hunk, wrap: dvm.wrapLines, showWhitespace: dvm.showWhitespace)
                                    .id("hunk-\(idx)")
                            }
                        }
                    }
                    .background(Theme.bgApp)
                    .onChange(of: currentHunk) { _, new in
                        withAnimation { proxy.scrollTo("hunk-\(new)", anchor: .top) }
                    }
                }
            }
        } else {
            centeredMessage("Select a file to view changes")
        }
    }

    private func fileView(dvm: DiffViewModel) -> some View {
        let lines = (dvm.fileContent ?? "").components(separatedBy: "\n")
        return ScrollView([.vertical, .horizontal]) {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(Array(lines.enumerated()), id: \.offset) { idx, text in
                    HStack(spacing: 0) {
                        Text("\(idx + 1)")
                            .font(Theme.codeFontFallback)
                            .foregroundStyle(Theme.diffLineNumber)
                            .frame(width: 48, alignment: .trailing)
                            .padding(.trailing, 10)
                        Text(text.isEmpty ? " " : text)
                            .font(Theme.codeFontFallback)
                            .foregroundStyle(Theme.textPrimary)
                    }
                    .frame(height: 17)
                }
            }
            .padding(.vertical, 6)
        }
        .background(Theme.bgApp)
    }

    private func centeredMessage(_ text: String) -> some View {
        Text(text).foregroundStyle(Theme.textMuted).frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Actions

    private func navigateHunk(_ delta: Int, dvm: DiffViewModel) {
        let count = dvm.diffFile?.hunks.count ?? 0
        guard count > 0 else { return }
        currentHunk = min(max(0, currentHunk + delta), count - 1)
    }

    private func openInEditor(repo: RepoViewModel, path: String) {
        guard let repoPath = repo.gitService.repoPath else { return }
        let full = (repoPath as NSString).appendingPathComponent(path)
        NSWorkspace.shared.open(URL(fileURLWithPath: full))
    }
}

// MARK: - Hunk block

struct DiffHunkBlock: View {
    let hunk: DiffHunk
    let wrap: Bool
    let showWhitespace: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 区块头 @@ ... @@
            Text(hunk.header)
                .font(Theme.codeFontFallback)
                .foregroundStyle(Theme.textMuted)
                .padding(.horizontal, 12).padding(.vertical, 3)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Theme.bgElevated)

            ForEach(hunk.lines) { line in
                DiffLineRow(line: line, wrap: wrap, showWhitespace: showWhitespace)
            }
        }
    }
}

struct DiffLineRow: View {
    let line: DiffLine
    let wrap: Bool
    let showWhitespace: Bool

    var body: some View {
        HStack(spacing: 0) {
            Text(gutterLabel)
                .font(Theme.codeFontFallback)
                .foregroundStyle(Theme.diffLineNumber)
                .frame(width: 48, alignment: .trailing)
                .padding(.trailing, 8)
                .background(gutterColor)

            HStack(spacing: 0) {
                Text(marker)
                    .font(Theme.codeFontFallback)
                    .foregroundStyle(markerColor)
                    .frame(width: 14, alignment: .center)
                Text(displayContent)
                    .font(Theme.codeFontFallback)
                    .foregroundStyle(Theme.textPrimary)
                    .lineLimit(wrap ? nil : 1)
                    .fixedSize(horizontal: false, vertical: wrap)
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(rowColor)
        }
        .frame(minHeight: 17)
    }

    private var gutterLabel: String {
        switch line.type {
        case .added: return line.newLineNumber.map(String.init) ?? ""
        case .removed: return line.oldLineNumber.map(String.init) ?? ""
        case .context: return line.newLineNumber.map(String.init) ?? ""
        case .hunkHeader: return ""
        }
    }

    private var displayContent: String {
        guard showWhitespace else { return line.content.isEmpty ? " " : line.content }
        let visible = line.content.replacingOccurrences(of: " ", with: "·")
            .replacingOccurrences(of: "\t", with: "→   ")
        return visible.isEmpty ? " " : visible
    }

    private var marker: String {
        switch line.type {
        case .added: return "+"
        case .removed: return "−"
        case .context, .hunkHeader: return ""
        }
    }

    private var markerColor: Color {
        switch line.type {
        case .added: return Theme.accentGreen
        case .removed: return Theme.accentRed
        default: return .clear
        }
    }

    private var rowColor: Color {
        switch line.type {
        case .added: return Theme.diffAddedBg
        case .removed: return Theme.diffRemovedBg
        default: return .clear
        }
    }

    private var gutterColor: Color {
        switch line.type {
        case .added: return Theme.diffAddedGutter
        case .removed: return Theme.diffRemovedGutter
        default: return Theme.bgPanel
        }
    }
}
