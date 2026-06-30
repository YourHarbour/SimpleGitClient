import SwiftUI

/// 提交编辑区 (Commit Composer) — spec §8.5
struct CommitComposerView: View {
    @Environment(AppViewModel.self) private var appVM
    @FocusState private var summaryFocused: Bool
    @State private var optionsExpanded = false

    private let summaryHint = 72

    var body: some View {
        if let repo = appVM.activeRepo {
            let binding = Bindable(repo)
            VStack(spacing: 0) {
                Rectangle().fill(Theme.border).frame(height: 1)

                VStack(spacing: 10) {
                    // 1. Commit 标签
                    HStack(spacing: 6) {
                        CommitNodeGlyph(color: Theme.accentTeal)
                        Text("Commit").font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(Theme.textPrimary)
                    }
                    .padding(.horizontal, 10).padding(.vertical, 5)
                    .background(RoundedRectangle(cornerRadius: 5).fill(Theme.bgElevated))
                    .frame(maxWidth: .infinity, alignment: .leading)

                    // 2. Summary + char counter
                    ZStack(alignment: .topTrailing) {
                        TextField("Commit summary", text: binding.commitSummary)
                            .textFieldStyle(.plain)
                            .font(.system(size: 13, weight: .medium))
                            .foregroundStyle(Theme.textPrimary)
                            .padding(.vertical, 8)
                            .padding(.leading, 8)
                            .padding(.trailing, 36)
                            .background(Theme.bgApp)
                            .overlay(RoundedRectangle(cornerRadius: 5)
                                .stroke(summaryFocused ? Theme.accentTeal : Theme.borderStrong, lineWidth: 1))
                            .focused($summaryFocused)

                        Text("\(summaryHint - repo.commitSummary.count)")
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(repo.commitSummary.count > summaryHint ? Theme.accentRed : Theme.textMuted)
                            .padding(.top, 9)
                            .padding(.trailing, 9)
                    }

                    // 4. Description
                    TextEditor(text: binding.commitDescription)
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.textPrimary)
                        .scrollContentBackground(.hidden)
                        .padding(4)
                        .background(Theme.bgApp)
                        .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
                        .frame(height: 84)
                        .overlay(alignment: .topLeading) {
                            if repo.commitDescription.isEmpty {
                                Text("Description")
                                    .font(.system(size: 13))
                                    .foregroundStyle(Theme.textMuted)
                                    .padding(.top, 10).padding(.leading, 9)
                                    .allowsHitTesting(false)
                            }
                        }

                    // 5. Commit options
                    VStack(spacing: 6) {
                        Button(action: { withAnimation(.easeInOut(duration: 0.15)) { optionsExpanded.toggle() } }) {
                            HStack(spacing: 6) {
                                Image(systemName: optionsExpanded ? "chevron.down" : "chevron.right")
                                    .font(.system(size: 9, weight: .semibold))
                                Text("Commit options").font(.system(size: 12))
                                Spacer()
                            }
                            .foregroundStyle(Theme.textSecondary)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)

                        if optionsExpanded {
                            VStack(alignment: .leading, spacing: 4) {
                                Toggle(isOn: binding.commitSignOff) {
                                    Text("Sign off (-s)").font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
                                }.toggleStyle(.checkbox)
                                Toggle(isOn: binding.commitAllowEmpty) {
                                    Text("Allow empty commit").font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
                                }.toggleStyle(.checkbox)
                            }
                            .padding(.leading, 16)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }

                    // 6. Main commit button
                    Button(action: { Task { await repo.performCommit() } }) {
                        HStack(spacing: 6) {
                            CommitNodeGlyph(color: repo.commitButtonState.isEnabled ? .white : Theme.textMuted)
                            Text(repo.commitButtonState.buttonText)
                                .font(.system(size: 13, weight: .semibold))
                        }
                        .foregroundStyle(repo.commitButtonState.isEnabled ? .white : Theme.textMuted)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 9)
                    }
                    .buttonStyle(PrimaryButtonStyle())
                    .disabled(!repo.commitButtonState.isEnabled)
                }
                .padding(14)
            }
            .background(Theme.bgPanel)
        }
    }

}

/// 提交节点小图标 "-○-"
struct CommitNodeGlyph: View {
    var color: Color = Theme.textPrimary
    var body: some View {
        HStack(spacing: 0) {
            Rectangle().fill(color).frame(width: 4, height: 1.5)
            Circle().stroke(color, lineWidth: 1.5).frame(width: 7, height: 7)
            Rectangle().fill(color).frame(width: 4, height: 1.5)
        }
    }
}
