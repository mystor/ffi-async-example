// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "swift",
    platforms: [
        .macOS(.v15),
    ],
    targets: [
        .target(
            name: "RustLib",
            path: "Sources/RustLib",
            publicHeadersPath: "include",
            linkerSettings: [
                .unsafeFlags(["-L", "../rust-lib/target/debug"]),
                .linkedLibrary("rust_lib"),
            ]
        ),
        .executableTarget(
            name: "swift",
            dependencies: ["RustLib"]
        ),
    ],
    swiftLanguageModes: [.v6]
)
