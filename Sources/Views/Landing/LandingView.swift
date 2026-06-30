import SwiftUI

/// New Tab 落地页 — 打开 / 克隆仓库 + 最近列表(参考 GitKraken 的 New Tab 页)
struct LandingView: View {
    @Environment(AppViewModel.self) private var appVM

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Text("Repositories")
                    .font(.system(size: 24, weight: .semibold))
                    .foregroundStyle(Theme.textPrimary)

                HStack(spacing: 14) {
                    actionButton(icon: "folder", title: "Open") { appVM.showOpenDialog() }
                    actionButton(icon: "cloud", title: "Clone") { appVM.showCloneSheet = true }
                }

                if !appVM.recentRepos.isEmpty {
                    VStack(alignment: .leading, spacing: 10) {
                        Text("Recent")
                            .font(.system(size: 14))
                            .foregroundStyle(Theme.textSecondary)

                        VStack(alignment: .leading, spacing: 2) {
                            ForEach(appVM.recentRepos, id: \.self) { path in
                                recentRow(path)
                            }
                        }
                    }
                }
            }
            .padding(40)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.bgApp)
    }

    private func actionButton(icon: String, title: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: icon).font(.system(size: 14))
                Text(title).font(.system(size: 14, weight: .medium))
            }
            .foregroundStyle(Theme.textPrimary)
            .padding(.horizontal, 18).padding(.vertical, 12)
            .background(RoundedRectangle(cornerRadius: 8).fill(Theme.bgElevated))
            .overlay(RoundedRectangle(cornerRadius: 8).stroke(Theme.border, lineWidth: 1))
        }
        .buttonStyle(.plain)
    }

    private func recentRow(_ path: String) -> some View {
        let name = (path as NSString).lastPathComponent
        return Button(action: { Task { await appVM.openRepository(at: path) } }) {
            HStack(spacing: 12) {
                Text(name)
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.accentBlue)
                Text(path)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.textMuted)
                    .lineLimit(1).truncationMode(.middle)
                Spacer()
            }
            .padding(.vertical, 5).padding(.horizontal, 8)
            .contentShape(Rectangle())
        }
        .buttonStyle(HoverRowStyle())
        .contextMenu {
            Button("Remove from Recent") { appVM.removeRecent(path) }
        }
    }
}
