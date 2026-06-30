import SwiftUI

/// 中央提交图谱视图 — spec §7.A
struct CommitGraphView: View {
    @Environment(AppViewModel.self) private var appVM

    // 布局常量
    static let branchColWidth: CGFloat = 172
    static let rowHeight: CGFloat = 40
    static let laneSpacing: CGFloat = 18
    static let leadingPad: CGFloat = 16
    static let nodeRadius: CGFloat = 11

    private func graphColWidth(maxLane: Int) -> CGFloat {
        max(Self.leadingPad + CGFloat(maxLane + 1) * Self.laneSpacing + 12, 60)
    }

    var body: some View {
        if let repo = appVM.activeRepo {
            let gvm = repo.graphViewModel
            let gWidth = graphColWidth(maxLane: gvm.maxLane)

            VStack(spacing: 0) {
                columnHeader(graphWidth: gWidth)

                if repo.isLoading && gvm.baseRows.isEmpty {
                    ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if gvm.rows.isEmpty {
                    VStack(spacing: 8) {
                        Image(systemName: "tray").font(.system(size: 30)).foregroundStyle(Theme.textMuted)
                        Text("No commits yet").font(.system(size: 13)).foregroundStyle(Theme.textSecondary)
                        Text("Make your first commit from the panel on the right.")
                            .font(.system(size: 11)).foregroundStyle(Theme.textMuted)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    ScrollView {
                        LazyVStack(spacing: 0) {
                            ForEach(gvm.rows) { row in
                                CommitGraphRow(row: row, repo: repo,
                                               maxLane: gvm.maxLane, graphWidth: gWidth)
                            }
                        }
                    }
                    .background(Theme.bgApp)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.bgApp)
        }
    }

    // MARK: - Column header

    private func columnHeader(graphWidth: CGFloat) -> some View {
        HStack(spacing: 0) {
            Text("BRANCH / TAG")
                .frame(width: Self.branchColWidth, alignment: .leading)
                .padding(.leading, 16)
            Text("GRAPH")
                .frame(width: graphWidth, alignment: .leading)
            Text("COMMIT MESSAGE")
                .frame(maxWidth: .infinity, alignment: .leading)
            Image(systemName: "gearshape")
                .font(.system(size: 12))
                .foregroundStyle(Theme.textSecondary)
                .padding(.trailing, 14)
        }
        .font(.system(size: 11, weight: .semibold))
        .foregroundStyle(Theme.textSecondary)
        .frame(height: Theme.columnHeaderHeight)
        .background(Theme.bgPanel)
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 1) }
    }
}

// MARK: - Row

struct CommitGraphRow: View {
    let row: GraphRow
    let repo: RepoViewModel
    let maxLane: Int
    let graphWidth: CGFloat

    private var isSelected: Bool { repo.graphViewModel.selectedRowID == row.id }
    private var isHovered: Bool { repo.graphViewModel.hoveredRowID == row.id }

    var body: some View {
        HStack(spacing: 0) {
            // ① BRANCH / TAG
            branchTagColumn
                .frame(width: CommitGraphView.branchColWidth, alignment: .trailing)
                .padding(.trailing, 6)

            // ② GRAPH
            GraphCell(row: row, maxLane: maxLane)
                .frame(width: graphWidth, height: CommitGraphView.rowHeight)

            // ③ COMMIT MESSAGE
            messageColumn
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.trailing, 14)
        }
        .frame(height: CommitGraphView.rowHeight)
        .background(isSelected ? Theme.selectedRow : (isHovered ? Theme.bgHover : Color.clear))
        .contentShape(Rectangle())
        .onHover { repo.graphViewModel.hoveredRowID = $0 ? row.id : nil }
        .onTapGesture {
            if row.isWIP { repo.selectWIPRow() }
            else { Task { await repo.selectCommitRow(row.id) } }
        }
    }

    // MARK: Branch / tag pills

    @ViewBuilder
    private var branchTagColumn: some View {
        if !row.refs.isEmpty {
            HStack(spacing: 4) {
                ForEach(row.refs) { ref in refPill(ref) }
            }
        } else {
            Color.clear
        }
    }

    private func refPill(_ ref: GitRef) -> some View {
        HStack(spacing: 4) {
            if ref.isHead {
                Image(systemName: "checkmark").font(.system(size: 8, weight: .bold))
            } else if ref.type == .tag {
                Image(systemName: "tag.fill").font(.system(size: 8))
            }
            Text(ref.name)
                .font(.system(size: 11, weight: .medium))
                .lineLimit(1)
            if ref.type == .localBranch {
                Image(systemName: "desktopcomputer").font(.system(size: 8))
            } else if ref.type == .remoteBranch {
                Image(systemName: "cloud").font(.system(size: 8))
            }
        }
        .foregroundStyle(pillColor(ref))
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .background(
            Capsule().fill(pillColor(ref).opacity(0.12))
        )
        .overlay(
            Capsule().stroke(pillColor(ref).opacity(0.6), lineWidth: 1)
        )
    }

    private func pillColor(_ ref: GitRef) -> Color {
        if ref.isHead { return Theme.accentTeal }
        switch ref.type {
        case .localBranch: return Theme.accentTeal
        case .remoteBranch: return Theme.accentOrange
        case .tag: return Theme.accentBlue
        }
    }

    // MARK: Message

    @ViewBuilder
    private var messageColumn: some View {
        if row.isWIP {
            HStack(spacing: 8) {
                Text("// WIP")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.textSecondary)
                if repo.totalChanges > 0 {
                    Text("+ \(repo.totalChanges)")
                        .font(.system(size: 11, weight: .bold))
                        .foregroundStyle(Theme.accentGreen)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(Capsule().fill(Theme.accentGreen.opacity(0.15)))
                }
                Spacer(minLength: 0)
            }
        } else if let commit = row.commit {
            HStack(spacing: 8) {
                Text(commit.message)
                    .font(Theme.listFont)
                    .foregroundStyle(Theme.textPrimary)
                    .lineLimit(1)
                if !commit.truncatedBody.isEmpty {
                    Text(commit.truncatedBody)
                        .font(.system(size: 12))
                        .foregroundStyle(Theme.textSecondary)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
            }
        }
    }
}

// MARK: - Graph cell (lines + node)

struct GraphCell: View {
    let row: GraphRow
    let maxLane: Int

    private func laneX(_ lane: Int) -> CGFloat {
        CommitGraphView.leadingPad + CGFloat(lane) * CommitGraphView.laneSpacing + CommitGraphView.nodeRadius
    }

    var body: some View {
        let h = CommitGraphView.rowHeight
        ZStack {
            Canvas { ctx, size in
                let mid = size.height / 2
                for edge in row.edges {
                    let fromX = laneX(edge.fromLane)
                    let toX = laneX(edge.toLane)
                    var path = Path()
                    switch edge.kind {
                    case .through:
                        path.move(to: CGPoint(x: fromX, y: 0))
                        path.addLine(to: CGPoint(x: fromX, y: size.height))
                    case .top:
                        path.move(to: CGPoint(x: fromX, y: 0))
                        if edge.fromLane == edge.toLane {
                            path.addLine(to: CGPoint(x: toX, y: mid))
                        } else {
                            path.addCurve(to: CGPoint(x: toX, y: mid),
                                          control1: CGPoint(x: fromX, y: mid * 0.55),
                                          control2: CGPoint(x: toX, y: mid * 0.45))
                        }
                    case .bottom:
                        path.move(to: CGPoint(x: fromX, y: mid))
                        if edge.fromLane == edge.toLane {
                            path.addLine(to: CGPoint(x: toX, y: size.height))
                        } else {
                            path.addCurve(to: CGPoint(x: toX, y: size.height),
                                          control1: CGPoint(x: fromX, y: mid + (size.height - mid) * 0.45),
                                          control2: CGPoint(x: toX, y: mid + (size.height - mid) * 0.55))
                        }
                    }
                    ctx.stroke(path, with: .color(edge.color),
                               style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
                }
            }
            node
                .position(x: laneX(row.lane), y: h / 2)
        }
    }

    @ViewBuilder
    private var node: some View {
        if row.isWIP {
            Circle()
                .strokeBorder(style: StrokeStyle(lineWidth: 1.6, dash: [3, 2.5]))
                .foregroundStyle(Theme.accentGreen)
                .frame(width: 18, height: 18)
        } else if let commit = row.commit {
            Circle()
                .fill(Theme.bgElevated)
                .overlay(
                    Text(String(commit.author.prefix(1)).uppercased())
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(Theme.textSecondary)
                )
                .overlay(Circle().stroke(row.nodeColor, lineWidth: 2))
                .frame(width: CommitGraphView.nodeRadius * 2, height: CommitGraphView.nodeRadius * 2)
        }
    }
}
