// swift-tools-version: 5.10
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let binaryTarget: Target = .binaryTarget(
    name: "UniMusicSyncCoreRS",
    // IMPORTANT: Swift packages importing this locally will not be able to
    // import the rust core unless you use a relative path.
    // This ONLY works for local development. For a larger scale usage example, see https://github.com/stadiamaps/ferrostar.
    // When you release a public package, you will need to build a release XCFramework,
    // upload it somewhere (usually with your release), and update Package.swift.
    // This will probably be the subject of a future blog.
    // Again, see Ferrostar for a more complex example, including more advanced GitHub actions.
    path: "./rust/target/ios/libunimusic_sync-rs.xcframework"
)

let package = Package(
    name: "UniMusicSync",
    platforms: [
        .iOS(.v16),
    ],
    products: [
        // Products define the executables and libraries a package produces, making them visible to other packages.
        .library(
            name: "UniMusicSync",
            targets: ["UniMusicSync"]
        ),
    ],
    targets: [
        binaryTarget,
        .target(
            name: "UniMusicSync",
            dependencies: [.target(name: "UniFFI")],
            path: "apple/Sources/UniMusicSync"
        ),
        .target(
            name: "UniFFI",
            dependencies: [.target(name: "UniMusicSyncCoreRS")],
            path: "apple/Sources/UniFFI"
        ),
        .testTarget(
            name: "UniMusicSyncTests",
            dependencies: ["UniMusicSync"],
            path: "apple/Tests/UniMusicSyncTests"
        ),
    ]
)
