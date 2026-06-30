import Foundation
import Security

class KeychainService {
    static let shared = KeychainService()
    private let servicePrefix = "com.gitpilot.credential"
    private init() {}
    
    func saveCredential(_ credential: GitCredential) throws {
        let service = "\(servicePrefix).\(credential.host)"
        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: credential.username
        ]
        SecItemDelete(deleteQuery as CFDictionary)
        
        guard let tokenData = credential.token.data(using: .utf8) else {
            throw KeychainError.encodingError
        }
        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: credential.username,
            kSecValueData as String: tokenData,
            kSecAttrLabel as String: "GitPilot - \(credential.host)",
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlocked
        ]
        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }
    
    func getCredential(host: String, username: String? = nil) throws -> GitCredential? {
        let service = "\(servicePrefix).\(host)"
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecReturnData as String: true,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        if let username = username {
            query[kSecAttrAccount as String] = username
        }
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess,
              let dict = item as? [String: Any],
              let tokenData = dict[kSecValueData as String] as? Data,
              let token = String(data: tokenData, encoding: .utf8),
              let account = dict[kSecAttrAccount as String] as? String
        else {
            if status == errSecItemNotFound { return nil }
            throw KeychainError.retrieveFailed(status)
        }
        return GitCredential(host: host, username: account, token: token, createdAt: Date())
    }
    
    func getAllCredentials() -> [GitCredential] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecMatchLimit as String: kSecMatchLimitAll,
            kSecReturnAttributes as String: true,
            kSecReturnData as String: true
        ]
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let items = result as? [[String: Any]] else { return [] }

        var credentials: [GitCredential] = []
        for item in items {
            guard let service = item[kSecAttrService as String] as? String,
                  service.hasPrefix(servicePrefix + ".") else { continue }
            let host = String(service.dropFirst(servicePrefix.count + 1))
            let account = item[kSecAttrAccount as String] as? String ?? ""
            let token = (item[kSecValueData as String] as? Data)
                .flatMap { String(data: $0, encoding: .utf8) } ?? ""
            credentials.append(GitCredential(host: host, username: account, token: token, createdAt: Date()))
        }
        return credentials.sorted { $0.host < $1.host }
    }
    
    func deleteCredential(host: String, username: String) throws {
        let service = "\(servicePrefix).\(host)"
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: username
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status)
        }
    }
}

enum KeychainError: LocalizedError {
    case encodingError
    case saveFailed(OSStatus)
    case retrieveFailed(OSStatus)
    case deleteFailed(OSStatus)
    
    var errorDescription: String? {
        switch self {
        case .encodingError: return "Failed to encode token data"
        case .saveFailed(let s): return "Keychain save failed: \(s)"
        case .retrieveFailed(let s): return "Keychain retrieve failed: \(s)"
        case .deleteFailed(let s): return "Keychain delete failed: \(s)"
        }
    }
}
