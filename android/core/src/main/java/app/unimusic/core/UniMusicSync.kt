package app.unimusic.core

import uniffi.unimusic_sync.IrohFactory;
import uniffi.unimusic_sync.IrohManager

class UniMusicSync {
    public var irohManager: IrohManager;
    constructor(irohManager: IrohManager) {
        this.irohManager = irohManager
    }

    companion object {
        suspend fun create(path: String): UniMusicSync {
            val irohFactory = IrohFactory();
            val irohManager = irohFactory.irohManager(path)
            val instance = UniMusicSync(irohManager)
            return instance
        }
    }
}

