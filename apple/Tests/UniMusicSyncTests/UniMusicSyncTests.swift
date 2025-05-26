import XCTest

@testable import UniMusicSync

let MODIFIED_FILE = (path: "dog_breeds.txt", contents: Data("German Shephard, Husky, Pomeranian".utf8))
let TEST_FILES = [
    "dog_breeds.txt": Data("American Eskimo Dog, Husky, Cocker Spaniel, Pomeranian".utf8),
    "bing chilling.txt": Data("""
        現在我有冰淇淋
        我很喜歡冰淇淋
        但是
        《速度與激情9》
        比冰淇淋
        《速度與激-》
        《速度與激情9》
        我最喜歡
        所以現在是
        音樂時間
        準備

        一
        二
        三

        兩個禮拜以後
        《速度與激情9》
        兩個禮拜以後
        《速度與激情9》
        兩個禮拜以後
        《速度與激情9》

        不要忘記
        不要錯過
        去電影院
        看《速度與激情9》
        因為非常好電影
        動作非常好
        差不多一樣「冰激淋」
        再見
    """.utf8),
]

final class UniMusicSyncTests: XCTestCase {
    var tempDirectoryURL: URL!
    override func setUp() {
        super.setUp()

        let uuid = UUID().uuidString
        tempDirectoryURL = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
            .appendingPathComponent("UniMusicSyncTests-\(uuid)")
        try! FileManager().createDirectory(at: tempDirectoryURL, withIntermediateDirectories: true)
    }

    override func tearDown() {
        try! FileManager().removeItem(at: tempDirectoryURL)
        super.tearDown()
    }

    func testConnection() async throws {
        func mockClient(dir: String) async throws -> UniMusicSync {
            let uniMusicSync = try await UniMusicSync(tempDirectoryURL.appendingPathComponent(dir).path)
            return uniMusicSync
        }

        func compareFileContents(_ client: UniMusicSync, fileHash: UHash, contents: Data?) async throws {
            let file = try? await client.readFileHash(fileHash)
            XCTAssertEqual(file, contents)
        }
        func compareFileContents(_ client: UniMusicSync, namespace: UNamespaceId, path: String, contents: Data?) async throws {
            let file = try? await client.readFile(namespace, path)
            XCTAssertEqual(file, contents)
        }

        let provider = try await mockClient(dir: "provider")
        print("[provider]: create namespace")
        let namespace = try await provider.createNamespace()

        // MARK: Make sure all files get written correctly

        var fileHashes: [UHash] = []
        print("[provider]: write files")
        for (path, contents) in TEST_FILES {
            let fileHash = try await provider.writeFile(namespace, path, contents)
            try await compareFileContents(provider, fileHash: fileHash, contents: contents)
            try await compareFileContents(provider, namespace: namespace, path: path, contents: contents)
            fileHashes.append(fileHash)
        }

        print("[provider]: share ticket")
        let docTicket = try await provider.share(namespace)

        // MARK: Test 5 concurrent connections

        try await withThrowingTaskGroup { group in
            for try i in 0 ..< 5 {
                group.addTask {
                    let receiver = try await mockClient(dir: "receiver_\(i)")

                    print("[receiver \(i)]: make sure files are empty before import")

                    for fileHash in fileHashes {
                        try await compareFileContents(receiver, fileHash: fileHash, contents: .none)
                    }
                    for (path, _) in TEST_FILES {
                        try await compareFileContents(receiver, namespace: namespace, path: path, contents: .none)
                    }

                    print("[receiver \(i)]: make sure imported namespace is equal to the provider one")
                    let importedNamespace = try await receiver.import(docTicket)
                    XCTAssertEqual(importedNamespace, namespace)

                    print("[receiver \(i)]: make sure files get properly imported")
                    for (j, (path, contents)) in TEST_FILES.enumerated() {
                        print("[receiver \(i)]: make sure \(path) gets properly imported")
                        try await compareFileContents(receiver, fileHash: fileHashes[j], contents: contents)
                        try await compareFileContents(receiver, namespace: namespace, path: path, contents: contents)
                    }

                    print("[receiver \(i)]: shutdown")
                    try await receiver.shutdown()
                }
            }
        }

        print("[provider]: modify \(MODIFIED_FILE.path)")
        let fileHash = try await provider.writeFile(namespace, MODIFIED_FILE.path, MODIFIED_FILE.contents)
        try await compareFileContents(provider, fileHash: fileHash, contents: MODIFIED_FILE.contents)

        // MARK: Make sure nodes properly reconnect and sync

        try await withThrowingTaskGroup { group in
            for try i in 0 ..< 5 {
                group.addTask {
                    print("[receiver \(i)]: recreate")
                    let receiver = try await mockClient(dir: "receiver_\(i)")

                    print("[receiver \(i)]: make sure all files are still there")
                    for (j, (path, contents)) in TEST_FILES.enumerated() {
                        try await compareFileContents(receiver, fileHash: fileHashes[j], contents: contents)
                        try await compareFileContents(receiver, namespace: namespace, path: path, contents: contents)
                    }

                    print("[receiver \(i)]: reconnect")
                    await receiver.reconnect()

                    print("[receiver \(i)]: sync")
                    try await receiver.sync(namespace)

                    print("[receiver \(i)]: make sure file got properly synced")
                    try await compareFileContents(receiver, fileHash: fileHash, contents: MODIFIED_FILE.contents)

                    print("[receiver \(i)]: shutdown")
                    try await receiver.shutdown()
                }
            }
        }

        print("[provider]: shutdown")
        try await provider.shutdown()
    }
}
