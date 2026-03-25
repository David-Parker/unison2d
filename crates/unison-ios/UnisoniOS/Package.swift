// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "UnisoniOS",
    platforms: [.iOS(.v15)],
    products: [
        .library(name: "UnisoniOS", targets: ["UnisoniOS"]),
    ],
    targets: [
        // C target: exposes the FFI header so Swift can call Rust functions.
        // The actual function bodies are in the game's static library (linked by Xcode).
        .target(
            name: "UnisonGameFFI",
            path: "Sources/UnisonGameFFI",
            publicHeadersPath: "include"
        ),
        // Swift target: GameViewController + Renderer that call through the C header.
        .target(
            name: "UnisoniOS",
            dependencies: ["UnisonGameFFI"],
            path: "Sources/UnisoniOS"
        ),
    ]
)
