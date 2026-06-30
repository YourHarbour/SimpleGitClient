import SwiftUI
import AppKit

/// 克隆仓库弹窗 — spec/截图 §图2(精简为 URL 与 GitHub 两种来源)
struct CloneSheet: View {
    @Environment(AppViewModel.self) private var appVM

    enum Source: String, CaseIterable { case url = "Clone with URL", github = "GitHub.com" }
    @State private var source: Source = .url
    @State private var destination: String = ""
    @State private var url: String = ""
    @State private var ownerRepo: String = ""   // GitHub: owner/repo
    @State private var name: String = ""        // 目标文件夹名(留空则用默认)
    @State private var shallow = false

    private var resolvedURL: String {
        switch source {
        case .url: return url.trimmingCharacters(in: .whitespaces)
        case .github:
            let s = ownerRepo.trimmingCharacters(in: .whitespaces)
            guard !s.isEmpty else { return "" }
            if s.hasPrefix("http") || s.contains("@") { return s }
            return "https://github.com/\(s).git"
        }
    }

    var body: some View {
        HStack(spacing: 0) {
            // 左侧来源列表
            VStack(alignment: .leading, spacing: 2) {
                ForEach(Source.allCases, id: \.self) { s in
                    Button(action: { source = s }) {
                        HStack(spacing: 10) {
                            Image(systemName: s == .url ? "globe" : "chevron.left.forwardslash.chevron.right")
                                .font(.system(size: 13)).frame(width: 18)
                            Text(s.rawValue).font(.system(size: 13))
                            Spacer()
                        }
                        .foregroundStyle(source == s ? Theme.textPrimary : Theme.textSecondary)
                        .padding(.horizontal, 14).padding(.vertical, 10)
                        .background(source == s ? Theme.accentBlue.opacity(0.25) : Color.clear)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .frame(width: 220)
            .frame(maxHeight: .infinity)
            .background(Theme.bgApp)

            // 右侧表单
            VStack(alignment: .leading, spacing: 18) {
                HStack {
                    Text(source == .url ? "Clone a Repo" : "Clone from GitHub")
                        .font(.system(size: 17, weight: .semibold))
                        .foregroundStyle(Theme.textPrimary)
                    Spacer()
                    Button(action: { appVM.showCloneSheet = false }) {
                        Image(systemName: "xmark").foregroundStyle(Theme.textSecondary)
                    }.buttonStyle(.plain)
                }

                formRow("Where to clone to") {
                    HStack(spacing: 8) {
                        textField("", text: $destination)
                        Button("Browse") { browse() }
                            .buttonStyle(.bordered)
                    }
                }

                if source == .url {
                    formRow("URL") { textField("https://… or git@…", text: $url) }
                } else {
                    formRow("Repository") { textField("owner/repo", text: $ownerRepo) }
                }

                formRow("Name") {
                    textField(AppViewModel.cloneFolderName(url: resolvedURL, name: nil), text: $name)
                }

                formRow("") {
                    Toggle(isOn: $shallow) {
                        Text("Shallow Clone").font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
                    }.toggleStyle(.checkbox)
                }

                HStack {
                    Spacer()
                    Button("Clone the repo!") { clone() }
                        .buttonStyle(.borderedProminent)
                        .tint(Theme.commitGreen)
                        .disabled(resolvedURL.isEmpty || destination.isEmpty)
                }

                Spacer()
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(Theme.bgPanel)
        }
        .frame(width: 720, height: 420)
        .onAppear { if destination.isEmpty { destination = defaultDestination() } }
    }

    // MARK: - Components

    private func formRow<Content: View>(_ label: String, @ViewBuilder content: () -> Content) -> some View {
        HStack(alignment: .center, spacing: 12) {
            Text(label)
                .font(.system(size: 13))
                .foregroundStyle(Theme.textSecondary)
                .frame(width: 120, alignment: .trailing)
            content()
        }
    }

    private func textField(_ placeholder: String, text: Binding<String>) -> some View {
        TextField(placeholder, text: text)
            .textFieldStyle(.plain)
            .font(.system(size: 13))
            .foregroundStyle(Theme.textPrimary)
            .padding(8)
            .background(Theme.bgApp)
            .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
    }

    // MARK: - Actions

    private func defaultDestination() -> String {
        if let recent = appVM.recentRepos.first {
            return (recent as NSString).deletingLastPathComponent
        }
        return NSHomeDirectory()
    }

    private func browse() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        if panel.runModal() == .OK, let dir = panel.url { destination = dir.path }
    }

    private func clone() {
        let target = destination, link = resolvedURL
        let folder = name.trimmingCharacters(in: .whitespaces)
        appVM.showCloneSheet = false
        Task { await appVM.cloneRepository(url: link, to: target, name: folder.isEmpty ? nil : folder) }
    }
}
