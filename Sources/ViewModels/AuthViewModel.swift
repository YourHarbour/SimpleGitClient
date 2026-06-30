import SwiftUI
import Observation

@Observable
class AuthViewModel {
    var credentials: [GitCredential] = []
    var newHost: String = "github.com"
    var newUsername: String = ""
    var newToken: String = ""
    var errorMessage: String?
    var showError = false
    private let keychainService = KeychainService.shared
    
    init() { loadCredentials() }
    
    func loadCredentials() {
        credentials = keychainService.getAllCredentials()
    }
    
    func saveCredential() {
        guard !newHost.isEmpty, !newUsername.isEmpty, !newToken.isEmpty else {
            errorMessage = "All fields are required"
            showError = true
            return
        }
        let credential = GitCredential(host: newHost, username: newUsername, token: newToken, createdAt: Date())
        do {
            try keychainService.saveCredential(credential)
            // 同时写入 git 的系统钥匙串,使 push/pull 能直接使用
            Task { try? await GitService().approveCredentialGlobal(host: credential.host,
                                                                   username: credential.username,
                                                                   token: credential.token) }
            newHost = "github.com"
            newUsername = ""
            newToken = ""
            loadCredentials()
        } catch {
            errorMessage = error.localizedDescription
            showError = true
        }
    }
    
    func deleteCredential(_ credential: GitCredential) {
        do {
            try keychainService.deleteCredential(host: credential.host, username: credential.username)
            loadCredentials()
        } catch {
            errorMessage = error.localizedDescription
            showError = true
        }
    }
    
    func configureGitCredentialHelper() async {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        process.arguments = ["config", "--global", "credential.helper", "osxkeychain"]
        try? process.run()
        process.waitUntilExit()
    }
}
