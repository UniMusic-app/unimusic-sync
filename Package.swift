// swift-tools-version: 5.10
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let useLocalFramework = false
let binaryTarget: Target

if useLocalFramework {
    binaryTarget = .binaryTarget(
        name: "UniMusicSyncCoreRS",
        // IMPORTANT: Swift packages importing this locally will not be able to
        // import UniMusicSync core unless you specify this as a relative path!
        path: "./rust/target/ios/libunimusic_sync-rs.xcframework"
    )
} else {
    let releaseTag = "0.1.6"
    let releaseChecksum = "3b963b0584c6f9f3f00e363a4ecb75496a1cd36f6c422ec9c64c1b84d3cd32e4"
    binaryTarget = .binaryTarget(
        name: "UniMusicSyncCoreRS",
        url:
        "https://github.com/UniMusic-app/unimusic-sync/releases/download/\(releaseTag)/libunimusic_sync-rs.xcframework.zip",
        checksum: releaseChecksum
    )
}

let package = Package(
    name: "UniMusicSync",
    platforms: [
        .iOS(.v15),
    ],
    products: [
        // Products define the executables and libraries a package produces, making them visible to other packages.
        .library(
            name: "UniMusicSync",
            targets: ["UniMusicSync"],
        ),
    ],
    targets: [
        binaryTarget,
        .target(
            name: "UniMusicSync",
            dependencies: [.target(name: "UniFFI")],
            path: "apple/Sources/UniMusicSync",
        ),
        .target(
            name: "UniFFI",
            dependencies: [.target(name: "UniMusicSyncCoreRS")],
            path: "apple/Sources/UniFFI",
        ),
        .testTarget(
            name: "UniMusicSyncTests",
            dependencies: ["UniMusicSync"],
            path: "apple/Tests/UniMusicSyncTests"
        ),
    ]
)
