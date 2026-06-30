import SwiftUI

/// 鉴权弹窗 — push 或 clone 需要 token 时出现。
/// Overleaf 等只需 token,用户名默认 `git` 折叠在「Advanced」里。勾选记住后写入钥匙串,下次自动完成。
struct AuthSheet: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var token = ""
    @State private var username = "git"
    @State private var remember = true
    @State private var showAdvanced = false
    @State private var working = false
    @FocusState private var tokenFocused: Bool

    private var isClone: Bool {
        if case .clone = appVM.authContext { return true }
        return false
    }
    private var actionWord: String { isClone ? "Clone" : "Push" }
    private var verb: String { isClone ? "Cloning from" : "Pushing to" }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Authentication required")
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(Theme.textPrimary)

            Text("\(verb) \(appVM.authHost) needs a token. For Overleaf, paste your Git token below — no username needed.")
                .font(.system(size: 12))
                .foregroundStyle(Theme.textSecondary)
                .fixedSize(horizontal: false, vertical: true)

            field("Token") {
                SecureField("Personal access / Git token", text: $token)
                    .textFieldStyle(.plain)
                    .focused($tokenFocused)
                    .onSubmit(submit)
            }

            Toggle(isOn: $remember) {
                Text("Remember this token (store in macOS Keychain)")
                    .font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
            }
            .toggleStyle(.checkbox)

            DisclosureGroup(isExpanded: $showAdvanced) {
                field("Username") {
                    TextField("git", text: $username).textFieldStyle(.plain)
                }
                .padding(.top, 6)
            } label: {
                Text("Advanced")
                    .font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
            }
            .tint(Theme.textSecondary)

            HStack {
                if working { ProgressView().controlSize(.small) }
                Spacer()
                Button("Cancel") { appVM.showAuthSheet = false }
                    .keyboardShortcut(.cancelAction)
                Button("Authenticate & \(actionWord)", action: submit)
                    .keyboardShortcut(.defaultAction)
                    .disabled(token.isEmpty || working)
            }
        }
        .padding(22)
        .frame(width: 440)
        .background(Theme.bgPanel)
        .onAppear {
            if !appVM.authUsername.isEmpty { username = appVM.authUsername }
            tokenFocused = true
        }
    }

    private func field<Content: View>(_ label: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.system(size: 11)).foregroundStyle(Theme.textMuted)
            content()
                .font(.system(size: 13))
                .foregroundStyle(Theme.textPrimary)
                .padding(8)
                .background(Theme.bgApp)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
        }
    }

    private func submit() {
        guard !token.isEmpty else { return }
        working = true
        let u = username.trimmingCharacters(in: .whitespaces).isEmpty ? "git" : username
        Task {
            await appVM.submitAuth(username: u, token: token, remember: remember)
            await MainActor.run { working = false }
        }
    }
}
