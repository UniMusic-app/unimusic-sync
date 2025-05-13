/**
 * FIXME: This file should move out of Android Tests ASAP. It only exists here because I haven't yet
 * figured out how to build and link the platform-native binaries via JNI just yet and this works.
 * See https://github.com/willir/cargo-ndk-android-gradle/issues/12.
 *
 * This solution is STUPIDLY INEFFICIENT and will probably contribute to global climate change since
 * an Android emulator uses like two whole CPU cores when idling.
 */
package app.unimusic.sync

import org.junit.Assert.*

class UniMusicSyncTest {}
