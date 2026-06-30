import Foundation

class FileWatcherService {
    private var stream: FSEventStreamRef?
    private let callback: () -> Void
    private var path: String
    private var isWatching = false
    
    init(path: String, callback: @escaping () -> Void) {
        self.path = path
        self.callback = callback
    }
    
    deinit {
        stop()
    }
    
    func start() {
        guard !isWatching else { return }
        let pathsToWatch = [path] as CFArray
        var context = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )
        let flags: FSEventStreamCreateFlags = UInt32(
            kFSEventStreamCreateFlagUseCFTypes |
            kFSEventStreamCreateFlagFileEvents |
            kFSEventStreamCreateFlagNoDefer
        )
        guard let stream = FSEventStreamCreate(
            nil,
            { (_, info, numEvents, _, _, _) in
                guard let info = info else { return }
                let watcher = Unmanaged<FileWatcherService>.fromOpaque(info).takeUnretainedValue()
                if numEvents > 0 {
                    DispatchQueue.main.async {
                        watcher.callback()
                    }
                }
            },
            &context,
            pathsToWatch,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            1.0,
            flags
        ) else { return }
        
        self.stream = stream
        FSEventStreamSetDispatchQueue(stream, DispatchQueue.global(qos: .utility))
        FSEventStreamStart(stream)
        isWatching = true
    }
    
    func stop() {
        guard let stream = stream, isWatching else { return }
        FSEventStreamStop(stream)
        FSEventStreamInvalidate(stream)
        FSEventStreamRelease(stream)
        self.stream = nil
        isWatching = false
    }
    
    func updatePath(_ newPath: String) {
        stop()
        path = newPath
        start()
    }
}
