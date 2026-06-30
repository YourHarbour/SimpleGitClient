// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "SimpleGitClient",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "SimpleGitClient", targets: ["SimpleGitClient"])
    ],
    targets: [
        .executableTarget(
            name: "SimpleGitClient",
            path: "Sources"
        )
    ]
)
