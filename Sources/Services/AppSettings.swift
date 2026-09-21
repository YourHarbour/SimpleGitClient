import Foundation

/// 全局偏好(UserDefaults)。目前只有自动 fetch。
enum AppSettings {
    private static let store = UserDefaults.standard
    private static let enabledKey = "SimpleGitClient.autoFetch.enabled"
    private static let intervalKey = "SimpleGitClient.autoFetch.interval"

    /// 后台自动 fetch —— 默认开(GitKraken / Fork / GitHub Desktop 都是默认开)。
    static var autoFetchEnabled: Bool {
        get { store.object(forKey: enabledKey) as? Bool ?? true }
        set { store.set(newValue, forKey: enabledKey) }
    }

    /// 自动 fetch 间隔(秒),默认 5 分钟。
    static var autoFetchInterval: TimeInterval {
        get {
            let v = store.object(forKey: intervalKey) as? Double ?? 300
            return max(60, v)
        }
        set { store.set(newValue, forKey: intervalKey) }
    }

    static let autoFetchIntervalChoices: [TimeInterval] = [60, 300, 600, 1800]

    static func intervalLabel(_ seconds: TimeInterval) -> String {
        seconds < 3600
            ? "\(Int(seconds / 60)) min"
            : "\(Int(seconds / 3600)) h"
    }
}
