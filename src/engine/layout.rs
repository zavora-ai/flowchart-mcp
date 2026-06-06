//! Layered auto-layout with content-aware node sizing.
//!
//! Each node is measured from its label and shape (a diamond needs ~2x the box
//! of a rectangle to hold the same text, since a rhombus only fits text in its
//! centre). Ranks become variable-width columns and lanes become variable-height
//! bands, so nothing overflows and long labels get room. When the chart has
//! swimlane containers, layout is lane-aware: the flow runs along the main axis
//! by rank while lanes form full-length bands on the cross axis.

use std::collections::HashMap;

use super::{ContainerKind, Direction, Flowchart, Shape};

/// Fallback / minimum node box size in pixels.
pub const NODE_W: f64 = 160.0;
pub const NODE_H: f64 = 56.0;
/// Gap between adjacent ranks and between siblings within a rank.
pub const RANK_GAP: f64 = 70.0;
pub const SIBLING_GAP: f64 = 34.0;
pub const MARGIN: f64 = 24.0;
/// Lane title-bar thickness (reserved at the main-axis start of each lane).
pub const LANE_TITLE: f64 = 30.0;
/// Cross-axis padding inside a lane band.
pub const LANE_PAD: f64 = 20.0;

/// Text metrics for ~12px Helvetica (the export font).
const CHAR_W: f64 = 7.1;
const LINE_H: f64 = 16.0;

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

fn round_up(v: f64, step: f64) -> f64 {
    (v / step).ceil() * step
}

/// Greedily word-wrap `label` to a max line width (px); return (widest_line_px,
/// line_count).
fn measure_text(label: &str, max_text_w: f64) -> (f64, usize) {
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() {
        return (3.0 * CHAR_W, 1);
    }
    let space = CHAR_W;
    let mut lines = 1usize;
    let mut cur = 0.0f64;
    let mut widest = 0.0f64;
    for w in &words {
        let wl = w.chars().count() as f64 * CHAR_W;
        if cur > 0.0 && cur + space + wl > max_text_w {
            widest = widest.max(cur);
            lines += 1;
            cur = wl;
        } else {
            cur = if cur > 0.0 { cur + space + wl } else { wl };
        }
    }
    widest = widest.max(cur);
    (widest, lines)
}

/// Content-aware box size for a node, by shape and label. Rounded to a 10px grid
/// and clamped to sensible bounds so the diagram stays tidy.
pub fn node_size(label: &str, shape: Shape) -> (f64, f64) {
    // Narrower wrap target for shapes whose usable text area is small.
    let max_text = match shape {
        Shape::Diamond => 104.0,
        Shape::Stadium | Shape::Circle | Shape::DoubleCircle => 112.0,
        _ => 196.0,
    };
    let (tw, lines) = measure_text(label, max_text);
    let th = lines as f64 * LINE_H;

    let (mut w, mut h) = match shape {
        // Rhombus only fits text in its centre ~50%, so ~2x both axes.
        Shape::Diamond => (2.0 * tw + 36.0, 2.0 * th + 36.0),
        // Pills need horizontal room for the rounded caps.
        Shape::Stadium => (tw + 54.0, th + 26.0),
        Shape::Circle | Shape::DoubleCircle => {
            let d = tw.max(th) + 48.0;
            (d, d)
        }
        // Document wave + cylinder ellipses need extra vertical room.
        Shape::Document => (tw + 34.0, th + 34.0),
        Shape::Cylinder => (tw + 34.0, th + 40.0),
        Shape::Hexagon => (tw + 60.0, th + 26.0),
        Shape::Parallelogram | Shape::ParallelogramAlt => (tw + 56.0, th + 26.0),
        Shape::Trapezoid | Shape::TrapezoidAlt => (tw + 60.0, th + 26.0),
        _ => (tw + 34.0, th + 26.0),
    };

    // Round to a 10px grid and clamp.
    w = round_up(w, 10.0);
    h = round_up(h, 10.0);
    let (min_w, max_w, min_h, max_h) = match shape {
        Shape::Diamond => (140.0, 300.0, 90.0, 180.0),
        Shape::Stadium => (90.0, 220.0, 48.0, 120.0),
        Shape::Circle | Shape::DoubleCircle => (90.0, 200.0, 90.0, 200.0),
        _ => (120.0, 260.0, 52.0, 170.0),
    };
    (w.clamp(min_w, max_w), h.clamp(min_h, max_h))
}

/// Assign ranks by longest path from any source node.
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

/// Content-aware box size for a node, honoring UML class compartments.
fn node_box_size(node: &super::Node) -> (f64, f64) {
    if node.shape == Shape::UmlClass {
        return class_size(node);
    }
    node_size(&node.label, node.shape)
}

/// Size a UML class box from its title and compartment lines.
fn class_size(node: &super::Node) -> (f64, f64) {
    let mut widest = node.label.chars().count() as f64 * CHAR_W + 24.0;
    let mut lines = 1usize; // title
    for comp in &node.compartments {
        lines += comp.len().max(1);
        for l in comp {
            widest = widest.max(l.chars().count() as f64 * CHAR_W + 16.0);
        }
    }
    let w = round_up(widest, 10.0).clamp(140.0, 320.0);
    // title row + compartment rows + separators.
    let h = round_up(28.0 + lines as f64 * LINE_H + node.compartments.len() as f64 * 6.0, 10.0)
        .clamp(60.0, 400.0);
    (w, h)
}

/// Compute pixel geometry for every node.
pub fn compute(fc: &Flowchart) -> Layout {
    let rank = rank_nodes(fc);
    let max_rank = rank.values().copied().max().unwrap_or(0);
    let lanes = lane_ids(fc);
    let sizes: HashMap<String, (f64, f64)> = fc
        .nodes
        .iter()
        .map(|n| (n.id.clone(), node_box_size(n)))
        .collect();

    let base = match fc.layout {
        crate::engine::LayoutKind::Tree | crate::engine::LayoutKind::MindMap if lanes.is_empty() => {
            compute_tree(fc, &sizes)
        }
        _ if lanes.is_empty() => compute_plain(fc, &rank, max_rank, &sizes),
        _ => compute_laned(fc, &rank, max_rank, &lanes, &sizes),
    };
    base.with_overrides(fc)
}

impl Layout {
    /// Apply manual per-node `pos`/`size` overrides on top of the auto-layout,
    /// then grow the canvas so overridden nodes stay in view.
    fn with_overrides(mut self, fc: &Flowchart) -> Self {
        let mut touched = false;
        for n in &fc.nodes {
            if n.pos.is_none() && n.size.is_none() {
                continue;
            }
            touched = true;
            let cur = self.get(&n.id);
            let (w, h) = n.size.map(|s| (s[0], s[1])).unwrap_or((cur.w, cur.h));
            let (x, y) = n.pos.map(|p| (p[0], p[1])).unwrap_or((cur.x, cur.y));
            self.boxes.insert(n.id.clone(), Box { x, y, w, h });
        }
        if touched {
            for b in self.boxes.values() {
                self.width = self.width.max(b.x + b.w + MARGIN);
                self.height = self.height.max(b.y + b.h + MARGIN);
            }
        }
        self
    }
}

fn size_of(sizes: &HashMap<String, (f64, f64)>, id: &str) -> (f64, f64) {
    sizes.get(id).copied().unwrap_or((NODE_W, NODE_H))
}

/// Lane-aware variable-size grid: ranks → columns (max main-size per rank),
/// lanes → bands, sibling rows aligned across ranks (max cross-size per row).
fn compute_laned(
    fc: &Flowchart,
    rank: &HashMap<String, usize>,
    max_rank: usize,
    lanes: &[String],
    sizes: &HashMap<String, (f64, f64)>,
) -> Layout {
    let vertical = fc.direction.is_vertical();
    let main_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical {
            h
        } else {
            w
        }
    };
    let cross_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical {
            w
        } else {
            h
        }
    };

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

    let min_main = if vertical { NODE_H } else { NODE_W };
    let min_cross = if vertical { NODE_W } else { NODE_H };

    // Column extent per rank = widest node (main axis) across all lanes.
    let mut rank_extent = vec![min_main; max_rank + 1];
    for r in 0..=max_rank {
        let mut m = min_main;
        for li in 0..nlanes {
            for id in &groups[li][r] {
                m = m.max(main_size(id));
            }
        }
        rank_extent[r] = m;
    }

    // Rows per lane and the cross-extent of each row (aligned across ranks).
    let lane_rows: Vec<usize> = groups
        .iter()
        .map(|g| g.iter().map(|v| v.len()).max().unwrap_or(0).max(1))
        .collect();
    let mut row_cross: Vec<Vec<f64>> = Vec::with_capacity(nlanes);
    for li in 0..nlanes {
        let mut rows = Vec::with_capacity(lane_rows[li]);
        for i in 0..lane_rows[li] {
            let mut m = min_cross;
            for r in 0..=max_rank {
                if let Some(id) = groups[li][r].get(i) {
                    m = m.max(cross_size(id));
                }
            }
            rows.push(m);
        }
        row_cross.push(rows);
    }

    // Main-axis column positions (after the lane title bar).
    let mut rank_main = vec![0.0; max_rank + 1];
    let mut acc = MARGIN + LANE_TITLE;
    for r in 0..=max_rank {
        rank_main[r] = acc;
        acc += rank_extent[r] + RANK_GAP;
    }
    let main_content_end = rank_main[max_rank] + rank_extent[max_rank];
    let main_full = (main_content_end - MARGIN) + LANE_PAD;
    let main_total = main_content_end + LANE_PAD + MARGIN;

    // Cross-axis lane bands.
    let lane_cross: Vec<f64> = (0..nlanes)
        .map(|li| {
            let rows: f64 = row_cross[li].iter().sum();
            let gaps = lane_rows[li].saturating_sub(1) as f64 * SIBLING_GAP;
            rows + gaps + 2.0 * LANE_PAD
        })
        .collect();
    let mut lane_start = vec![0.0; nlanes];
    let mut acc2 = MARGIN;
    for li in 0..nlanes {
        lane_start[li] = acc2;
        acc2 += lane_cross[li];
    }
    let cross_total = acc2 + MARGIN;

    // Place nodes: centred in their (column, row) cell.
    let mut boxes = HashMap::new();
    for li in 0..nlanes {
        let mut row_start = Vec::with_capacity(lane_rows[li]);
        let mut ra = lane_start[li] + LANE_PAD;
        for i in 0..lane_rows[li] {
            row_start.push(ra);
            ra += row_cross[li][i] + SIBLING_GAP;
        }
        for r in 0..=max_rank {
            for (i, id) in groups[li][r].iter().enumerate() {
                let (w, h) = size_of(sizes, id);
                let main_pos = rank_main[r] + (rank_extent[r] - main_size(id)) / 2.0;
                let cross_pos = row_start[i] + (row_cross[li][i] - cross_size(id)) / 2.0;
                let (x, y) = if vertical {
                    (cross_pos, main_pos)
                } else {
                    (main_pos, cross_pos)
                };
                boxes.insert(id.clone(), Box { x, y, w, h });
            }
        }
    }

    // Fork/join centring: align a decision (fork) on the cross-axis midpoint of
    // its branch targets, and a merge (join: >=2 incoming) on its sources, so
    // branches splay symmetrically and reconverge cleanly instead of weaving
    // past boxes. The node is clamped to stay inside its own lane band.
    center_forks_and_joins(fc, &mut boxes, vertical, &lane_start, &lane_cross, &node_lane);

    if matches!(fc.direction, Direction::BT | Direction::RL) {
        for b in boxes.values_mut() {
            if vertical {
                b.y = main_total - b.y - b.h;
            } else {
                b.x = main_total - b.x - b.w;
            }
        }
    }

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
        (cross_total, main_total)
    } else {
        (main_total, cross_total)
    };

    Layout {
        boxes,
        lanes: lanes_geom,
        width: width.max(NODE_W + MARGIN * 2.0),
        height: height.max(NODE_H + MARGIN * 2.0),
    }
}

/// Tree / mind-map layout. Roots (no incoming edge) fan out to children along
/// the main axis by depth; siblings are packed on the cross axis so subtrees
/// never overlap. MindMap splits a single root's top-level branches to both
/// sides of the root on the cross axis.
fn compute_tree(fc: &Flowchart, sizes: &HashMap<String, (f64, f64)>) -> Layout {
    let vertical = fc.direction.is_vertical();
    let mind_map = fc.layout == crate::engine::LayoutKind::MindMap;
    let main_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical { h } else { w }
    };
    let cross_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical { w } else { h }
    };

    // Children adjacency in declaration order; track incoming to find roots.
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut indeg: HashMap<&str, usize> = fc.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    for e in &fc.edges {
        children.entry(e.from.as_str()).or_default().push(e.to.as_str());
        *indeg.entry(e.to.as_str()).or_default() += 1;
    }
    let roots: Vec<&str> = fc
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .filter(|id| indeg.get(id).copied().unwrap_or(0) == 0)
        .collect();
    let roots: Vec<&str> = if roots.is_empty() {
        fc.nodes.first().map(|n| n.id.as_str()).into_iter().collect()
    } else {
        roots
    };

    // Depth (main-axis rank) and cross-axis position via leaf packing.
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut cross: HashMap<String, f64> = HashMap::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut cursor = MARGIN; // running cross-axis offset (centre of each leaf band)

    // Recursive packer: returns the cross-centre of the placed subtree.
    fn place(
        id: &str,
        d: usize,
        children: &HashMap<&str, Vec<&str>>,
        cross_size: &dyn Fn(&str) -> f64,
        depth: &mut HashMap<String, usize>,
        cross: &mut HashMap<String, f64>,
        visited: &mut std::collections::HashSet<String>,
        cursor: &mut f64,
    ) -> f64 {
        if !visited.insert(id.to_string()) {
            return *cross.get(id).unwrap_or(&MARGIN);
        }
        depth.insert(id.to_string(), d);
        let kids: Vec<&str> = children
            .get(id)
            .map(|v| v.iter().copied().filter(|k| !visited.contains(*k)).collect())
            .unwrap_or_default();
        let c = if kids.is_empty() {
            let half = cross_size(id) / 2.0;
            let pos = *cursor + half;
            *cursor += cross_size(id) + SIBLING_GAP;
            pos
        } else {
            let mut centres = Vec::new();
            for k in &kids {
                centres.push(place(k, d + 1, children, cross_size, depth, cross, visited, cursor));
            }
            centres.iter().sum::<f64>() / centres.len() as f64
        };
        cross.insert(id.to_string(), c);
        c
    }

    for r in &roots {
        place(r, 0, &children, &cross_size, &mut depth, &mut cross, &mut visited, &mut cursor);
    }
    // Any unvisited (cyclic) nodes get appended as their own leaves.
    for n in &fc.nodes {
        if !visited.contains(&n.id) {
            place(&n.id, 0, &children, &cross_size, &mut depth, &mut cross, &mut visited, &mut cursor);
        }
    }

    // Main-axis offset per depth = max main-size of that depth + RANK_GAP.
    let max_depth = depth.values().copied().max().unwrap_or(0);
    let mut depth_main = vec![0.0f64; max_depth + 1];
    let mut depth_extent = vec![0.0f64; max_depth + 1];
    for (id, &d) in &depth {
        depth_extent[d] = depth_extent[d].max(main_size(id));
    }
    let mut acc = MARGIN;
    for d in 0..=max_depth {
        depth_main[d] = acc;
        acc += depth_extent[d] + RANK_GAP * 1.4;
    }

    // Mind-map: mirror half the first root's branches to the negative side.
    let mut neg: std::collections::HashSet<String> = std::collections::HashSet::new();
    if mind_map {
        if let Some(root) = roots.first() {
            if let Some(kids) = children.get(*root) {
                // Collect each branch's whole subtree, alternate sides.
                for (i, k) in kids.iter().enumerate() {
                    if i % 2 == 1 {
                        collect_subtree(k, &children, &mut neg);
                    }
                }
            }
        }
    }

    let mut boxes = HashMap::new();
    let mut max_cross = 0.0f64;
    let mut max_main = 0.0f64;
    for n in &fc.nodes {
        let d = *depth.get(&n.id).unwrap_or(&0);
        let c = *cross.get(&n.id).unwrap_or(&MARGIN);
        let (w, h) = size_of(sizes, &n.id);
        let main_pos = if mind_map && neg.contains(&n.id) {
            // place on the opposite side of the root's depth-0 main line
            depth_main[0] - (depth_main[d] - depth_main[0]) - main_size(&n.id) + main_size_root(&roots, sizes, vertical)
        } else {
            depth_main[d]
        };
        let cross_pos = c - cross_size(&n.id) / 2.0;
        let (x, y) = if vertical { (cross_pos, main_pos) } else { (main_pos, cross_pos) };
        boxes.insert(n.id.clone(), Box { x, y, w, h });
        max_cross = max_cross.max(cross_pos + cross_size(&n.id));
        max_main = max_main.max(main_pos + main_size(&n.id));
    }

    // Normalise negative main positions (mind-map left side) into view.
    let min_main = boxes
        .values()
        .map(|b| if vertical { b.y } else { b.x })
        .fold(f64::INFINITY, f64::min);
    if min_main < MARGIN {
        let shift = MARGIN - min_main;
        for b in boxes.values_mut() {
            if vertical { b.y += shift } else { b.x += shift }
        }
        max_main += shift;
    }

    let (width, height) = if vertical {
        (max_cross + MARGIN, max_main + MARGIN)
    } else {
        (max_main + MARGIN, max_cross + MARGIN)
    };

    Layout {
        boxes,
        lanes: Vec::new(),
        width: width.max(NODE_W + MARGIN * 2.0),
        height: height.max(NODE_H + MARGIN * 2.0),
    }
}

/// Main-axis size of the first root (used to mirror mind-map branches).
fn main_size_root(roots: &[&str], sizes: &HashMap<String, (f64, f64)>, vertical: bool) -> f64 {
    roots
        .first()
        .map(|r| {
            let (w, h) = size_of(sizes, r);
            if vertical { h } else { w }
        })
        .unwrap_or(0.0)
}

/// Collect a node and all its descendants into `set`.
fn collect_subtree(id: &str, children: &HashMap<&str, Vec<&str>>, set: &mut std::collections::HashSet<String>) {
    if !set.insert(id.to_string()) {
        return;
    }
    if let Some(kids) = children.get(id) {
        for k in kids {
            collect_subtree(k, children, set);
        }
    }
}

/// Center fork nodes (>=2 outgoing) on their targets and join nodes (>=2
/// incoming) on their sources, along the cross axis, so branches splay and
/// reconverge symmetrically. Each moved node is clamped to remain fully within
/// its own lane band. Operates in pre-flip coordinates.
#[allow(clippy::too_many_arguments)]
fn center_forks_and_joins(
    fc: &Flowchart,
    boxes: &mut HashMap<String, Box>,
    vertical: bool,
    lane_start: &[f64],
    lane_cross: &[f64],
    node_lane: &HashMap<&str, usize>,
) {
    // Cross-axis centre & size accessors for the current orientation.
    let cross_c = |b: &Box| if vertical { b.x + b.w / 2.0 } else { b.y + b.h / 2.0 };
    let cross_s = |b: &Box| if vertical { b.w } else { b.h };

    // Collect neighbour centres for each fork/join, then compute the target
    // centre. Done in two phases so reads don't fight the borrow checker.
    let mut moves: Vec<(String, f64)> = Vec::new();
    let neighbours = |id: &str, outgoing: bool| -> Vec<String> {
        fc.edges
            .iter()
            .filter_map(|e| {
                if outgoing && e.from == id {
                    Some(e.to.clone())
                } else if !outgoing && e.to == id {
                    Some(e.from.clone())
                } else {
                    None
                }
            })
            .collect()
    };

    for n in &fc.nodes {
        let outs = fc.edges.iter().filter(|e| e.from == n.id).count();
        let ins = fc.edges.iter().filter(|e| e.to == n.id).count();
        // Prefer centring a fork on its branches; else a join on its sources.
        let neigh = if outs >= 2 {
            neighbours(&n.id, true)
        } else if ins >= 2 {
            neighbours(&n.id, false)
        } else {
            continue;
        };
        let centres: Vec<f64> = neigh
            .iter()
            .filter_map(|m| boxes.get(m).map(|b| cross_c(b)))
            .collect();
        if centres.is_empty() {
            continue;
        }
        let avg = centres.iter().sum::<f64>() / centres.len() as f64;
        moves.push((n.id.clone(), avg));
    }

    for (id, target_centre) in moves {
        let Some(b) = boxes.get(&id).copied() else { continue };
        let li = *node_lane.get(id.as_str()).unwrap_or(&0);
        let band_lo = lane_start[li] + LANE_PAD;
        let band_hi = lane_start[li] + lane_cross[li] - LANE_PAD;
        let half = cross_s(&b) / 2.0;
        // Clamp the new centre so the box stays inside its lane band.
        let lo = band_lo + half;
        let hi = (band_hi - half).max(lo);
        let c = target_centre.clamp(lo, hi);
        if let Some(bm) = boxes.get_mut(&id) {
            if vertical {
                bm.x = c - bm.w / 2.0;
            } else {
                bm.y = c - bm.h / 2.0;
            }
        }
    }
}

/// Plain layered layout (no swimlanes), variable sizes, ranks centred on the
/// cross axis.
fn compute_plain(
    fc: &Flowchart,
    rank: &HashMap<String, usize>,
    max_rank: usize,
    sizes: &HashMap<String, (f64, f64)>,
) -> Layout {
    let vertical = fc.direction.is_vertical();
    let main_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical {
            h
        } else {
            w
        }
    };
    let cross_size = |id: &str| {
        let (w, h) = size_of(sizes, id);
        if vertical {
            w
        } else {
            h
        }
    };

    let mut ranks: Vec<Vec<String>> = vec![Vec::new(); max_rank + 1];
    for node in &fc.nodes {
        let r = *rank.get(&node.id).unwrap_or(&0);
        ranks[r].push(node.id.clone());
    }

    let min_main = if vertical { NODE_H } else { NODE_W };

    // Column extent per rank and total cross span (widest rank's stacked nodes).
    let mut rank_extent = vec![min_main; max_rank + 1];
    let mut rank_cross_extent = vec![0.0f64; max_rank + 1];
    for r in 0..=max_rank {
        let mut m = min_main;
        let mut cross = 0.0;
        for (i, id) in ranks[r].iter().enumerate() {
            m = m.max(main_size(id));
            if i > 0 {
                cross += SIBLING_GAP;
            }
            cross += cross_size(id);
        }
        rank_extent[r] = m;
        rank_cross_extent[r] = cross;
    }
    let cross_span = rank_cross_extent.iter().cloned().fold(0.0, f64::max);

    let mut rank_main = vec![0.0; max_rank + 1];
    let mut acc = MARGIN;
    for r in 0..=max_rank {
        rank_main[r] = acc;
        acc += rank_extent[r] + RANK_GAP;
    }
    let main_total = if max_rank == 0 && ranks[0].is_empty() {
        MARGIN * 2.0 + min_main
    } else {
        rank_main[max_rank] + rank_extent[max_rank] + MARGIN
    };

    let mut boxes = HashMap::new();
    for r in 0..=max_rank {
        let mut cross_pos = MARGIN + (cross_span - rank_cross_extent[r]).max(0.0) / 2.0;
        for id in &ranks[r] {
            let (w, h) = size_of(sizes, id);
            let main_pos = rank_main[r] + (rank_extent[r] - main_size(id)) / 2.0;
            let (x, y) = if vertical {
                (cross_pos, main_pos)
            } else {
                (main_pos, cross_pos)
            };
            boxes.insert(id.clone(), Box { x, y, w, h });
            cross_pos += cross_size(id) + SIBLING_GAP;
        }
    }

    if matches!(fc.direction, Direction::BT | Direction::RL) {
        for b in boxes.values_mut() {
            if vertical {
                b.y = main_total - b.y - b.h;
            } else {
                b.x = main_total - b.x - b.w;
            }
        }
    }

    let (width, height) = if vertical {
        (cross_span + MARGIN * 2.0, main_total)
    } else {
        (main_total, cross_span + MARGIN * 2.0)
    };

    Layout {
        boxes,
        lanes: Vec::new(),
        width: width.max(NODE_W + MARGIN * 2.0),
        height: height.max(NODE_H + MARGIN * 2.0),
    }
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
        assert!(l.lanes[0].b.y < l.lanes[1].b.y);
        assert!((l.lanes[0].b.w - l.lanes[1].b.w).abs() < 1.0);
        assert!(l.get("a").y < l.get("b").y);
    }

    #[test]
    fn diamond_is_larger_than_rectangle_for_same_text() {
        let label = "Cargo nominated to CFS via?";
        let (dw, dh) = node_size(label, Shape::Diamond);
        let (rw, rh) = node_size(label, Shape::Rectangle);
        // A diamond must be meaningfully bigger to hold the same text.
        assert!(dw * dh > rw * rh, "diamond {dw}x{dh} should exceed rect {rw}x{rh}");
        assert!(dh >= 90.0);
    }

    #[test]
    fn long_label_widens_box() {
        let (w_short, _) = node_size("OK", Shape::Rectangle);
        let (w_long, _) = node_size(
            "Confirm arrival from KPA site; assign task to port clerks",
            Shape::Rectangle,
        );
        assert!(w_long > w_short);
    }

    #[test]
    fn tree_layout_places_root_before_children() {
        let mut fc = Flowchart::new(Direction::TB);
        fc.set_layout(crate::engine::LayoutKind::Tree);
        fc.add_node("root", "Root", Shape::Rectangle).unwrap();
        fc.add_node("a", "A", Shape::Rectangle).unwrap();
        fc.add_node("b", "B", Shape::Rectangle).unwrap();
        fc.add_edge("root", "a", None, crate::engine::LineStyle::Solid, true).unwrap();
        fc.add_edge("root", "b", None, crate::engine::LineStyle::Solid, true).unwrap();
        let l = compute(&fc);
        assert!(l.get("root").y < l.get("a").y);
        assert!(l.get("root").y < l.get("b").y);
        assert!((l.get("a").y - l.get("b").y).abs() < 1.0);
        assert!(l.get("a").x != l.get("b").x);
    }

    #[test]
    fn manual_override_wins_and_grows_canvas() {
        let mut fc = chain();
        fc.move_node("c", Some([900.0, 700.0]), Some([180.0, 90.0]), false)
            .unwrap();
        let l = compute(&fc);
        let b = l.get("c");
        assert_eq!((b.x, b.y, b.w, b.h), (900.0, 700.0, 180.0, 90.0));
        // Canvas grew to include the manually placed node.
        assert!(l.width >= 900.0 + 180.0);
        assert!(l.height >= 700.0 + 90.0);
        // Non-overridden nodes keep auto-layout.
        assert!(l.get("a").x < 900.0);
    }
}
