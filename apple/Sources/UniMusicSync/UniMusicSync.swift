#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

import Foundation
@_exported import UniFFI

public class UniMusicSync {
    public let irohManager: IrohManager

    deinit {
        let irohManager = self.irohManager
        Task { try await irohManager.shutdown() }
    }

    public init(_ path: String) async throws {
        let irohFactory = IrohFactory()
        irohManager = try await irohFactory.irohManager(path: path)
    }

    public func getOrCreateNamespace() async throws -> UNamespaceId {
        let namespaceId = try! await irohManager.getOrCreateNamespace()
        return namespaceId
    }

    public func getAuthor() async throws -> String {
        let author = try await irohManager.getAuthor()
        return author
    }

    public func writeFile(_ namespace: UNamespaceId, _ path: String, _ data: Data) async throws -> UHash {
        let hash = try await irohManager.writeFile(
            namespace: namespace,
            path: path,
            data: data
        )
        return hash
    }

    public func readFile(_ namespace: UNamespaceId, _ path: String) async throws -> Data {
        let data = try await irohManager.readFile(namespace: namespace, path: path)
        return data
    }

    public func readFileHash(_ hash: UHash) async throws -> Data {
        let data = try await irohManager.readFileHash(hash: hash)
        return data
    }

    public func share(_ namespace: String) async throws -> UDocTicket {
        let ticket = try! await irohManager.share(namespace: namespace)
        return ticket
    }

    public func `import`(_ ticket: String) async throws -> UNamespaceId {
        let namespaceId = try! await irohManager.import(ticket: ticket)
        return namespaceId
    }

    public func listen(_ namespace: UNamespaceId) async throws {
        try await irohManager.listen(namespace: namespace)
    }
}
