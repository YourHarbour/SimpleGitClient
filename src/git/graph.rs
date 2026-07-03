//! Commit-graph lane layout — direct port of `Models/GitGraph.swift`.
//!
//! Assigns every commit a lane and produces gap-free connecting edges. Requires
//! the commit list to be in `--topo-order` (parents after children).

use super::models::GitCommit;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeKind {
    /// row-top → row-middle (merging into the node)
    Top,
    /// row-middle → row-bottom (branching out to a parent)
    Bottom,
    /// row-top → row-bottom (an unrelated lane passing straight through)
    Through,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub from_lane: usize,
    pub to_lane: usize,
    /// Lane index whose palette color paints this edge.
    pub color_lane: usize,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphRow {
    /// commit hash, or "WIP"
    pub id: String,
    pub commit: Option<GitCommit>,
    pub is_wip: bool,
    pub lane: usize,
    pub is_merge_node: bool,
    pub edges: Vec<GraphEdge>,
}

impl GraphRow {
    pub fn refs(&self) -> &[super::models::GitRef] {
        match &self.commit {
            Some(c) => &c.refs,
            None => &[],
        }
    }
}

fn first_free_lane(lanes: &mut Vec<Option<String>>) -> usize {
    if let Some(idx) = lanes.iter().position(|x| x.is_none()) {
        idx
    } else {
        lanes.push(None);
        lanes.len() - 1
    }
}

/// Compute the base rows (no WIP row) plus the maximum lane index used.
pub fn calculate(commits: &[GitCommit]) -> (Vec<GraphRow>, usize) {
    if commits.is_empty() {
        return (Vec::new(), 0);
    }

    // lanes[j] = the hash lane j is currently waiting to render.
    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut rows: Vec<GraphRow> = Vec::new();
    let mut max_lane = 0usize;

    for commit in commits {
        let before = lanes.clone();

        // 1. lane for this commit
        let lane_c = match lanes.iter().position(|x| x.as_deref() == Some(commit.id.as_str())) {
            Some(idx) => idx,
            None => first_free_lane(&mut lanes),
        };
        lanes[lane_c] = Some(commit.id.clone());

        // 2. lanes merging into this commit (iterate the `before` snapshot)
        let mut incoming: Vec<usize> = Vec::new();
        for (j, slot) in before.iter().enumerate() {
            if slot.as_deref() == Some(commit.id.as_str()) {
                incoming.push(j);
            }
        }
        for &j in &incoming {
            if j != lane_c {
                lanes[j] = None;
            }
        }

        // 3. assign lanes to parents
        let mut parent_lanes: Vec<usize> = Vec::new();
        if let Some(first_parent) = commit.parent_hashes.first() {
            lanes[lane_c] = Some(first_parent.clone());
            parent_lanes.push(lane_c);
            for parent in commit.parent_hashes.iter().skip(1) {
                let pj = match lanes.iter().position(|x| x.as_deref() == Some(parent.as_str())) {
                    Some(idx) => idx,
                    None => first_free_lane(&mut lanes),
                };
                lanes[pj] = Some(parent.clone());
                parent_lanes.push(pj);
            }
        } else {
            lanes[lane_c] = None; // root commit
        }

        let after = lanes.clone();

        // 4. build edges
        let mut edges: Vec<GraphEdge> = Vec::new();
        // 4a. incoming (top halves)
        for &j in &incoming {
            edges.push(GraphEdge { from_lane: j, to_lane: lane_c, color_lane: j, kind: EdgeKind::Top });
        }
        // 4b. branch to parents (bottom halves)
        for (idx, &pj) in parent_lanes.iter().enumerate() {
            let color_lane = if idx == 0 { lane_c } else { pj };
            edges.push(GraphEdge { from_lane: lane_c, to_lane: pj, color_lane, kind: EdgeKind::Bottom });
        }
        // 4c. unrelated lanes passing through
        for (j, slot) in before.iter().enumerate() {
            if j == lane_c {
                continue;
            }
            if slot.is_some()
                && slot.as_deref() != Some(commit.id.as_str())
                && j < after.len()
                && after[j].is_some()
            {
                edges.push(GraphEdge { from_lane: j, to_lane: j, color_lane: j, kind: EdgeKind::Through });
            }
        }

        rows.push(GraphRow {
            id: commit.id.clone(),
            commit: Some(commit.clone()),
            is_wip: false,
            lane: lane_c,
            is_merge_node: commit.is_merge(),
            edges,
        });

        max_lane = max_lane.max(lanes.len().saturating_sub(1)).max(lane_c);
        for &pj in &parent_lanes {
            max_lane = max_lane.max(pj);
        }
    }

    (rows, max_lane)
}

/// Insert the top WIP row when the working tree has changes — mirrors
/// `GraphViewModel.rows`.
pub fn build_display_rows(base: &[GraphRow], change_count: usize) -> Vec<GraphRow> {
    if change_count == 0 {
        return base.to_vec();
    }
    if base.is_empty() {
        return vec![GraphRow {
            id: "WIP".into(),
            commit: None,
            is_wip: true,
            lane: 0,
            is_merge_node: false,
            edges: Vec::new(),
        }];
    }

    let head_lane = base[0].lane;
    let mut result = base.to_vec();
    // continue the line up from HEAD into the WIP node
    result[0].edges.push(GraphEdge {
        from_lane: head_lane,
        to_lane: head_lane,
        color_lane: head_lane,
        kind: EdgeKind::Top,
    });

    let wip = GraphRow {
        id: "WIP".into(),
        commit: None,
        is_wip: true,
        lane: head_lane,
        is_merge_node: false,
        edges: vec![GraphEdge {
            from_lane: head_lane,
            to_lane: head_lane,
            color_lane: head_lane,
            kind: EdgeKind::Bottom,
        }],
    };

    let mut out = Vec::with_capacity(result.len() + 1);
    out.push(wip);
    out.extend(result);
    out
}
