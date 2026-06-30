import SwiftUI

/// 设置 → Git 凭据管理(⚙️ 入口)。可添加任意 host(含 git.overleaf.com)的 token。
struct CredentialsSheet: View {
    @Environment(AppViewModel.self) private var appVM
    @State private var authVM = AuthViewModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                Text("Git Credentials")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(Theme.textPrimary)
                Spacer()
                Button(action: { appVM.showCredentialsSheet = false }) {
                    Image(systemName: "xmark").foregroundStyle(Theme.textSecondary)
                }.buttonStyle(.plain)
            }

            if authVM.showError, let error = authVM.errorMessage {
                Text(error).font(.system(size: 12)).foregroundStyle(Theme.accentRed)
            }

            // 已保存
            if !authVM.credentials.isEmpty {
                Text("Saved").font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
                VStack(spacing: 0) {
                    ForEach(authVM.credentials) { cred in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(cred.host).font(.system(size: 13, weight: .medium)).foregroundStyle(Theme.textPrimary)
                                Text(cred.username).font(.system(size: 11)).foregroundStyle(Theme.textSecondary)
                            }
                            Spacer()
                            Button("Delete") { authVM.deleteCredential(cred) }
                                .buttonStyle(.plain).foregroundStyle(Theme.accentRed).font(.system(size: 12))
                        }
                        .padding(.vertical, 8).padding(.horizontal, 10)
                        .background(Theme.bgApp)
                        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
                    }
                }
                .overlay(RoundedRectangle(cornerRadius: 6).stroke(Theme.border, lineWidth: 1))
            }

            // 新增
            Text("Add token").font(.system(size: 12)).foregroundStyle(Theme.textSecondary)
            field("Host", placeholder: "github.com / git.overleaf.com", text: $authVM.newHost)
            field("Username", placeholder: "git", text: $authVM.newUsername)
            secureField("Personal access token", text: $authVM.newToken)

            HStack {
                Spacer()
                Button("Save") { authVM.saveCredential() }
                    .buttonStyle(.borderedProminent).tint(Theme.commitGreen)
                    .disabled(authVM.newHost.isEmpty || authVM.newToken.isEmpty)
            }

            Spacer()
        }
        .padding(22)
        .frame(width: 460, height: 520)
        .background(Theme.bgPanel)
    }

    private func field(_ label: String, placeholder: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.system(size: 11)).foregroundStyle(Theme.textMuted)
            TextField(placeholder, text: text)
                .textFieldStyle(.plain).font(.system(size: 13)).foregroundStyle(Theme.textPrimary)
                .padding(8).background(Theme.bgApp)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
        }
    }

    private func secureField(_ placeholder: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Token").font(.system(size: 11)).foregroundStyle(Theme.textMuted)
            SecureField(placeholder, text: text)
                .textFieldStyle(.plain).font(.system(size: 13)).foregroundStyle(Theme.textPrimary)
                .padding(8).background(Theme.bgApp)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(Theme.borderStrong, lineWidth: 1))
        }
    }
}
