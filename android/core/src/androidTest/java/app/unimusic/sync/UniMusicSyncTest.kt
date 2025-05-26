package app.unimusic.sync

import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry
import kotlin.time.Duration.Companion.minutes
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.joinAll
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.unimusic_sync.UHash
import uniffi.unimusic_sync.UNamespaceId

val MODIFIED_FILE = Pair("dog_breeds.txt", "German Shepherd, Husky, Pomeranian".encodeToByteArray())
val TEST_FILES =
    mapOf(
        "dog_breeds.txt" to
            "American Eskimo Dog, Husky, Cocker Spaniel, Pomeranian".encodeToByteArray(),
        "bing chilling.txt" to
            """
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
    """
                .encodeToByteArray(),
    )

class UniMusicSyncTest {

  // This android test can be REALLY flake-y...
  // This is one of the reasons that this tests 3 concurrent connections compared to 5 in other
  // tests
  @Test
  fun testConnection() =
      runTest(timeout = 15.minutes) {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val tempDir = context.filesDir.resolve("iroh")

        if (tempDir.exists()) {
          tempDir.deleteRecursively()
        }

        suspend fun <T> maybeRetry(times: Int = 5, block: suspend () -> T): T {
          var lastError: Throwable? = null
          for (i in 0..times) {
            try {
              return block()
            } catch (e: Throwable) {
              Log.d("UniMusicSyncTest", "[$i] Failed to run a block, retrying in 5s")
              lastError = e
              delay(5_000)
              continue
            }
          }
          throw lastError!!
        }

        suspend fun mockClient(dir: String): UniMusicSync {
          val path = tempDir.resolve(dir).path
          val uniMusicSync = UniMusicSync.create(path)
          return uniMusicSync
        }

        suspend fun compareFileContents(
            client: UniMusicSync,
            fileHash: UHash,
            contents: ByteArray?
        ) {
          val file = runCatching { client.readFileHash(fileHash) }.getOrNull()
          assertEquals(contents?.toString(Charsets.UTF_8), file?.toString(Charsets.UTF_8))
        }

        suspend fun compareFileContents(
            client: UniMusicSync,
            namespace: UNamespaceId,
            path: String,
            contents: ByteArray?
        ) {
          val file = runCatching { client.readFile(namespace, path) }.getOrNull()
          assertEquals(contents?.toString(Charsets.UTF_8), file?.toString(Charsets.UTF_8))
        }

        val provider = mockClient("provider")

        Log.d("UniMusicSyncTest", "[provider]: create namespace")
        val namespace = provider.createNamespace()

        var fileHashes = arrayListOf<UHash>()
        Log.d("UniMusicSyncTest", "[provider]: write files")
        TEST_FILES.forEach { (path, contents) ->
          val fileHash = provider.writeFile(namespace, path, contents)
          compareFileContents(provider, fileHash, contents)
          compareFileContents(provider, namespace, path, contents)
          fileHashes.add(fileHash)
        }

        Log.d("UniMusicSyncTest", "[provider]: share ticket")
        val ticket = provider.share(namespace)

        // region Test 3 concurrent connections
        val jobs =
            List(3) { i ->
              launch(context = Dispatchers.IO) {
                val receiver = UniMusicSync.create(tempDir.resolve("receiver-$i").path)

                Log.d("UniMusicSyncTest", "[receiver $i]: make sure files are empty before import")

                fileHashes.forEach { fileHash -> compareFileContents(receiver, fileHash, null) }
                TEST_FILES.forEach { (path, _) ->
                  compareFileContents(receiver, namespace, path, null)
                }

                Log.d(
                    "UniMusicSyncTest",
                    "[receiver $i]: make sure imported namespace is equal to provider one")
                val importedNamespace = receiver.import(ticket)
                assertEquals(importedNamespace, namespace)

                Log.d("UniMusicSyncTest", "[receiver $i]: make sure files get properly imported")
                for ((j, entry) in TEST_FILES.entries.withIndex()) {
                  val (path, contents) = entry
                  Log.d("UniMusicSyncTest", "[receiver $i]: make sure $path gets properly imported")
                  compareFileContents(receiver, fileHashes[j], contents)
                  compareFileContents(receiver, namespace, path, contents)
                }

                Log.d("UniMusicSyncTest", "[receiver $i]: shutdown")
                receiver.shutdown()
              }
            }

        jobs.joinAll()
        Log.d("UniMusicSyncTest", "[jobs]: finished")
        // endregion

        Log.d("UniMusicSyncTest", "[provider]: modify ${MODIFIED_FILE.first}")
        val fileHash = provider.writeFile(namespace, MODIFIED_FILE.first, MODIFIED_FILE.second)
        compareFileContents(provider, fileHash, MODIFIED_FILE.second)

        delay(5_000)

        // region Make sure nodes properly reconnect and sync
        val jobs2 =
            List(3) { i ->
              launch(context = Dispatchers.IO) {
                Log.d("UniMusicSyncTest", "[receiver $i]: recreate")
                val receiver = UniMusicSync.create(tempDir.resolve("receiver-$i").path)

                Log.d("UniMusicSyncTest", "[receiver $i]: make sure all files are still there")
                for ((j, entry) in TEST_FILES.entries.withIndex()) {
                  val (path, contents) = entry
                  compareFileContents(receiver, fileHashes[j], contents)
                  compareFileContents(receiver, namespace, path, contents)
                }

                Log.d("UniMusicSyncTest", "[receiver $i]: reconnect")
                var retries = 0
                while (!receiver.getKnownNodes().contains(provider.getNodeId())) {
                  if (retries >= 15) {
                    Log.d("UniMusicSyncTest", "[receiver $i]: missing provider node - giving up")
                    break
                  } else if (retries > 0) {
                    Log.d(
                        "UniMusicSyncTest",
                        "[receiver $i]: missing provider node - trying again in 10s")
                    delay(10_000)
                  }

                  receiver.reconnect()
                  retries += 1
                }

                Log.d("UniMusicSyncTest", "[receiver $i]: sync")
                retries = 0
                maybeRetry(15) { receiver.sync(namespace) }
                Log.d("UniMusicSyncTest", "[receiver $i]: synced, waiting 5s to propagate")

                delay(10_000)

                Log.d("UniMusicSyncTest", "[receiver $i]: make sure file got properly synced")
                compareFileContents(receiver, namespace, MODIFIED_FILE.first, MODIFIED_FILE.second)

                Log.d("UniMusicSyncTest", "[receiver $i]: shutdown")
              }
            }

        jobs2.joinAll()
        Log.d("UniMusicSyncTest", "[jobs2]: finished")
        // endregion

        Log.d("UniMusicSyncTest", "[provider]: shutdown")
        provider.shutdown()

        tempDir.deleteRecursively()
      }
}
