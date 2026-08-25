// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "ambient-context-ax",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "ambient-context-ax", targets: ["ambient-context-ax"])
    ],
    targets: [
        .executableTarget(
            name: "ambient-context-ax",
            path: "Sources"
        )
    ]
)
