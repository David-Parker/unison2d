// swift-tools-version: 5.9
//
// Root-level Swift package so the engine can be consumed via
// `XCRemoteSwiftPackageReference` pointing at this repository. The actual
// sources live under `crates/unison-ios/UnisoniOS/` — this manifest delegates
// via `path:`.

import PackageDescription

let package = Package(
    name: "Unison2D",
    platforms: [.iOS(.v15)],
    products: [
        .library(name: "UnisoniOS", targets: ["UnisoniOS"]),
    ],
    targets: [
        .target(
            name: "UnisonGameFFI",
            path: "crates/unison-ios/UnisoniOS/Sources/UnisonGameFFI",
            publicHeadersPath: "include"
        ),
        .target(
            name: "UnisoniOS",
            dependencies: ["UnisonGameFFI"],
            path: "crates/unison-ios/UnisoniOS/Sources/UnisoniOS"
        ),
    ]
)
