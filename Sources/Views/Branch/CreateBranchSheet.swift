import SwiftUI

/// 新建分支弹窗
struct CreateBranchSheet: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var name = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Create Branch")
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)

            if let repo = appVM.activeRepo {
                Text("From \(repo.currentBranch.isEmpty ? "HEAD" : repo.currentBranch)")
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.textSecondary)
            }

            TextField("Branch name", text: $name)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
                .foregroundStyle(Theme.textPrimary)
                .padding(8)
                .background(Theme.bgApp)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
                .focused($focused)
                .onSubmit(create)

            HStack {
                Spacer()
                Button("Cancel") { appVM.showCreateBranchSheet = false }
                    .keyboardShortcut(.cancelAction)
                Button("Create", action: create)
                    .keyboardShortcut(.defaultAction)
                    .disabled(trimmed.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 380)
        .background(Theme.bgPanel)
        .onAppear { focused = true }
    }

    private var trimmed: String {
        name.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: " ", with: "-")
    }

    private func create() {
        guard !trimmed.isEmpty, let repo = appVM.activeRepo else { return }
        Task {
            try? await repo.createBranch(trimmed)
            await MainActor.run { appVM.showCreateBranchSheet = false }
        }
    }
}
