//! Layered auto-layout. Assigns each node a rank (longest path from a source
//! along edge direction) and an order within the rank, then maps ranks/orders
//! to pixel coordinates honoring the flow [`Direction`].

use std::collections::HashMap;

use super::{Direction, Flowchart};

/// Default node box size in pixels.
pub const NODE_W: f64 = 120.0;
pub const NODE_H: f64 = 48.0;
/// Gap between adjacent ranks and between siblings within a rank.
pub const RANK_GAP: f64 = 60.0;
pub const SIBLING_GAP: f64 = 40.0;
pub const MARGIN: f64 = 20.0;

/// Computed geometry for a node (top-left origin).
#[derive(Debug, Clone, Copy)]
pub struct Box {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Layout result: per-node boxes (keyed by node id) and overall canvas size.
pub struct Layout {
    pub boxes: HashMap<String, Box>,
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
    // Relax edges up to |nodes| times (Bellman-Ford style longest path on a DAG;
    // bounded iteration makes it safe for cyclic graphs too).
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

/// Compute pixel geometry for every node.
pub fn compute(fc: &Flowchart) -> Layout {
    let rank = rank_nodes(fc);

    // Group node ids by rank, preserving insertion order for stable layout.
    let max_rank = rank.values().copied().max().unwrap_or(0);
    let mut ranks: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node in &fc.nodes {
        let r = *rank.get(&node.id).unwrap_or(&0);
        ranks[r].push(node.id.clone());
    }

    let vertical = fc.direction.is_vertical();
    let widest = ranks.iter().map(|r| r.len()).max().unwrap_or(1).max(1);

    // Cross-axis extent (within a rank) and main-axis extent (across ranks).
    let cross_span = widest as f64 * (cross_step(vertical));
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

    // Flip the main axis for BT / RL so growth runs the requested way.
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
}
