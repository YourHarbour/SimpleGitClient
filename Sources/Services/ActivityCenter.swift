import SwiftUI
import Observation

struct ActivityItem: Identifiable {
    let id: UUID
    let label: String
}

/// 后台操作进度中心。pull/push/fetch/clone 等运行期间在右下角显示一个进度提示。
@Observable
final class ActivityCenter {
    static let shared = ActivityCenter()
    private(set) var items: [ActivityItem] = []

    var current: String? { items.last?.label }
    var isBusy: Bool { !items.isEmpty }

    @discardableResult
    func begin(_ label: String) -> UUID {
        let id = UUID()
        Task { @MainActor in self.items.append(ActivityItem(id: id, label: label)) }
        return id
    }

    func end(_ id: UUID) {
        Task { @MainActor in self.items.removeAll { $0.id == id } }
    }

    /// 包裹一段异步操作:开始时显示进度,结束/出错后自动隐藏。
    func track<T>(_ label: String, _ work: () async throws -> T) async rethrows -> T {
        let id = begin(label)
        defer { end(id) }
        return try await work()
    }
}
