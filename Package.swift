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
    let releaseTag = "0.1.8"
    let releaseChecksum = "c8b7eb797ec491ec2f0d0bf3eff764f10d14c5b3282df7aae26af0c96fd633e7"
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
