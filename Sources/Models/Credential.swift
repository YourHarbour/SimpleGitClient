import Foundation

struct GitCredential: Identifiable, Hashable {
    var id: String { host + "/" + username }
    let host: String
    let username: String
    var token: String
    let createdAt: Date
    
    static let commonHosts = [
        "github.com",
        "gitlab.com",
        "bitbucket.org",
    ]
}
