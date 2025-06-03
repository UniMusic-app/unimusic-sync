#if canImport(SystemConfiguration)
    import SystemConfiguration
#endif

import Foundation
@_exported import UniFFI

public class UniMusicSync {
    public let irohManager: IrohManager

    public init(_ path: String) async throws {
        let irohFactory = IrohFactory()
        irohManager = try await irohFactory.irohManager(path: path)
    }

    public func shutdown() async throws {
        try await irohManager.shutdown()
    }

    public func createNamespace() async throws -> UNamespaceId {
        let namespace = try await irohManager.createNamespace()
        return namespace
    }

    public func deleteNamespace(namespace: UNamespaceId) async throws {
        try await irohManager.deleteNamespace(namespace: namespace)
    }

    public func getAuthor() async throws -> String {
        let author = try await irohManager.getAuthor()
        return author
    }

    public func getNodeId() async -> UNodeId {
        let nodeId = await irohManager.getNodeId()
        return nodeId
    }

    public func getKnownNodes() async -> [UNodeId] {
        let knownNodes = await irohManager.getKnownNodes()
        return knownNodes
    }

    public func getFiles(_ namespace: UNamespaceId) async throws -> [UEntry] {
        let files = try await irohManager.getFiles(namespace: namespace)
        return files
    }

    public func writeFile(_ namespace: UNamespaceId, _ path: String, _ data: Data) async throws -> UHash {
        let hash = try await irohManager.writeFile(
            namespace: namespace,
            path: path,
            data: data
        )
        return hash
    }

    public func deleteFile(_ namespace: UNamespaceId, _ path: String) async throws -> UInt32 {
        let deletedFiles = try await irohManager.deleteFile(namespace: namespace, path: path)
        return deletedFiles
    }

    public func readFile(_ namespace: UNamespaceId, _ path: String) async throws -> Data {
        let data = try await irohManager.readFile(namespace: namespace, path: path)
        return data
    }

    public func readFileHash(_ hash: UHash) async throws -> Data {
        let data = try await irohManager.readFileHash(hash: hash)
        return data
    }

    public func export(_ namespace: UNamespaceId, _ path: String, _ destination: String) async throws {
        try await irohManager.export(namespace: namespace, path: path, destination: destination)
    }

    public func exportHash(_ hash: UHash, _ destination: String) async throws {
        try await irohManager.exportHash(hash: hash, destination: destination)
    }

    public func share(_ namespace: String) async throws -> UDocTicket {
        let ticket = try await irohManager.share(namespace: namespace)
        return ticket
    }

    public func `import`(_ ticket: String) async throws -> UNamespaceId {
        let namespaceId = try await irohManager.import(ticket: ticket)
        return namespaceId
    }

    public func sync(_ namespace: UNamespaceId) async throws {
        try await irohManager.sync(namespace: namespace)
    }

    public func reconnect() async {
        await irohManager.reconnect()
    }
}
