import XCTest

@testable import UniMusicSync

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
    // We store provider here to prevent deinitialization
    var provider: UniMusicSync?

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

        func testProvider() async throws -> (UAuthorId, UNamespaceId, [UHash], UDocTicket) {
            let provider = try await mockClient(dir: "provider")
            self.provider = provider
            let author = try await provider.getAuthor()

            // MARK: Make sure namespace gets reused

            let namespace = try await provider.getOrCreateNamespace()
            if true {
                let namespace2 = try await provider.getOrCreateNamespace()
                XCTAssertEqual(namespace, namespace2)
            }

            Task {
                try await provider.listen(namespace)
            }

            // MARK: Make sure all files get written correctly

            var fileHashes: [UHash] = []
            for (path, contents) in TEST_FILES {
                let fileHash = try await provider.writeFile(namespace, path, contents)
                try await compareFileContents(provider, fileHash: fileHash, contents: contents)
                try await compareFileContents(provider, namespace: namespace, path: path, contents: contents)
                fileHashes.append(fileHash)
            }

            let docTicket = try await provider.share(namespace)

            return (author, namespace, fileHashes, docTicket)
        }

        func testReceiver(dir: String, author _: UAuthorId, namespace: UNamespaceId, fileHashes: [UHash], docTicket: UDocTicket) async throws {
            let receiver = try await mockClient(dir: dir)

            // MARK: Make sure files are empty before syncing

            for fileHash in fileHashes {
                try await compareFileContents(receiver, fileHash: fileHash, contents: .none)
            }
            for (path, _) in TEST_FILES {
                try await compareFileContents(receiver, namespace: namespace, path: path, contents: .none)
            }

            // MARK: Make sure imported namespace is equal to the provider one

            let importedNamespace = try await receiver.import(docTicket)
            XCTAssertEqual(importedNamespace, namespace)

            // MARK: Make sure all files match

            for (i, (path, contents)) in TEST_FILES.enumerated() {
                print("File hashes: \(fileHashes)")
                print("Comparing \(path), contents: \(contents)")
                try await compareFileContents(receiver, fileHash: fileHashes[i], contents: contents)
                try await compareFileContents(receiver, namespace: namespace, path: path, contents: contents)
            }
        }

        let (author, namespace, fileHashes, docTicket) = try await testProvider()

        // MARK: Test 5 concurrent connections

        if true {
            try await withThrowingTaskGroup { group in
                for try i in 0 ... 5 {
                    group.addTask {
                        try await testReceiver(
                            dir: "receiver_\(i)",
                            author: author,
                            namespace: namespace,
                            fileHashes: fileHashes,
                            docTicket: docTicket
                        )
                    }
                }
            }

            // Wait for databases to close
            try await Task.sleep(nanoseconds: 5 * NSEC_PER_SEC)
        }

        // MARK: Make sure nodes are getting saved

        if true {
            let client = try await mockClient(dir: "receiver_0")
            let nodes = await client.irohManager.getKnownNodes()
            XCTAssertEqual(nodes.isEmpty, false)
        }
    }

    // TODO: Tests for synchronisation after importing tickets
}
