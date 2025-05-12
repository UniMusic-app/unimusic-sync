import SystemConfiguration
import UniFFI

public class UniMusicSync {
    public let irohManager: IrohManager

    public init(_ path: String) async throws {
        let irohFactory = IrohFactory()
        irohManager = try! await irohFactory.irohManager(path: path)
    }
}
