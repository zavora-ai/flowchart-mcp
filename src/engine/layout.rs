//! Layered auto-layout. Assigns each node a rank (longest path from a source
//! along edge direction) and an order within the rank, then maps ranks/orders
//! to pixel coordinates honoring the flow [`Direction`].
//!
//! When the chart contains swimlane containers, layout becomes lane-aware: the
//! flow runs along the main axis (by rank) while lanes form full-length bands
//! along the cross axis, with nodes stacked inside their lane. This produces
//! proper cross-functional diagrams instead of loose boxes.

use std::collections::HashMap;

use super::{ContainerKind, Direction, Flowchart};

/// Default node box size in pixels.
pub const NODE_W: f64 = 170.0;
pub const NODE_H: f64 = 60.0;
/// Gap between adjacent ranks and between siblings within a rank.
pub const RANK_GAP: f64 = 70.0;
pub const SIBLING_GAP: f64 = 32.0;
pub const MARGIN: f64 = 24.0;
/// Lane title-bar thickness (reserved at the main-axis start of each lane).
pub const LANE_TITLE: f64 = 30.0;
/// Cross-axis padding inside a lane band.
pub const LANE_PAD: f64 = 18.0;

/// Computed geometry for a node (top-left origin).
#[derive(Debug, Clone, Copy)]
pub struct Box {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Absolute geometry for a swimlane band.
#[derive(Debug, Clone)]
pub struct LaneGeom {
    pub id: String,
    pub b: Box,
}

/// Layout result: per-node boxes (keyed by node id), swimlane bands, and
/// overall canvas size.
pub struct Layout {
    pub boxes: HashMap<String, Box>,
    pub lanes: Vec<LaneGeom>,
    pub width: f64,
    pub height: f64,
}

impl Layout {
    pub fn get(&self, id: &str) -> Box {
        self.boxes.get(id).copied().unwrap_or(Box {
            x: MARGIN,
            y: MARGIN,
            w: NODE_W,
            h: NODE_H,
        })
    }
}

/// Assign ranks by longest path from any source node. Cycles are handled by a
/// visited guard so each node is relaxed a bounded number of times.
fn rank_nodes(fc: &Flowchart) -> HashMap<String, usize> {
    let mut rank: HashMap<String, usize> = fc.nodes.iter().map(|n| (n.id.clone(), 0)).collect();
    let n = fc.nodes.len();
    for _ in 0..n {
        let mut changed = false;
        for e in &fc.edges {
            if let (Some(&rf), Some(&rt)) = (rank.get(&e.from), rank.get(&e.to)) {
                if rt < rf + 1 {
                    rank.insert(e.to.clone(), rf + 1);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    rank
}

/// Ordered ids of swimlane containers (these become lane bands).
fn lane_ids(fc: &Flowchart) -> Vec<String> {
    fc.subgraphs
        .iter()
        .filter(|s| s.kind == ContainerKind::Swimlane)
        .map(|s| s.id.clone())
        .collect()
}

/// Compute pixel geometry for every node.
pub fn compute(fc: &Flowchart) -> Layout {
    let rank = rank_nodes(fc);
    let max_rank = rank.values().copied().max().unwrap_or(0);
    let lanes = lane_ids(fc);
    if lanes.is_empty() {
        compute_plain(fc, &rank, max_rank)
    } else {
        compute_laned(fc, &rank, max_rank, &lanes)
    }
}

/// Lane-aware layout: flow along the main axis by rank, lanes as cross-axis
/// bands with nodes stacked inside their lane at each rank.
fn compute_laned(
    fc: &Flowchart,
    rank: &HashMap<String, usize>,
    max_rank: usize,
    lanes: &[String],
) -> Layout {
    let vertical = fc.direction.is_vertical();
    let main_node = if vertical { NODE_H } else { NODE_W };
    let cross_node = if vertical { NODE_W } else { NODE_H };
    let main_step = main_node + RANK_GAP;
    let cross_step = cross_node + SIBLING_GAP;

    // node id -> lane index (first swimlane that lists it; default 0).
    let mut node_lane: HashMap<&str, usize> = HashMap::new();
    for (li, lid) in lanes.iter().enumerate() {
        if let Some(sg) = fc.subgraphs.iter().find(|s| &s.id == lid) {
            for m in &sg.members {
                node_lane.entry(m.as_str()).or_insert(li);
            }
        }
    }

    let nlanes = lanes.len();
    // groups[lane][rank] = node ids (insertion order).
    let mut groups: Vec<Vec<Vec<String>>> = vec![vec![Vec::new(); max_rank + 1]; nlanes];
    for node in &fc.nodes {
        let li = *node_lane.get(node.id.as_str()).unwrap_or(&0);
        let r = *rank.get(&node.id).unwrap_or(&0);
        groups[li][r].push(node.id.clone());
    }

    // Lane cross-extent = busiest rank in that lane.
    let lane_rows: Vec<usize> = groups
        .iter()
        .map(|g| g.iter().map(|v| v.len()).max().unwrap_or(0).max(1))
        .collect();
    let lane_cross: Vec<f64> = lane_rows
        .iter()
        .map(|&r| r as f64 * cross_step - SIBLING_GAP + 2.0 * LANE_PAD)
        .collect();

    // Cross-axis start of each lane band.
    let mut lane_start = vec![0.0; nlanes];
    let mut acc = MARGIN;
    for i in 0..nlanes {
        lane_start[i] = acc;
        acc += lane_cross[i];
    }
    let cross_end = acc + MARGIN;

    // Main-axis full length (covers the lane title bar + all ranks).
    let main_full = LANE_TITLE + max_rank as f64 * main_step + main_node + LANE_PAD;
    let main_total = MARGIN + main_full + MARGIN;

    let mut boxes = HashMap::new();
    for li in 0..nlanes {
        for r in 0..=max_rank {
            let ids = &groups[li][r];
            let k = ids.len();
            if k == 0 {
                continue;
            }
            let group_cross = k as f64 * cross_step - SIBLING_GAP;
            let inner = lane_cross[li] - 2.0 * LANE_PAD;
            let off = ((inner - group_cross).max(0.0)) / 2.0;
            for (i, id) in ids.iter().enumerate() {
                let main_pos = MARGIN + LANE_TITLE + r as f64 * main_step;
                let cross_pos = lane_start[li] + LANE_PAD + off + i as f64 * cross_step;
                let (x, y) = if vertical {
                    (cross_pos, main_pos)
                } else {
                    (main_pos, cross_pos)
                };
                boxes.insert(
                    id.clone(),
                    Box {
                        x,
                        y,
                        w: NODE_W,
                        h: NODE_H,
                    },
                );
            }
        }
    }

    // Flip the main axis for BT / RL.
    if matches!(fc.direction, Direction::BT | Direction::RL) {
        for b in boxes.values_mut() {
            if vertical {
                b.y = main_total - b.y - b.h;
            } else {
                b.x = main_total - b.x - b.w;
            }
        }
    }

    // Lane band geometry (spans the full main axis).
    let lanes_geom: Vec<LaneGeom> = (0..nlanes)
        .map(|li| {
            let b = if vertical {
                Box {
                    x: lane_start[li],
                    y: MARGIN,
                    w: lane_cross[li],
                    h: main_full,
                }
            } else {
                Box {
                    x: MARGIN,
                    y: lane_start[li],
                    w: main_full,
                    h: lane_cross[li],
                }
            };
            LaneGeom {
                id: lanes[li].clone(),
                b,
            }
        })
        .collect();

    let (width, height) = if vertical {
        (cross_end, main_total)
    } else {
        (main_total, cross_end)
    };

    Layout {
        boxes,
        lanes: lanes_geom,
        width: width.max(NODE_W + MARGIN * 2.0),
        height: height.max(NODE_H + MARGIN * 2.0),
    }
}

/// Plain layered layout (no swimlanes).
fn compute_plain(fc: &Flowchart, rank: &HashMap<String, usize>, max_rank: usize) -> Layout {
    let mut ranks: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node in &fc.nodes {
        let r = *rank.get(&node.id).unwrap_or(&0);
        ranks[r].push(node.id.clone());
    }

    let vertical = fc.direction.is_vertical();
    let widest = ranks.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    let cross_span = widest as f64 * cross_step(vertical);
    let mut boxes = HashMap::new();

    for (r, ids) in ranks.iter().enumerate() {
        let count = ids.len().max(1);
        let used = count as f64 * cross_step(vertical) - cross_gap(vertical);
        let start = MARGIN + (cross_span - cross_gap(vertical) - used).max(0.0) / 2.0;
        for (i, id) in ids.iter().enumerate() {
            let main = MARGIN + r as f64 * main_step(vertical);
            let cross = start + i as f64 * cross_step(vertical);
            let (x, y) = if vertical { (cross, main) } else { (main, cross) };
            boxes.insert(
                id.clone(),
                Box {
                    x,
                    y,
                    w: NODE_W,
                    h: NODE_H,
                },
            );
        }
    }

    let main_extent = MARGIN * 2.0 + (max_rank as f64) * main_step(vertical) + main_box(vertical);
    if matches!(fc.direction, Direction::BT | Direction::RL) {
        for b in boxes.values_mut() {
            if vertical {
                b.y = main_extent - b.y - b.h;
            } else {
                b.x = main_extent - b.x - b.w;
            }
        }
    }

    let (width, height) = if vertical {
        (cross_span + MARGIN * 2.0, main_extent)
    } else {
        (main_extent, cross_span + MARGIN * 2.0)
    };

    Layout {
        boxes,
        lanes: Vec::new(),
        width: width.max(NODE_W + MARGIN * 2.0),
        height: height.max(NODE_H + MARGIN * 2.0),
    }
}

fn main_step(vertical: bool) -> f64 {
    if vertical {
        NODE_H + RANK_GAP
    } else {
        NODE_W + RANK_GAP
    }
}
fn main_box(vertical: bool) -> f64 {
    if vertical {
        NODE_H
    } else {
        NODE_W
    }
}
fn cross_step(vertical: bool) -> f64 {
    if vertical {
        NODE_W + SIBLING_GAP
    } else {
        NODE_H + SIBLING_GAP
    }
}
fn cross_gap(vertical: bool) -> f64 {
    let _ = vertical;
    SIBLING_GAP
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Direction, Shape};

    fn chain() -> Flowchart {
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_node("a", "A", Shape::Stadium).unwrap();
        fc.add_node("b", "B", Shape::Rectangle).unwrap();
        fc.add_node("c", "C", Shape::Rectangle).unwrap();
        fc.add_edge("a", "b", None, crate::engine::LineStyle::Solid, true)
            .unwrap();
        fc.add_edge("b", "c", None, crate::engine::LineStyle::Solid, true)
            .unwrap();
        fc
    }

    #[test]
    fn ranks_stack_down_for_tb() {
        let fc = chain();
        let l = compute(&fc);
        assert!(l.get("a").y < l.get("b").y);
        assert!(l.get("b").y < l.get("c").y);
    }

    #[test]
    fn lr_lays_out_horizontally() {
        let mut fc = chain();
        fc.set_direction(Direction::LR);
        let l = compute(&fc);
        assert!(l.get("a").x < l.get("b").x);
        assert!(l.get("b").x < l.get("c").x);
    }

    #[test]
    fn bt_inverts_main_axis() {
        let mut fc = chain();
        fc.set_direction(Direction::BT);
        let l = compute(&fc);
        assert!(l.get("a").y > l.get("c").y);
    }

    #[test]
    fn swimlanes_form_stacked_bands() {
        let mut fc = chain();
        fc.set_direction(Direction::LR);
        fc.add_subgraph("L1", "Lane 1", vec!["a".into(), "c".into()], ContainerKind::Swimlane, None, None)
            .unwrap();
        fc.add_subgraph("L2", "Lane 2", vec!["b".into()], ContainerKind::Swimlane, None, None)
            .unwrap();
        let l = compute(&fc);
        assert_eq!(l.lanes.len(), 2);
        // Lane 2 sits below lane 1 (stacked on the cross axis).
        assert!(l.lanes[0].b.y < l.lanes[1].b.y);
        // Both lanes span the same full main-axis length.
        assert!((l.lanes[0].b.w - l.lanes[1].b.w).abs() < 1.0);
        // Node b (lane 2) is in a different cross band than a (lane 1).
        assert!(l.get("a").y < l.get("b").y);
    }
}
