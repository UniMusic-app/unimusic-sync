package app.unimusic.sync

import uniffi.unimusic_sync.IrohFactory
import uniffi.unimusic_sync.IrohManager
import uniffi.unimusic_sync.UDocTicket
import uniffi.unimusic_sync.UEntry
import uniffi.unimusic_sync.UHash
import uniffi.unimusic_sync.UNamespaceId
import uniffi.unimusic_sync.UNodeId

@Suppress("unused")
class UniMusicSync(private val irohManager: IrohManager) {
  companion object {
    suspend fun create(path: String): UniMusicSync {
      val irohFactory = IrohFactory()
      val irohManager = irohFactory.irohManager(path)
      val instance = UniMusicSync(irohManager)
      return instance
    }
  }

  // This allows to attach a context, which results in much cleaner stack traces
  private inline fun <T> handleException(block: () -> T): T {
    return try {
      block()
    } catch (exception: Exception) {
      throw RuntimeException("Exception in handleException block", exception)
    }
  }

  fun close() {
    irohManager.close()
  }

  suspend fun shutdown() {
    handleException { irohManager.shutdown() }
  }

  suspend fun createNamespace(): UNamespaceId {
    val namespace = handleException { irohManager.createNamespace() }
    return namespace
  }

  suspend fun deleteNamespace(namespace: UNamespaceId) {
    handleException { irohManager.deleteNamespace(namespace) }
  }

  suspend fun getAuthor(): String {
    val author = handleException { irohManager.getAuthor() }
    return author
  }

  suspend fun getNodeId(): UNodeId {
    val nodeId = irohManager.getNodeId()
    return nodeId
  }

  suspend fun getKnownNodes(): List<UNodeId> {
    val knownNodes = handleException { irohManager.getKnownNodes() }
    return knownNodes
  }

  suspend fun getFiles(namespace: UNamespaceId): List<UEntry> {
    val files = handleException { irohManager.getFiles(namespace) }
    return files
  }

  suspend fun writeFile(namespace: UNamespaceId, path: String, data: ByteArray): UHash {
    val hash = handleException { irohManager.writeFile(namespace, path, data) }
    return hash
  }

  suspend fun deleteFile(namespace: UNamespaceId, path: String): UHash {
    val tombstoneHash = handleException { irohManager.deleteFile(namespace, path) }
    return tombstoneHash
  }

  suspend fun readFile(namespace: UNamespaceId, path: String): ByteArray {
    val data = handleException { irohManager.readFile(namespace, path) }
    return data
  }

  suspend fun readFileHash(hash: UHash): ByteArray {
    val data = handleException { irohManager.readFileHash(hash) }
    return data
  }

  suspend fun export(namespace: UNamespaceId, path: String, destination: String) {
    handleException { irohManager.export(namespace, path, destination) }
  }

  suspend fun exportHash(hash: UHash, destination: String) {
    handleException { irohManager.exportHash(hash, destination) }
  }

  suspend fun share(namespace: UNamespaceId): UDocTicket {
    val ticket = handleException { irohManager.share(namespace) }
    return ticket
  }

  suspend fun import(ticket: String): UNamespaceId {
    val namespace = handleException { irohManager.import(ticket) }
    return namespace
  }

  suspend fun sync(namespace: UNamespaceId) {
    handleException { irohManager.sync(namespace) }
  }

  suspend fun reconnect() {
    handleException { irohManager.reconnect() }
  }
}
