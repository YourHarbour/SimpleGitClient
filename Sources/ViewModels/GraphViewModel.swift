import SwiftUI
import Observation

@Observable
class GraphViewModel {
    /// 不含 WIP 的基础行(由 git log 计算得出)
    private(set) var baseRows: [GraphRow] = []
    private(set) var maxLane: Int = 0

    /// 工作目录改动数 —— 驱动顶部 WIP 行的出现与 "+N" 徽标
    var changeCount: Int = 0

    /// 当前选中行:"WIP" 或 commit hash
    var selectedRowID: String? = "WIP"
    var hoveredRowID: String?

    /// 最终渲染用的行(改动数 > 0 时在顶部插入 WIP 行,并把 HEAD 行接上一段上行连线)
    var rows: [GraphRow] {
        guard changeCount > 0 else { return baseRows }
        guard let head = baseRows.first else {
            // 仓库还没有提交,但工作区有改动 → 单独一个 WIP 节点
            return [GraphRow(id: "WIP", commit: nil, isWIP: true, lane: 0,
                             nodeColor: Theme.accentGreen, edges: [])]
        }
        var result = baseRows
        result[0].edges.append(
            GraphEdge(fromLane: head.lane, toLane: head.lane,
                      color: Theme.branchColor(at: head.lane), kind: .top)
        )
        let wip = GraphRow(
            id: "WIP", commit: nil, isWIP: true,
            lane: head.lane, nodeColor: Theme.accentGreen,
            edges: [GraphEdge(fromLane: head.lane, toLane: head.lane,
                              color: Theme.branchColor(at: head.lane), kind: .bottom)]
        )
        return [wip] + result
    }

    func updateCommits(_ commits: [GitCommit]) {
        let result = GraphLayoutCalculator.calculate(commits: commits)
        baseRows = result.rows
        maxLane = result.maxLane
    }
}
