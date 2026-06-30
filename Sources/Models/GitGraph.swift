import Foundation
import SwiftUI

// MARK: - Graph Edge

/// 一段连线。kind 决定它在行内的纵向覆盖范围:
/// - `.top`    : 行顶 → 行中(节点的上半段 / 汇入节点)
/// - `.bottom` : 行中 → 行底(节点的下半段 / 分叉到父)
/// - `.through`: 行顶 → 行底(与本行节点无关、纵向穿过的其它分支)
struct GraphEdge: Identifiable {
    let id = UUID()
    let fromLane: Int
    let toLane: Int
    let color: Color
    let kind: Kind

    enum Kind { case top, bottom, through }
}

// MARK: - Graph Row

/// 提交图中的一行。可以是真实提交,也可以是顶部的 WIP(工作目录)行。
struct GraphRow: Identifiable {
    let id: String          // commit hash,或 "WIP"
    let commit: GitCommit?  // WIP 行为 nil
    let isWIP: Bool
    let lane: Int           // 节点所在的 lane
    let nodeColor: Color
    var edges: [GraphEdge]

    var refs: [GitRef] { commit?.refs ?? [] }
}

// MARK: - Layout Calculator

/// 经典 git 图布局:为每个提交分配 lane,并生成连续(无断裂)的连线。
enum GraphLayoutCalculator {

    /// 计算不含 WIP 的"基础行"。WIP 行由 GraphViewModel 在渲染时按工作目录状态动态插入。
    static func calculate(commits: [GitCommit]) -> (rows: [GraphRow], maxLane: Int) {
        guard !commits.isEmpty else { return ([], 0) }

        // lanes[j] = 该 lane 当前正在"等待"出现的提交 hash(即某个已渲染子节点为其祖先预留的轨道)
        var lanes: [String?] = []
        var rows: [GraphRow] = []
        var maxLane = 0

        func firstFreeLane() -> Int {
            if let idx = lanes.firstIndex(where: { $0 == nil }) { return idx }
            lanes.append(nil)
            return lanes.count - 1
        }

        for commit in commits {
            let before = lanes

            // 1. 找到本提交的 lane
            let laneC: Int
            if let existing = lanes.firstIndex(where: { $0 == commit.id }) {
                laneC = existing
            } else {
                laneC = firstFreeLane()
            }
            lanes[laneC] = commit.id

            // 2. 收集所有"等待本提交"的 lane(多个子节点汇聚到这里)
            //    注意:遍历 before.indices —— firstFreeLane() 可能已扩展 lanes,
            //    用 lanes.indices 会越界访问 before。
            var incomingLanes: [Int] = []
            for j in before.indices where before[j] == commit.id {
                incomingLanes.append(j)
            }
            // 释放除 laneC 外的汇入 lane(它们的连线在本行终止于节点)
            for j in incomingLanes where j != laneC {
                lanes[j] = nil
            }

            // 3. 为父提交分配 lane
            var parentLanes: [Int] = []
            if let firstParent = commit.parentHashes.first {
                lanes[laneC] = firstParent          // 第一父继续占用本 lane
                parentLanes.append(laneC)
                for parent in commit.parentHashes.dropFirst() {
                    let pj: Int
                    if let existing = lanes.firstIndex(where: { $0 == parent }) {
                        pj = existing
                    } else {
                        pj = firstFreeLane()
                    }
                    lanes[pj] = parent
                    parentLanes.append(pj)
                }
            } else {
                lanes[laneC] = nil                  // 根提交,lane 结束
            }

            let after = lanes

            // 4. 生成本行连线
            var edges: [GraphEdge] = []

            // 4a. 汇入节点(上半段):所有 before == commit 的 lane → laneC
            for j in incomingLanes {
                edges.append(GraphEdge(fromLane: j, toLane: laneC, color: laneColor(j), kind: .top))
            }
            // 4b. 自节点分出(下半段):每个父 lane
            for (idx, pj) in parentLanes.enumerated() {
                // 第一父若与节点同 lane 即纵向延续;额外父为分叉曲线
                let color = idx == 0 ? laneColor(laneC) : laneColor(pj)
                edges.append(GraphEdge(fromLane: laneC, toLane: pj, color: color, kind: .bottom))
            }
            // 4c. 穿过本行的无关 lane(纵向直线)
            for j in before.indices {
                guard j != laneC else { continue }
                if before[j] != nil, before[j] != commit.id,
                   j < after.count, after[j] != nil {
                    edges.append(GraphEdge(fromLane: j, toLane: j, color: laneColor(j), kind: .through))
                }
            }

            let color = commit.isMerge ? Theme.accentBlue : laneColor(laneC)
            rows.append(GraphRow(id: commit.id, commit: commit, isWIP: false,
                                 lane: laneC, nodeColor: color, edges: edges))
            maxLane = max(maxLane, lanes.count - 1, laneC)
            for pj in parentLanes { maxLane = max(maxLane, pj) }
        }

        return (rows, maxLane)
    }

    static func laneColor(_ lane: Int) -> Color { Theme.branchColor(at: lane) }
}
