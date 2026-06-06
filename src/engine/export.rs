//! Exporters: draw.io (mxGraph XML), Mermaid, Graphviz DOT, and SVG.
//!
//! `to_drawio` takes the whole [`Document`] and emits one `<diagram>` page each,
//! with nested containers/swimlanes, node images, rich styles, and arrowheads.
//! The text exporters (mermaid/dot/svg) operate on a single [`Flowchart`].

use std::collections::HashMap;

use super::layout::{self, Box};
use super::{
    ContainerKind, Document, Edge, Flowchart, LineStyle, Shape, Style, Subgraph,
};

/// Escape text for XML attribute/text content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Sanitize a node id into a token safe for DOT / cell ids.
fn ident(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        format!("n_{t}")
    } else {
        t
    }
}

/// Strip HTML tags to plain text, turning `<br>`/`</p>`/`</div>` into spaces.
/// Used by the text/SVG exporters when a node's label is rich HTML.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let lower = s.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0;
    for c in s.chars() {
        if c == '<' {
            // line-breaking tags become a space
            if lower[i..].starts_with("<br") || lower[i..].starts_with("</p") || lower[i..].starts_with("</div") {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
        i += c.len_utf8();
    }
    let _ = bytes;
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A node's label as plain text — tags stripped only when the label is HTML.
pub(crate) fn plain_label(node: &super::Node) -> String {
    if node.html == Some(true) {
        strip_html(&node.label)
    } else {
        node.label.clone()
    }
}

// ---------------------------------------------------------------------------
// Mermaid
// ---------------------------------------------------------------------------

pub fn to_mermaid(fc: &Flowchart) -> String {
    let mut out = String::new();
    out.push_str(&format!("flowchart {}\n", fc.direction.as_mermaid()));

    let member_of: HashMap<&str, &str> = fc
        .subgraphs
        .iter()
        .flat_map(|sg| sg.members.iter().map(move |m| (m.as_str(), sg.id.as_str())))
        .collect();

    for node in &fc.nodes {
        if !member_of.contains_key(node.id.as_str()) {
            out.push_str(&format!(
                "    {}{}\n",
                ident(&node.id),
                node.shape.mermaid_wrap(&plain_label(node))
            ));
        }
    }
    for sg in &fc.subgraphs {
        out.push_str(&format!("    subgraph {} [{}]\n", ident(&sg.id), sg.label));
        for m in &sg.members {
            if let Some(node) = fc.nodes.iter().find(|n| &n.id == m) {
                out.push_str(&format!(
                    "        {}{}\n",
                    ident(&node.id),
                    node.shape.mermaid_wrap(&plain_label(node))
                ));
            }
        }
        out.push_str("    end\n");
    }
    for e in &fc.edges {
        out.push_str(&format!("    {}{}\n", ident(&e.from), mermaid_edge(e)));
    }
    for node in &fc.nodes {
        if let Some(style) = mermaid_style(&node.style) {
            out.push_str(&format!("    style {} {}\n", ident(&node.id), style));
        }
    }
    out
}

fn mermaid_edge(e: &Edge) -> String {
    let arrow = if e.arrow { ">" } else { "-" };
    let connector = match e.line {
        LineStyle::Solid => format!("--{arrow}"),
        LineStyle::Dotted => format!("-.-{arrow}"),
        LineStyle::Thick => format!("==={arrow}"),
    };
    match &e.label {
        Some(l) if !l.is_empty() => match e.line {
            LineStyle::Dotted => format!(" -. {l} .-{arrow} {}", ident(&e.to)),
            LineStyle::Thick => format!(" =={l}=={arrow} {}", ident(&e.to)),
            LineStyle::Solid => format!(" --{l}--{arrow} {}", ident(&e.to)),
        },
        _ => format!(" {connector} {}", ident(&e.to)),
    }
}

fn mermaid_style(s: &Style) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(f) = &s.fill {
        parts.push(format!("fill:{f}"));
    }
    if let Some(st) = &s.stroke {
        parts.push(format!("stroke:{st}"));
    }
    if let Some(c) = &s.text_color {
        parts.push(format!("color:{c}"));
    }
    if let Some(w) = s.stroke_width {
        parts.push(format!("stroke-width:{w}px"));
    }
    if s.dashed == Some(true) {
        parts.push("stroke-dasharray:5 5".to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

// ---------------------------------------------------------------------------
// Graphviz DOT
// ---------------------------------------------------------------------------

pub fn to_dot(fc: &Flowchart) -> String {
    let mut out = String::new();
    out.push_str("digraph G {\n");
    out.push_str(&format!("  rankdir={};\n", fc.direction.as_dot()));
    out.push_str("  node [fontname=\"Helvetica\"];\n");
    out.push_str("  edge [fontname=\"Helvetica\"];\n");
    if let Some(t) = &fc.title {
        out.push_str(&format!("  label=\"{}\";\n  labelloc=t;\n", dot_escape(t)));
    }

    for (i, sg) in fc.subgraphs.iter().enumerate() {
        out.push_str(&format!("  subgraph cluster_{i} {{\n"));
        out.push_str(&format!("    label=\"{}\";\n", dot_escape(&sg.label)));
        for m in &sg.members {
            out.push_str(&format!("    {};\n", ident(m)));
        }
        out.push_str("  }\n");
    }

    for node in &fc.nodes {
        out.push_str(&format!(
            "  {} [label=\"{}\", shape={}{}];\n",
            ident(&node.id),
            dot_escape(&plain_label(node)),
            dot_shape(node.shape),
            dot_style(&node.style),
        ));
    }
    for e in &fc.edges {
        let mut attrs = Vec::new();
        if let Some(l) = &e.label {
            if !l.is_empty() {
                attrs.push(format!("label=\"{}\"", dot_escape(l)));
            }
        }
        match e.line {
            LineStyle::Dotted => attrs.push("style=dashed".into()),
            LineStyle::Thick => attrs.push("penwidth=2.5".into()),
            LineStyle::Solid => {}
        }
        if e.resolved_end() == super::Arrow::None {
            attrs.push("dir=none".into());
        }
        if let Some(c) = &e.color {
            attrs.push(format!("color=\"{c}\""));
        }
        let suffix = if attrs.is_empty() {
            String::new()
        } else {
            format!(" [{}]", attrs.join(", "))
        };
        out.push_str(&format!(
            "  {} -> {}{};\n",
            ident(&e.from),
            ident(&e.to),
            suffix
        ));
    }
    out.push_str("}\n");
    out
}

fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn dot_shape(shape: Shape) -> &'static str {
    match shape {
        Shape::Rectangle | Shape::Card => "box",
        Shape::RoundRect | Shape::Stadium => "box style=rounded",
        Shape::Subroutine => "box",
        Shape::Cylinder => "cylinder",
        Shape::Circle => "circle",
        Shape::DoubleCircle => "doublecircle",
        Shape::Diamond => "diamond",
        Shape::Hexagon => "hexagon",
        Shape::Parallelogram | Shape::ParallelogramAlt => "parallelogram",
        Shape::Trapezoid | Shape::TrapezoidAlt => "trapezium",
        Shape::Note | Shape::Document => "note",
        Shape::UmlClass => "record",
    }
}

fn dot_style(s: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(f) = &s.fill {
        parts.push(format!("style=filled, fillcolor=\"{f}\""));
    }
    if let Some(st) = &s.stroke {
        parts.push(format!("color=\"{st}\""));
    }
    if let Some(c) = &s.text_color {
        parts.push(format!("fontcolor=\"{c}\""));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(", {}", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Container geometry (shared by draw.io + SVG)
// ---------------------------------------------------------------------------

/// Top inset reserved for a titled container's label bar.
fn title_inset(kind: ContainerKind) -> f64 {
    match kind {
        ContainerKind::Group => 8.0,
        _ => 28.0,
    }
}

/// Absolute padded box for a container, unioning member nodes and any child
/// containers (recursively; the hierarchy is acyclic by construction).
fn container_abs_box(fc: &Flowchart, l: &layout::Layout, id: &str) -> Option<Box> {
    let sg = fc.subgraphs.iter().find(|s| s.id == id)?;
    let mut bb = bounds(l, &sg.members);
    for child in fc.subgraphs.iter().filter(|s| s.parent.as_deref() == Some(id)) {
        if let Some(cb) = container_abs_box(fc, l, &child.id) {
            bb = Some(match bb {
                Some(b) => union(b, cb),
                None => cb,
            });
        }
    }
    let pad = 16.0;
    let top = title_inset(sg.kind);
    bb.map(|b| Box {
        x: b.x - pad,
        y: b.y - top,
        w: b.w + pad * 2.0,
        h: b.h + top + pad,
    })
}

fn union(a: Box, b: Box) -> Box {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Box { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// Containers ordered parents-before-children (stable for shallow nesting).
fn ordered_containers(fc: &Flowchart) -> Vec<usize> {
    let mut order: Vec<usize> = Vec::new();
    let depth = |sg: &Subgraph| -> usize {
        let mut d = 0;
        let mut p = sg.parent.clone();
        while let Some(pid) = p {
            d += 1;
            p = fc.subgraphs.iter().find(|s| s.id == pid).and_then(|s| s.parent.clone());
            if d > 16 {
                break;
            }
        }
        d
    };
    let mut idx: Vec<usize> = (0..fc.subgraphs.len()).collect();
    idx.sort_by_key(|&i| depth(&fc.subgraphs[i]));
    order.extend(idx);
    order
}

// ---------------------------------------------------------------------------
// draw.io (mxGraph XML)
// ---------------------------------------------------------------------------

pub fn to_drawio(doc: &Document) -> String {
    let mut diagrams = String::new();
    for page in &doc.pages {
        diagrams.push_str(&drawio_page(&page.name, &page.chart));
    }
    format!("<mxfile host=\"app.diagrams.net\" type=\"device\">\n{diagrams}</mxfile>\n")
}

/// Render a single page wrapped in a complete `<mxfile>` (for per-page export).
pub fn to_drawio_page(name: &str, fc: &Flowchart) -> String {
    format!(
        "<mxfile host=\"app.diagrams.net\" type=\"device\">\n{}</mxfile>\n",
        drawio_page(name, fc)
    )
}

/// File extension for an export format token.
pub fn format_ext(format: &str) -> &'static str {
    match format.to_ascii_lowercase().as_str() {
        "drawio" | "xml" => "drawio",
        "mermaid" | "mmd" => "mmd",
        "dot" | "graphviz" => "dot",
        "svg" => "svg",
        "json" => "json",
        _ => "txt",
    }
}

fn drawio_page(name: &str, fc: &Flowchart) -> String {
    let l = layout::compute(fc);
    let mut cells = String::new();

    // Reserve a header band when the chart has a title; every absolute y is
    // shifted down by this amount so the title sits cleanly on top.
    let header = if fc.title.as_deref().map(|t| !t.is_empty()).unwrap_or(false) {
        46.0
    } else {
        0.0
    };

    // Title header.
    if header > 0.0 {
        let title = fc.title.as_deref().unwrap_or("");
        cells.push_str(&format!(
            "        <mxCell id=\"title\" value=\"{}\" style=\"text;html=1;strokeColor=none;fillColor=none;\
             align=left;verticalAlign=middle;whiteSpace=wrap;fontSize=18;fontStyle=1;fontColor=#1F2A37;\" \
             vertex=\"1\" parent=\"1\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"8\" width=\"{:.0}\" height=\"32\" as=\"geometry\"/>\n        </mxCell>\n",
            xml_escape(title),
            layout::MARGIN,
            (l.width - layout::MARGIN * 2.0).max(200.0),
        ));
    }

    // Swimlane bands (full-length, non-overlapping) from the lane-aware layout.
    // Emitted as background cells; nodes are positioned absolutely on top.
    let lane_ids: std::collections::HashSet<&str> =
        l.lanes.iter().map(|lg| lg.id.as_str()).collect();
    let vertical = fc.direction.is_vertical();
    for lg in &l.lanes {
        let label = fc
            .subgraphs
            .iter()
            .find(|s| s.id == lg.id)
            .map(|s| s.label.as_str())
            .unwrap_or("");
        cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"1\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
            ident(&lg.id),
            xml_escape(label),
            drawio_lane_style(vertical),
            lg.b.x,
            lg.b.y + header,
            lg.b.w,
            lg.b.h,
        ));
    }

    // Non-swimlane containers (group/container/pool) keep member-bounds boxes.
    for i in ordered_containers(fc) {
        let sg = &fc.subgraphs[i];
        if lane_ids.contains(sg.id.as_str()) {
            continue;
        }
        let Some(b) = container_abs_box(fc, &l, &sg.id) else {
            continue;
        };
        cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"1\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
            ident(&sg.id),
            xml_escape(&sg.label),
            drawio_container_style(sg),
            b.x,
            b.y + header,
            b.w,
            b.h,
        ));
    }

    // Step numbering (optional): a sequential number on each "step" node
    // (process/document/decision — not start/end terminators) in flow order.
    let step_numbers: HashMap<&str, usize> = if fc.number_steps {
        number_steps(fc)
    } else {
        HashMap::new()
    };

    // Nodes — absolute coordinates; parented to their layer cell when set.
    for node in &fc.nodes {
        let b = l.get(&node.id);
        let parent = match &node.layer {
            Some(lid) if fc.layers.iter().any(|l| &l.id == lid) => ident(lid),
            _ => "1".to_string(),
        };
        cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"{}\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
            ident(&node.id),
            drawio_node_value(node),
            drawio_node_style(node),
            parent,
            b.x,
            b.y + header,
            b.w,
            b.h,
        ));

        // Step-number badge: a small numbered circle pinned to the node's top.
        if let Some(&num) = step_numbers.get(node.id.as_str()) {
            let bw = 22.0;
            let bx = b.x + b.w - bw / 2.0 - 6.0;
            let by = b.y + header - bw / 2.0 + 6.0;
            cells.push_str(&format!(
                "        <mxCell id=\"{}_badge\" value=\"{}\" style=\"ellipse;whiteSpace=wrap;html=1;\
                 fillColor=#1F2A37;strokeColor=#FFFFFF;fontColor=#FFFFFF;fontStyle=1;fontSize=11;\
                 verticalAlign=middle;align=center;\" vertex=\"1\" parent=\"1\">\n          \
                 <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
                ident(&node.id),
                num,
                bx,
                by,
                bw,
                bw,
            ));
        }
    }
    // Group edges by source (fan-out) and target (fan-in).
    let mut out_edges: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut in_edges: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in fc.edges.iter().enumerate() {
        out_edges.entry(e.from.as_str()).or_default().push(i);
        in_edges.entry(e.to.as_str()).or_default().push(i);
    }
    let shape_of: HashMap<&str, Shape> =
        fc.nodes.iter().map(|n| (n.id.as_str(), n.shape)).collect();
    let is_diamond = |id: &str| matches!(shape_of.get(id), Some(Shape::Diamond));

    let vertical = fc.direction.is_vertical();
    let cross_of = |bx: Box| if vertical { bx.x + bx.w / 2.0 } else { bx.y + bx.h / 2.0 };
    let main_of = |bx: Box| if vertical { bx.y + bx.h / 2.0 } else { bx.x + bx.w / 2.0 };

    // Diamonds connect only at their four tips (top/right/bottom/left); a point
    // anywhere else on the bounding box floats off the slanted sides. So for a
    // decision we route incoming to the rear tip and each branch to a distinct
    // tip chosen by the target's direction. Rectangles keep fractional spreading
    // along the forward face (valid on a straight edge).
    const TOP: (f64, f64) = (0.5, 0.0);
    const BOTTOM: (f64, f64) = (0.5, 1.0);
    const LEFT: (f64, f64) = (0.0, 0.5);
    const RIGHT: (f64, f64) = (1.0, 0.5);

    // Tip for a branch leaving a decision toward `b` (source box `a`).
    // Rear tip a decision receives its single incoming edge on.
    let diamond_entry_tip = |a: Box, b: Box| -> (f64, f64) {
        // dmain > 0 means the source sits AHEAD of the decision along the flow
        // (so the edge comes from the front); <= 0 means it comes from behind.
        let dmain = main_of(a) - main_of(b);
        let dcross = cross_of(a) - cross_of(b);
        let rear = if vertical { TOP } else { LEFT };
        let front = if vertical { BOTTOM } else { RIGHT };
        let up = if vertical { LEFT } else { TOP };
        let down = if vertical { RIGHT } else { BOTTOM };
        if dmain.abs() >= dcross.abs() {
            if dmain <= 0.0 { rear } else { front }
        } else if dcross < 0.0 {
            up
        } else {
            down
        }
    };

    // For rectangle-like nodes, spread k edges across the forward/incoming face.
    let frac = |rank: usize, count: usize| (rank as f64 + 1.0) / (count as f64 + 1.0);
    let order_by_target_cross = |idxs: &[usize], pick_other: &dyn Fn(usize) -> Box| -> Vec<usize> {
        let mut ord = idxs.to_vec();
        ord.sort_by(|&i, &j| {
            cross_of(pick_other(i))
                .partial_cmp(&cross_of(pick_other(j)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ord
    };

    // Precompute exit anchors.
    let mut exit_anchor: HashMap<usize, (f64, f64)> = HashMap::new();
    for (src, idxs) in out_edges.iter() {
        if idxs.len() < 2 {
            continue;
        }
        let a = l.get(src);
        if is_diamond(src) {
            // Decision fan-out: order branches by target cross-position and map
            // them onto distinct tips. Rear tip is reserved for the incoming
            // edge; branches use up / forward / down.
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].to));
            let (up, fwd, down) = if vertical {
                (LEFT, BOTTOM, RIGHT)
            } else {
                (TOP, RIGHT, BOTTOM)
            };
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let b = l.get(&fc.edges[ei].to);
                let dc = cross_of(b) - cross_of(a);
                let tip = if n == 2 {
                    // Two branches ALWAYS use two distinct tips (even when both
                    // point to the same target). Rank decides which: the first
                    // branch goes forward, the second drops to the "down" tip —
                    // unless their targets clearly separate on the cross axis,
                    // in which case follow geometry (up vs down).
                    let other = l.get(&fc.edges[ord[1 - rank]].to);
                    let dc_other = cross_of(other) - cross_of(a);
                    if (dc - dc_other).abs() >= 24.0 {
                        if dc <= dc_other { up } else { down }
                    } else if rank == 0 {
                        fwd
                    } else {
                        down
                    }
                } else if rank == 0 {
                    up
                } else if rank + 1 == n {
                    down
                } else {
                    fwd
                };
                exit_anchor.insert(ei, tip);
            }
        } else {
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].to));
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let f = frac(rank, n);
                exit_anchor.insert(ei, if vertical { (f, 1.0) } else { (1.0, f) });
            }
        }
    }
    // Single outgoing edge from a decision snaps to its forward tip.
    for (src, idxs) in out_edges.iter() {
        if idxs.len() == 1 && is_diamond(src) {
            let ei = idxs[0];
            let a = l.get(src);
            let b = l.get(&fc.edges[ei].to);
            let dc = cross_of(b) - cross_of(a);
            let (up, fwd, down) = if vertical { (LEFT, BOTTOM, RIGHT) } else { (TOP, RIGHT, BOTTOM) };
            let tip = if dc.abs() < 24.0 { fwd } else if dc < 0.0 { up } else { down };
            exit_anchor.insert(ei, tip);
        }
    }

    // Precompute entry anchors.
    let mut entry_anchor: HashMap<usize, (f64, f64)> = HashMap::new();
    for (tgt, idxs) in in_edges.iter() {
        if idxs.len() < 2 {
            continue;
        }
        if is_diamond(tgt) {
            let b = l.get(tgt);
            let mut used: Vec<(f64, f64)> = Vec::new();
            for &ei in idxs.iter() {
                let mut tip = diamond_entry_tip(l.get(&fc.edges[ei].from), b);
                let mut guard = 0;
                while used.iter().any(|u| (u.0 - tip.0).abs() < 0.01 && (u.1 - tip.1).abs() < 0.01) && guard < 4 {
                    tip = match tip {
                        TOP => RIGHT,
                        RIGHT => BOTTOM,
                        BOTTOM => LEFT,
                        _ => TOP,
                    };
                    guard += 1;
                }
                used.push(tip);
                entry_anchor.insert(ei, tip);
            }
        } else {
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].from));
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let f = frac(rank, n);
                entry_anchor.insert(ei, if vertical { (f, 0.0) } else { (0.0, f) });
            }
        }
    }
    // Single incoming edge into a decision snaps to its rear tip.
    for (tgt, idxs) in in_edges.iter() {
        if idxs.len() == 1 && is_diamond(tgt) {
            let ei = idxs[0];
            entry_anchor.insert(ei, diamond_entry_tip(l.get(&fc.edges[ei].from), l.get(tgt)));
        }
    }

    for (i, e) in fc.edges.iter().enumerate() {
        let mut style = drawio_edge_style(e);
        let fan_out = out_edges.get(e.from.as_str()).map(|v| v.len()).unwrap_or(0) >= 2;
        let fan_in = in_edges.get(e.to.as_str()).map(|v| v.len()).unwrap_or(0) >= 2;

        if let Some(&(ex, ey)) = exit_anchor.get(&i) {
            style.push_str(&format!("exitX={ex};exitY={ey};exitDx=0;exitDy=0;"));
        }
        if let Some(&(nx, ny)) = entry_anchor.get(&i) {
            style.push_str(&format!("entryX={nx};entryY={ny};entryDx=0;entryDy=0;"));
        }
        // Explicit fixed ports override the computed anchors.
        if let Some([ex, ey]) = e.exit {
            style.push_str(&format!("exitX={ex};exitY={ey};exitDx=0;exitDy=0;"));
        }
        if let Some([nx, ny]) = e.entry {
            style.push_str(&format!("entryX={nx};entryY={ny};entryDx=0;entryDy=0;"));
        }

        // Manual waypoints, if any, become an explicit points array.
        let waypoints = if e.waypoints.is_empty() {
            String::new()
        } else {
            let pts: String = e
                .waypoints
                .iter()
                .map(|p| format!("<mxPoint x=\"{:.0}\" y=\"{:.0}\"/>", p[0], p[1] + header))
                .collect();
            format!("<Array as=\"points\">{pts}</Array>")
        };

        // Explicit label styling (background / border) overrides the default.
        if let Some(bg) = &e.label_bg {
            style.push_str(&format!("labelBackgroundColor={bg};"));
        }
        if let Some(bd) = &e.label_border {
            style.push_str(&format!("labelBorderColor={bd};"));
        }

        // Label placement: explicit label_pos/offset win; else bias toward the
        // separated end so it sits on this branch's own segment.
        let has_label = e.label.as_deref().map(|l| !l.is_empty()).unwrap_or(false);
        let geom = if e.label_pos.is_some() || e.label_offset.is_some() {
            let x = e.label_pos.unwrap_or(0.0);
            let off = e.label_offset.unwrap_or(0.0);
            format!(
                "<mxGeometry x=\"{x}\" relative=\"1\" as=\"geometry\"><mxPoint y=\"{off}\" as=\"offset\"/></mxGeometry>"
            )
        } else if !waypoints.is_empty() {
            format!("<mxGeometry relative=\"1\" as=\"geometry\">{waypoints}</mxGeometry>")
        } else if has_label && fan_out {
            "<mxGeometry x=\"-0.5\" relative=\"1\" as=\"geometry\"><mxPoint as=\"offset\"/></mxGeometry>".to_string()
        } else if has_label && fan_in {
            "<mxGeometry x=\"0.5\" relative=\"1\" as=\"geometry\"><mxPoint as=\"offset\"/></mxGeometry>".to_string()
        } else {
            "<mxGeometry relative=\"1\" as=\"geometry\"/>".to_string()
        };

        cells.push_str(&format!(
            "        <mxCell id=\"edge{i}\" value=\"{}\" style=\"{}\" edge=\"1\" parent=\"1\" \
             source=\"{}\" target=\"{}\">\n          {geom}\n        </mxCell>\n",
            xml_escape(e.label.as_deref().unwrap_or("")),
            style,
            ident(&e.from),
            ident(&e.to),
        ));
    }

    // Named layers as cells parented to the root (id="0"); hidden → visible="0".
    let mut layer_cells = String::new();
    for lyr in &fc.layers {
        layer_cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"\"{} parent=\"0\"/>\n",
            ident(&lyr.id),
            xml_escape(&lyr.label),
            if lyr.visible { "" } else { " visible=\"0\"" },
        ));
    }

    format!(
        "  <diagram id=\"{}\" name=\"{}\">\n    \
         <mxGraphModel dx=\"{:.0}\" dy=\"{:.0}\" grid=\"1\" gridSize=\"10\" guides=\"1\" \
         tooltips=\"1\" connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" \
         math=\"0\" shadow=\"0\">\n      <root>\n        \
         <mxCell id=\"0\"/>\n        <mxCell id=\"1\" parent=\"0\"/>\n{layer_cells}{cells}      </root>\n    \
         </mxGraphModel>\n  </diagram>\n",
        ident(name),
        xml_escape(name),
        (l.width + 40.0).max(800.0),
        (l.height + header + 40.0).max(600.0),
    )
}

/// Assign sequential step numbers (1-based) to step nodes in flow order.
/// Terminators (stadium start/end) are skipped — they are not "steps".
/// Ordering: BFS from start terminators (or, lacking any, the first node),
/// which follows the reading order of the flow; ties fall back to node order.
fn number_steps(fc: &Flowchart) -> HashMap<&str, usize> {
    use std::collections::{HashSet, VecDeque};

    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut indeg: HashMap<&str, usize> = fc.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    for e in &fc.edges {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
        *indeg.entry(e.to.as_str()).or_insert(0) += 1;
    }

    // Seeds: start terminators with no incoming edge, else any zero-indegree
    // node, else the first node. Preserve declaration order among seeds.
    let mut seeds: Vec<&str> = fc
        .nodes
        .iter()
        .filter(|n| n.shape == Shape::Stadium && indeg.get(n.id.as_str()).copied().unwrap_or(0) == 0)
        .map(|n| n.id.as_str())
        .collect();
    if seeds.is_empty() {
        seeds = fc
            .nodes
            .iter()
            .filter(|n| indeg.get(n.id.as_str()).copied().unwrap_or(0) == 0)
            .map(|n| n.id.as_str())
            .collect();
    }
    if seeds.is_empty() {
        if let Some(n) = fc.nodes.first() {
            seeds.push(n.id.as_str());
        }
    }

    let is_step = |id: &str| {
        fc.nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.shape != Shape::Stadium)
            .unwrap_or(false)
    };

    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    for s in &seeds {
        if visited.insert(s) {
            queue.push_back(s);
        }
    }
    let mut numbers: HashMap<&str, usize> = HashMap::new();
    let mut counter = 0usize;
    while let Some(u) = queue.pop_front() {
        if is_step(u) {
            counter += 1;
            numbers.insert(u, counter);
        }
        if let Some(succ) = adj.get(u) {
            for &v in succ {
                if visited.insert(v) {
                    queue.push_back(v);
                }
            }
        }
    }
    // Any node not reached by BFS (disconnected) still gets a number in
    // declaration order so nothing is left unlabeled.
    for n in &fc.nodes {
        if n.shape != Shape::Stadium && !numbers.contains_key(n.id.as_str()) {
            counter += 1;
            numbers.insert(n.id.as_str(), counter);
        }
    }
    numbers
}

/// Swimlane band style. Title bar on the left for horizontal flow (LR/RL),
/// on top for vertical flow (TB/BT).
fn drawio_lane_style(vertical: bool) -> String {
    format!(
        "swimlane;whiteSpace=wrap;html=1;startSize={:.0};horizontal={};\
         fillColor=#F5F8FB;swimlaneFillColor=#FFFFFF;strokeColor=#9DB3C8;\
         fontColor=#1F2A37;fontStyle=1;fontSize=13;",
        layout::LANE_TITLE,
        if vertical { 1 } else { 0 },
    )
}

fn drawio_container_style(sg: &Subgraph) -> String {
    let vertical = sg.orientation.as_deref() == Some("vertical");
    match sg.kind {
        ContainerKind::Group => {
            "rounded=0;whiteSpace=wrap;html=1;dashed=1;verticalAlign=top;fillColor=none;".to_string()
        }
        ContainerKind::Container => {
            "rounded=0;whiteSpace=wrap;html=1;verticalAlign=top;container=1;collapsible=0;fillColor=none;"
                .to_string()
        }
        ContainerKind::Swimlane => format!(
            "swimlane;whiteSpace=wrap;html=1;startSize=24;horizontal={};",
            if vertical { 0 } else { 1 }
        ),
        ContainerKind::Pool => format!(
            "swimlane;whiteSpace=wrap;html=1;startSize=24;childLayout=stackLayout;\
             horizontal={};horizontalStack={};resizeParent=1;collapsible=0;",
            if vertical { 0 } else { 1 },
            if vertical { 1 } else { 0 },
        ),
    }
}

/// The draw.io cell `value` for a node. UML class nodes render as an HTML
/// table (title + `<hr>` compartments); everything else is the escaped label
/// (which is also valid HTML when `html=1`).
fn drawio_node_value(node: &super::Node) -> String {
    if node.shape == Shape::UmlClass {
        let mut html = format!(
            "<p style=\"margin:0px;margin-top:4px;text-align:center;\"><b>{}</b></p>",
            xml_escape(&node.label)
        );
        for comp in &node.compartments {
            html.push_str("<hr size=\"1\"/>");
            let lines: Vec<String> = comp.iter().map(|l| xml_escape(l)).collect();
            html.push_str(&format!(
                "<p style=\"margin:0px;margin-left:4px;\">{}</p>",
                lines.join("<br/>")
            ));
        }
        // The value attribute must itself be XML-escaped HTML.
        return xml_escape(&html);
    }
    xml_escape(&node.label)
}

/// Resolve a node's draw.io style, preferring a stencil token over image/shape.
fn drawio_node_style(node: &super::Node) -> String {
    if let Some(key) = &node.stencil {
        if let Some(r) = crate::stencils::resolve(key) {
            let mut s = crate::stencils::drawio_base(&r);
            push_common(&mut s, &node.style);
            return s;
        }
    }
    drawio_style(node.shape, &node.style, node.image.as_deref())
}

fn drawio_style(shape: Shape, style: &Style, image: Option<&str>) -> String {
    if let Some(uri) = image {
        let mut s = format!(
            "shape=image;html=1;imageAspect=0;aspect=fixed;verticalLabelPosition=bottom;\
             verticalAlign=top;image={};",
            xml_escape(uri)
        );
        push_common(&mut s, style);
        return s;
    }
    let base = match shape {
        Shape::Rectangle => "rounded=0;whiteSpace=wrap;html=1;".to_string(),
        Shape::RoundRect => "rounded=1;whiteSpace=wrap;html=1;".to_string(),
        Shape::Stadium => "rounded=1;whiteSpace=wrap;html=1;arcSize=40;".to_string(),
        Shape::Subroutine => "shape=process;whiteSpace=wrap;html=1;".to_string(),
        Shape::Cylinder => "shape=cylinder3;whiteSpace=wrap;html=1;boundedLbl=1;".to_string(),
        Shape::Circle => "ellipse;whiteSpace=wrap;html=1;aspect=fixed;".to_string(),
        Shape::DoubleCircle => {
            "ellipse;shape=doubleEllipse;whiteSpace=wrap;html=1;aspect=fixed;".to_string()
        }
        Shape::Diamond => "rhombus;whiteSpace=wrap;html=1;".to_string(),
        Shape::Hexagon => "shape=hexagon;perimeter=hexagonPerimeter2;whiteSpace=wrap;html=1;".to_string(),
        Shape::Parallelogram => {
            "shape=parallelogram;perimeter=parallelogramPerimeter;whiteSpace=wrap;html=1;".to_string()
        }
        Shape::ParallelogramAlt => {
            "shape=parallelogram;perimeter=parallelogramPerimeter;whiteSpace=wrap;html=1;flipH=1;"
                .to_string()
        }
        Shape::Trapezoid => {
            "shape=trapezoid;perimeter=trapezoidPerimeter;whiteSpace=wrap;html=1;".to_string()
        }
        Shape::TrapezoidAlt => {
            "shape=trapezoid;perimeter=trapezoidPerimeter;whiteSpace=wrap;html=1;direction=south;"
                .to_string()
        }
        Shape::Note => "shape=note;whiteSpace=wrap;html=1;size=14;".to_string(),
        Shape::Card => "shape=card;whiteSpace=wrap;html=1;size=14;".to_string(),
        Shape::Document => "shape=document;whiteSpace=wrap;html=1;boundedLbl=1;".to_string(),
        Shape::UmlClass => {
            "verticalAlign=top;align=left;overflow=fill;html=1;whiteSpace=wrap;".to_string()
        }
    };
    let mut s = base;
    apply_theme(&mut s, shape, style);
    push_common(&mut s, style);
    s
}

/// Default fill/stroke/font per shape, applied only for fields the caller did
/// not set. Gives polished output (tinted terminators, amber decisions, blue
/// documents, soft-blue process boxes) without requiring explicit styling.
fn apply_theme(s: &mut String, shape: Shape, style: &Style) {
    let (fill, stroke, font) = match shape {
        Shape::Stadium | Shape::Circle | Shape::DoubleCircle => ("#1F6FB8", "#15527F", "#FFFFFF"),
        Shape::Diamond => ("#FFF2CC", "#D6B656", "#7A5C00"),
        Shape::Document | Shape::Note | Shape::Card => ("#DAE8FC", "#6C8EBF", "#1F2A37"),
        Shape::Cylinder => ("#E1D5E7", "#9673A6", "#1F2A37"),
        _ => ("#EAF2FB", "#4A7AAA", "#1F2A37"),
    };
    if style.fill.is_none() {
        s.push_str(&format!("fillColor={fill};"));
    }
    if style.stroke.is_none() {
        s.push_str(&format!("strokeColor={stroke};"));
    }
    if style.text_color.is_none() {
        s.push_str(&format!("fontColor={font};"));
    }
    if matches!(shape, Shape::Stadium) && style.bold.is_none() {
        s.push_str("fontStyle=1;");
    }
    if style.font_size.is_none() {
        s.push_str("fontSize=12;");
    }
}

/// Append the shared style fields onto a draw.io style string.
fn push_common(s: &mut String, style: &Style) {
    if let Some(f) = &style.fill {
        s.push_str(&format!("fillColor={f};"));
    }
    if let Some(st) = &style.stroke {
        s.push_str(&format!("strokeColor={st};"));
    }
    if let Some(c) = &style.text_color {
        s.push_str(&format!("fontColor={c};"));
    }
    if let Some(w) = style.stroke_width {
        s.push_str(&format!("strokeWidth={w};"));
    }
    if let Some(f) = &style.font_family {
        s.push_str(&format!("fontFamily={f};"));
    }
    if let Some(sz) = style.font_size {
        s.push_str(&format!("fontSize={sz};"));
    }
    // fontStyle bitmask: 1=bold, 2=italic.
    let mut fs = 0;
    if style.bold == Some(true) {
        fs |= 1;
    }
    if style.italic == Some(true) {
        fs |= 2;
    }
    if fs != 0 {
        s.push_str(&format!("fontStyle={fs};"));
    }
    if let Some(a) = &style.align {
        s.push_str(&format!("align={a};"));
    }
    if let Some(o) = style.opacity {
        s.push_str(&format!("opacity={o};"));
    }
    if style.rounded == Some(true) && !s.contains("rounded=1") {
        s.push_str("rounded=1;");
    }
    if style.shadow == Some(true) {
        s.push_str("shadow=1;");
    }
    if style.dashed == Some(true) {
        s.push_str("dashed=1;");
    }
    if let Some(g) = &style.gradient {
        s.push_str(&format!("gradientColor={g};"));
    }
    if style.sketch == Some(true) {
        s.push_str("sketch=1;");
    }
    if style.glass == Some(true) {
        s.push_str("glass=1;");
    }
}

fn drawio_edge_style(e: &Edge) -> String {
    // Self-loop: use draw.io's loop routing so it arcs cleanly off one side.
    if e.from == e.to {
        let mut s = "edgeStyle=loopEdgeStyle;html=1;rounded=1;".to_string();
        match e.line {
            LineStyle::Dotted => s.push_str("dashed=1;"),
            LineStyle::Thick => s.push_str("strokeWidth=3;"),
            LineStyle::Solid => {}
        }
        s.push_str(&format!(
            "startArrow={};endArrow={};endFill=1;strokeColor={};fontColor=#44515E;fontSize=11;labelBackgroundColor=#FFFFFF;",
            e.resolved_start().drawio(),
            e.resolved_end().drawio(),
            e.color.as_deref().unwrap_or("#44515E"),
        ));
        return s;
    }
    let routing = e
        .routing
        .unwrap_or(super::EdgeRouting::Orthogonal)
        .drawio();
    let mut s = format!("{routing}html=1;jettySize=auto;");
    match e.line {
        LineStyle::Dotted => s.push_str("dashed=1;"),
        LineStyle::Thick => s.push_str("strokeWidth=3;"),
        LineStyle::Solid => {}
    }
    s.push_str(&format!(
        "startArrow={};endArrow={};endFill=1;",
        e.resolved_start().drawio(),
        e.resolved_end().drawio()
    ));
    // Default connector color + white label backing so labels stay legible.
    s.push_str(&format!(
        "strokeColor={};fontColor=#44515E;fontSize=11;labelBackgroundColor=#FFFFFF;",
        e.color.as_deref().unwrap_or("#44515E"),
    ));
    s
}

fn bounds(l: &layout::Layout, members: &[String]) -> Option<Box> {
    let mut it = members.iter().filter_map(|m| l.boxes.get(m).copied());
    let first = it.next()?;
    let (mut x0, mut y0) = (first.x, first.y);
    let (mut x1, mut y1) = (first.x + first.w, first.y + first.h);
    for b in it {
        x0 = x0.min(b.x);
        y0 = y0.min(b.y);
        x1 = x1.max(b.x + b.w);
        y1 = y1.max(b.y + b.h);
    }
    Some(Box { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

// ---------------------------------------------------------------------------
// SVG
// ---------------------------------------------------------------------------

/// Default fill/stroke/font per shape — the same palette the draw.io theme uses,
/// so the SVG canvas matches the diagrams.net export.
fn theme_colors(shape: Shape) -> (&'static str, &'static str, &'static str) {
    match shape {
        Shape::Stadium | Shape::Circle | Shape::DoubleCircle => ("#1F6FB8", "#15527F", "#FFFFFF"),
        Shape::Diamond => ("#FFF2CC", "#D6B656", "#7A5C00"),
        Shape::Document | Shape::Note | Shape::Card => ("#DAE8FC", "#6C8EBF", "#1F2A37"),
        Shape::Cylinder => ("#E1D5E7", "#9673A6", "#1F2A37"),
        _ => ("#EAF2FB", "#4A7AAA", "#1F2A37"),
    }
}

/// Clone a node style, filling in themed defaults for any unset visual field.
fn themed_style(shape: Shape, style: &Style) -> Style {
    let (fill, stroke, font) = theme_colors(shape);
    let mut s = style.clone();
    if s.fill.is_none() {
        s.fill = Some(fill.to_string());
    }
    if s.stroke.is_none() {
        s.stroke = Some(stroke.to_string());
    }
    if s.text_color.is_none() {
        s.text_color = Some(font.to_string());
    }
    if s.font_size.is_none() {
        s.font_size = Some(13.0);
    }
    if matches!(shape, Shape::Stadium) && s.bold.is_none() {
        s.bold = Some(true);
    }
    s
}

/// Absolute point on box `b` for an anchor fraction `(fx, fy)`.
fn anchor_point(b: Box, (fx, fy): (f64, f64)) -> (f64, f64) {
    (b.x + fx * b.w, b.y + fy * b.h)
}

/// Outward direction for an anchor on the box border.
fn anchor_dir((fx, fy): (f64, f64)) -> (f64, f64) {
    if fy <= 0.01 {
        (0.0, -1.0)
    } else if fy >= 0.99 {
        (0.0, 1.0)
    } else if fx <= 0.01 {
        (-1.0, 0.0)
    } else if fx >= 0.99 {
        (1.0, 0.0)
    } else {
        (0.0, 0.0)
    }
}

/// Pick default exit/entry anchors from the relative position of two boxes,
/// choosing the dominant axis (so forward and back edges both route cleanly).
fn default_anchors(a: Box, b: Box) -> ((f64, f64), (f64, f64)) {
    let dx = (b.x + b.w / 2.0) - (a.x + a.w / 2.0);
    let dy = (b.y + b.h / 2.0) - (a.y + a.h / 2.0);
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 { ((1.0, 0.5), (0.0, 0.5)) } else { ((0.0, 0.5), (1.0, 0.5)) }
    } else if dy >= 0.0 {
        ((0.5, 1.0), (0.5, 0.0))
    } else {
        ((0.5, 0.0), (0.5, 1.0))
    }
}

/// Orthogonal (manhattan) route between two boxes given their anchors: a stub
/// out of each end then an L/Z connection — the flowchart connector look.
fn ortho_route(a: Box, b: Box, ea: (f64, f64), na: (f64, f64)) -> Vec<(f64, f64)> {
    let stub = 16.0;
    let p0 = anchor_point(a, ea);
    let p3 = anchor_point(b, na);
    let d0 = anchor_dir(ea);
    let d3 = anchor_dir(na);
    let p1 = (p0.0 + d0.0 * stub, p0.1 + d0.1 * stub);
    let p2 = (p3.0 + d3.0 * stub, p3.1 + d3.1 * stub);

    let mut pts = vec![p0, p1];
    let aligned = (p1.0 - p2.0).abs() < 0.5 || (p1.1 - p2.1).abs() < 0.5;
    if !aligned {
        if d0.0.abs() > 0.0 {
            let midx = (p1.0 + p2.0) / 2.0;
            pts.push((midx, p1.1));
            pts.push((midx, p2.1));
        } else if d0.1.abs() > 0.0 {
            let midy = (p1.1 + p2.1) / 2.0;
            pts.push((p1.0, midy));
            pts.push((p2.0, midy));
        } else {
            // Center anchor (rare): single elbow.
            pts.push((p2.0, p1.1));
        }
    }
    pts.push(p2);
    pts.push(p3);

    // Drop consecutive duplicates.
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().map(|q| (q.0 - p.0).abs() < 0.5 && (q.1 - p.1).abs() < 0.5).unwrap_or(false) {
            continue;
        }
        out.push(p);
    }
    out
}

/// Midpoint along a polyline (by cumulative length) for label placement.
fn polyline_midpoint(pts: &[(f64, f64)]) -> (f64, f64) {
    if pts.is_empty() {
        return (0.0, 0.0);
    }
    let total: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    let mut target = total / 2.0;
    for w in pts.windows(2) {
        let d = dist(w[0], w[1]);
        if d >= target {
            let t = if d > 0.0 { target / d } else { 0.0 };
            return (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t);
        }
        target -= d;
    }
    *pts.last().unwrap()
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Compute exit/entry anchor fractions per edge — the same decision-tip /
/// fan-out spreading logic the draw.io exporter uses, so SVG matches it.
fn compute_edge_anchors(
    fc: &Flowchart,
    l: &layout::Layout,
) -> (HashMap<usize, (f64, f64)>, HashMap<usize, (f64, f64)>) {
    let mut out_edges: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut in_edges: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in fc.edges.iter().enumerate() {
        out_edges.entry(e.from.as_str()).or_default().push(i);
        in_edges.entry(e.to.as_str()).or_default().push(i);
    }
    let shape_of: HashMap<&str, Shape> =
        fc.nodes.iter().map(|n| (n.id.as_str(), n.shape)).collect();
    let is_diamond = |id: &str| matches!(shape_of.get(id), Some(Shape::Diamond));

    let vertical = fc.direction.is_vertical();
    let cross_of = |bx: Box| if vertical { bx.x + bx.w / 2.0 } else { bx.y + bx.h / 2.0 };
    let main_of = |bx: Box| if vertical { bx.y + bx.h / 2.0 } else { bx.x + bx.w / 2.0 };

    const TOP: (f64, f64) = (0.5, 0.0);
    const BOTTOM: (f64, f64) = (0.5, 1.0);
    const LEFT: (f64, f64) = (0.0, 0.5);
    const RIGHT: (f64, f64) = (1.0, 0.5);

    let diamond_entry_tip = |a: Box, b: Box| -> (f64, f64) {
        let dmain = main_of(a) - main_of(b);
        let dcross = cross_of(a) - cross_of(b);
        let rear = if vertical { TOP } else { LEFT };
        let front = if vertical { BOTTOM } else { RIGHT };
        let up = if vertical { LEFT } else { TOP };
        let down = if vertical { RIGHT } else { BOTTOM };
        if dmain.abs() >= dcross.abs() {
            if dmain <= 0.0 { rear } else { front }
        } else if dcross < 0.0 {
            up
        } else {
            down
        }
    };

    let frac = |rank: usize, count: usize| (rank as f64 + 1.0) / (count as f64 + 1.0);
    let order_by_target_cross = |idxs: &[usize], pick_other: &dyn Fn(usize) -> Box| -> Vec<usize> {
        let mut ord = idxs.to_vec();
        ord.sort_by(|&i, &j| {
            cross_of(pick_other(i))
                .partial_cmp(&cross_of(pick_other(j)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ord
    };

    let mut exit_anchor: HashMap<usize, (f64, f64)> = HashMap::new();
    for (src, idxs) in out_edges.iter() {
        if idxs.len() < 2 {
            continue;
        }
        let a = l.get(src);
        if is_diamond(src) {
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].to));
            let (up, fwd, down) = if vertical { (LEFT, BOTTOM, RIGHT) } else { (TOP, RIGHT, BOTTOM) };
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let b = l.get(&fc.edges[ei].to);
                let dc = cross_of(b) - cross_of(a);
                let tip = if n == 2 {
                    let other = l.get(&fc.edges[ord[1 - rank]].to);
                    let dc_other = cross_of(other) - cross_of(a);
                    if (dc - dc_other).abs() >= 24.0 {
                        if dc <= dc_other { up } else { down }
                    } else if rank == 0 {
                        fwd
                    } else {
                        down
                    }
                } else if rank == 0 {
                    up
                } else if rank + 1 == n {
                    down
                } else {
                    fwd
                };
                exit_anchor.insert(ei, tip);
            }
        } else {
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].to));
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let f = frac(rank, n);
                exit_anchor.insert(ei, if vertical { (f, 1.0) } else { (1.0, f) });
            }
        }
    }
    for (src, idxs) in out_edges.iter() {
        if idxs.len() == 1 && is_diamond(src) {
            let ei = idxs[0];
            let a = l.get(src);
            let b = l.get(&fc.edges[ei].to);
            let dc = cross_of(b) - cross_of(a);
            let (up, fwd, down) = if vertical { (LEFT, BOTTOM, RIGHT) } else { (TOP, RIGHT, BOTTOM) };
            let tip = if dc.abs() < 24.0 { fwd } else if dc < 0.0 { up } else { down };
            exit_anchor.insert(ei, tip);
        }
    }

    let mut entry_anchor: HashMap<usize, (f64, f64)> = HashMap::new();
    for (tgt, idxs) in in_edges.iter() {
        if idxs.len() < 2 {
            continue;
        }
        if is_diamond(tgt) {
            let b = l.get(tgt);
            let mut used: Vec<(f64, f64)> = Vec::new();
            for &ei in idxs.iter() {
                let mut tip = diamond_entry_tip(l.get(&fc.edges[ei].from), b);
                let mut guard = 0;
                while used.iter().any(|u| (u.0 - tip.0).abs() < 0.01 && (u.1 - tip.1).abs() < 0.01)
                    && guard < 4
                {
                    tip = match tip {
                        TOP => RIGHT,
                        RIGHT => BOTTOM,
                        BOTTOM => LEFT,
                        _ => TOP,
                    };
                    guard += 1;
                }
                used.push(tip);
                entry_anchor.insert(ei, tip);
            }
        } else {
            let ord = order_by_target_cross(idxs, &|i| l.get(&fc.edges[i].from));
            let n = ord.len();
            for (rank, &ei) in ord.iter().enumerate() {
                let f = frac(rank, n);
                entry_anchor.insert(ei, if vertical { (f, 0.0) } else { (0.0, f) });
            }
        }
    }
    for (tgt, idxs) in in_edges.iter() {
        if idxs.len() == 1 && is_diamond(tgt) {
            let ei = idxs[0];
            entry_anchor.insert(ei, diamond_entry_tip(l.get(&fc.edges[ei].from), l.get(tgt)));
        }
    }

    (exit_anchor, entry_anchor)
}

pub fn to_svg(fc: &Flowchart) -> String {
    let l = layout::compute(fc);
    let mut body = String::new();

    // Header band for the title (everything else is drawn inside a <g> shifted
    // down by `header`, so coordinates stay in layout space).
    let header = if fc.title.as_deref().map(|t| !t.is_empty()).unwrap_or(false) {
        46.0
    } else {
        0.0
    };

    // Swimlane bands (full-length) from the lane-aware layout — drawn first so
    // nodes sit on top. Matches the draw.io band styling.
    let lane_ids: std::collections::HashSet<&str> =
        l.lanes.iter().map(|lg| lg.id.as_str()).collect();
    let vertical = fc.direction.is_vertical();
    for lg in &l.lanes {
        let label = fc
            .subgraphs
            .iter()
            .find(|s| s.id == lg.id)
            .map(|s| s.label.as_str())
            .unwrap_or("");
        let (b, t) = (lg.b, layout::LANE_TITLE);
        // Band body (white) + a tinted title strip.
        body.push_str(&format!(
            "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"#FFFFFF\" stroke=\"#9DB3C8\"/>\n",
            b.x, b.y, b.w, b.h,
        ));
        if vertical {
            body.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{t:.1}\" \
                 fill=\"#F5F8FB\" stroke=\"#9DB3C8\"/>\n  \
                 <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"13\" \
                 font-weight=\"bold\" fill=\"#1F2A37\" text-anchor=\"middle\">{}</text>\n",
                b.x, b.y, b.w, b.x + b.w / 2.0, b.y + t / 2.0 + 4.0, xml_escape(label),
            ));
        } else {
            let (cx, cy) = (b.x + t / 2.0, b.y + b.h / 2.0);
            body.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{t:.1}\" height=\"{:.1}\" \
                 fill=\"#F5F8FB\" stroke=\"#9DB3C8\"/>\n  \
                 <text x=\"{cx:.1}\" y=\"{cy:.1}\" font-family=\"Helvetica\" font-size=\"13\" \
                 font-weight=\"bold\" fill=\"#1F2A37\" text-anchor=\"middle\" \
                 transform=\"rotate(-90 {cx:.1} {cy:.1})\">{}</text>\n",
                b.x, b.y, b.h, xml_escape(label),
            ));
        }
    }

    // Non-lane container backdrops, parents first (so children draw on top).
    for i in ordered_containers(fc) {
        let sg = &fc.subgraphs[i];
        if lane_ids.contains(sg.id.as_str()) {
            continue;
        }
        if let Some(b) = container_abs_box(fc, &l, &sg.id) {
            let (stroke, dash, fill) = match sg.kind {
                ContainerKind::Group => ("#999", " stroke-dasharray=\"4 3\"", "none"),
                _ => ("#666", "", "#00000008"),
            };
            body.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
                 fill=\"{fill}\" stroke=\"{stroke}\"{dash}/>\n  \
                 <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"12\" fill=\"#555\">{}</text>\n",
                b.x, b.y, b.w, b.h, b.x + 6.0, b.y + 16.0, xml_escape(&sg.label),
            ));
        }
    }

    // Edges — orthogonal connectors using shared anchor logic.
    let (exit_anchor, entry_anchor) = compute_edge_anchors(fc, &l);
    for (i, e) in fc.edges.iter().enumerate() {
        let (a, b) = (l.get(&e.from), l.get(&e.to));
        let dash = match e.line {
            LineStyle::Dotted => " stroke-dasharray=\"5 4\"",
            _ => "",
        };
        let width = if e.line == LineStyle::Thick { 3.0 } else { 1.6 };
        let color = e.color.as_deref().unwrap_or("#44515E");
        // Self-loop: a small arc off the node's top edge.
        if e.from == e.to {
            let lx = a.x + a.w * 0.7;
            let rx = a.x + a.w * 0.95;
            let ty = a.y;
            let r = 14.0;
            body.push_str(&format!(
                "  <path d=\"M{lx:.1},{ty:.1} C{:.1},{:.1} {:.1},{:.1} {rx:.1},{ty:.1}\" \
                 fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\"{dash} marker-end=\"url(#arrow)\"/>\n",
                lx, ty - r * 2.0, rx, ty - r * 2.0,
            ));
            if let Some(lbl) = &e.label {
                if !lbl.is_empty() {
                    body.push_str(&format!(
                        "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"11\" \
                         fill=\"#44515E\" text-anchor=\"middle\">{}</text>\n",
                        (lx + rx) / 2.0,
                        ty - r * 2.0 - 2.0,
                        xml_escape(lbl),
                    ));
                }
            }
            continue;
        }
        let (def_e, def_n) = default_anchors(a, b);
        let ea = exit_anchor.get(&i).copied().unwrap_or(def_e);
        let na = entry_anchor.get(&i).copied().unwrap_or(def_n);
        let pts = ortho_route(a, b, ea, na);
        let mut markers = String::new();
        if e.resolved_end() != super::Arrow::None {
            markers.push_str(" marker-end=\"url(#arrow)\"");
        }
        if e.resolved_start() != super::Arrow::None {
            markers.push_str(" marker-start=\"url(#arrow-start)\"");
        }
        let pts_str = pts
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect::<Vec<_>>()
            .join(" ");
        body.push_str(&format!(
            "  <polyline points=\"{pts_str}\" fill=\"none\" stroke=\"{color}\" \
             stroke-width=\"{width}\"{dash}{markers}/>\n",
        ));
        if let Some(lbl) = &e.label {
            if !lbl.is_empty() {
                let (mx, my) = polyline_midpoint(&pts);
                let w = lbl.chars().count() as f64 * 6.4 + 8.0;
                body.push_str(&format!(
                    "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{w:.1}\" height=\"16\" rx=\"2\" \
                     fill=\"#FFFFFF\" opacity=\"0.92\"/>\n  \
                     <text x=\"{mx:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"11\" \
                     fill=\"#44515E\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>\n",
                    mx - w / 2.0, my - 8.0, my, xml_escape(lbl),
                ));
            }
        }
    }

    // Step-number badges (when enabled).
    let step_numbers = if fc.number_steps { number_steps(fc) } else { HashMap::new() };

    for node in &fc.nodes {
        let b = l.get(&node.id);
        if let Some(uri) = &node.image {
            body.push_str(&format!(
                "  <image href=\"{}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 preserveAspectRatio=\"xMidYMid meet\"/>\n",
                xml_escape(uri), b.x, b.y, b.w, b.h,
            ));
            body.push_str(&svg_label(node, &node.style, b));
        } else if node.shape == Shape::UmlClass {
            body.push_str(&svg_uml_class(node, b));
            continue;
        } else {
            let style = themed_style(node.shape, &node.style);
            body.push_str(&svg_shape(node.shape, b, &style));
            body.push_str(&svg_label(node, &style, b));
        }

        if let Some(&num) = step_numbers.get(node.id.as_str()) {
            let (cx, cy) = (b.x + b.w - 6.0, b.y + 6.0);
            body.push_str(&format!(
                "  <circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"11\" fill=\"#1F2A37\" stroke=\"#FFFFFF\"/>\n  \
                 <text x=\"{cx:.1}\" y=\"{cy:.1}\" font-family=\"Helvetica\" font-size=\"11\" \
                 font-weight=\"bold\" fill=\"#FFFFFF\" text-anchor=\"middle\" dominant-baseline=\"central\">{num}</text>\n",
            ));
        }
    }

    // Title text on top.
    let mut title_cell = String::new();
    if header > 0.0 {
        let title = fc.title.as_deref().unwrap_or("");
        title_cell = format!(
            "  <text x=\"{:.1}\" y=\"28\" font-family=\"Helvetica\" font-size=\"18\" \
             font-weight=\"bold\" fill=\"#1F2A37\">{}</text>\n",
            layout::MARGIN,
            xml_escape(title),
        );
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" \
         viewBox=\"0 0 {:.0} {:.0}\">\n  <defs>\n    \
         <marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" \
         orient=\"auto\" markerUnits=\"strokeWidth\">\n      \
         <path d=\"M0,0 L8,3 L0,6 z\" fill=\"#44515E\"/>\n    </marker>\n    \
         <marker id=\"arrow-start\" markerWidth=\"10\" markerHeight=\"10\" refX=\"0\" refY=\"3\" \
         orient=\"auto\" markerUnits=\"strokeWidth\">\n      \
         <path d=\"M8,0 L0,3 L8,6 z\" fill=\"#44515E\"/>\n    </marker>\n  </defs>\n  \
         <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n{title_cell}  \
         <g transform=\"translate(0,{header})\">\n{body}  </g>\n</svg>\n",
        l.width,
        l.height + header,
        l.width,
        l.height + header,
    )
}

/// Render a UML class box: bordered rect, bold title row, and a separated row
/// per compartment line.
fn svg_uml_class(node: &super::Node, b: Box) -> String {
    let fill = node.style.fill.as_deref().unwrap_or("#FFFFFF");
    let stroke = node.style.stroke.as_deref().unwrap_or("#33415C");
    let title_h = 22.0;
    let line_h = 16.0;
    let mut s = format!(
        "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{fill}\" \
         stroke=\"{stroke}\" stroke-width=\"1.2\"/>\n  \
         <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"13\" font-weight=\"bold\" \
         text-anchor=\"middle\" fill=\"#1F2A37\">{}</text>\n",
        b.x, b.y, b.w, b.h,
        b.x + b.w / 2.0, b.y + 15.0,
        xml_escape(&node.label),
    );
    let mut y = b.y + title_h;
    for comp in &node.compartments {
        s.push_str(&format!(
            "  <line x1=\"{:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"{stroke}\" stroke-width=\"1\"/>\n",
            b.x, b.x + b.w,
        ));
        for line in comp {
            y += line_h;
            s.push_str(&format!(
                "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"12\" fill=\"#1F2A37\">{}</text>\n",
                b.x + 6.0, y - 3.0, xml_escape(line),
            ));
        }
        y += 4.0;
    }
    s
}

fn svg_label(node: &super::Node, s: &Style, b: Box) -> String {
    let (anchor, tx) = match s.align.as_deref() {
        Some("left") => ("start", b.x + 6.0),
        Some("right") => ("end", b.x + b.w - 6.0),
        _ => ("middle", b.x + b.w / 2.0),
    };
    let mut extra = String::new();
    if s.bold == Some(true) {
        extra.push_str(" font-weight=\"bold\"");
    }
    if s.italic == Some(true) {
        extra.push_str(" font-style=\"italic\"");
    }
    let size = s.font_size.unwrap_or(13.0);
    let family = s.font_family.as_deref().unwrap_or("Helvetica");
    let fill = s.text_color.as_deref().unwrap_or("#000");

    // Image labels sit below the image; everything else is centred in the box.
    if node.image.is_some() {
        let baseline = b.y + b.h + 12.0;
        return format!(
            "  <text x=\"{tx:.1}\" y=\"{baseline:.1}\" font-family=\"{family}\" \
             font-size=\"{size}\" text-anchor=\"{anchor}\" dominant-baseline=\"middle\" \
             fill=\"{fill}\"{extra}>{}</text>\n",
            xml_escape(&plain_label(node)),
        );
    }

    // Word-wrap to the usable text width of the shape (a diamond only uses its
    // centre, so wrap narrower), so long labels don't overflow the box.
    let usable = match node.shape {
        Shape::Diamond => b.w * 0.5,
        Shape::Stadium | Shape::Circle | Shape::DoubleCircle => b.w - 28.0,
        _ => b.w - 16.0,
    };
    let lines = wrap_label(&plain_label(node), usable.max(24.0));
    let line_h = (size * 1.2).max(14.0);
    let cy = b.y + b.h / 2.0;
    let start = cy - (lines.len() as f64 - 1.0) * line_h / 2.0;

    let mut out = format!(
        "  <text x=\"{tx:.1}\" y=\"{start:.1}\" font-family=\"{family}\" font-size=\"{size}\" \
         text-anchor=\"{anchor}\" dominant-baseline=\"middle\" fill=\"{fill}\"{extra}>",
    );
    for (i, line) in lines.iter().enumerate() {
        let dy = if i == 0 { 0.0 } else { line_h };
        out.push_str(&format!(
            "<tspan x=\"{tx:.1}\" dy=\"{dy:.1}\">{}</tspan>",
            xml_escape(line)
        ));
    }
    out.push_str("</text>\n");
    out
}

/// Greedy word-wrap for SVG labels at ~7.1px/char (matches layout metrics).
fn wrap_label(label: &str, max_w: f64) -> Vec<String> {
    const CHAR_W: f64 = 7.1;
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0.0;
    for w in words {
        let wl = w.chars().count() as f64 * CHAR_W;
        let space = if cur.is_empty() { 0.0 } else { CHAR_W };
        if !cur.is_empty() && cur_w + space + wl > max_w {
            lines.push(std::mem::take(&mut cur));
            cur_w = wl;
            cur.push_str(w);
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(w);
            cur_w += space + wl;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn svg_shape(shape: Shape, b: Box, style: &Style) -> String {
    let fill = style.fill.as_deref().unwrap_or("#ffffff");
    let stroke = style.stroke.as_deref().unwrap_or("#333333");
    let sw = style.stroke_width.unwrap_or(1.5);
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    match shape {
        Shape::Diamond => format!(
            "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
             fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
            cx, b.y, b.x + b.w, cy, cx, b.y + b.h, b.x, cy,
        ),
        Shape::Circle | Shape::DoubleCircle => {
            let mut s = format!(
                "  <ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                b.w / 2.0,
                b.h / 2.0,
            );
            if shape == Shape::DoubleCircle {
                s.push_str(&format!(
                    "  <ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" \
                     fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                    b.w / 2.0 - 4.0,
                    b.h / 2.0 - 4.0,
                ));
            }
            s
        }
        Shape::Hexagon => {
            let dx = b.w * 0.2;
            format!(
                "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                b.x + dx, b.y, b.x + b.w - dx, b.y, b.x + b.w, cy,
                b.x + b.w - dx, b.y + b.h, b.x + dx, b.y + b.h, b.x, cy,
            )
        }
        Shape::Parallelogram | Shape::ParallelogramAlt => {
            let dx = b.w * 0.2;
            let (p0, p1, p2, p3) = if shape == Shape::Parallelogram {
                (b.x + dx, b.x + b.w, b.x + b.w - dx, b.x)
            } else {
                (b.x, b.x + b.w - dx, b.x + b.w, b.x + dx)
            };
            format!(
                "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                p0, b.y, p1, b.y, p2, b.y + b.h, p3, b.y + b.h,
            )
        }
        Shape::Trapezoid | Shape::TrapezoidAlt => {
            let dx = b.w * 0.2;
            let (t0, t1, bo0, bo1) = if shape == Shape::Trapezoid {
                (b.x + dx, b.x + b.w - dx, b.x + b.w, b.x)
            } else {
                (b.x, b.x + b.w, b.x + b.w - dx, b.x + dx)
            };
            format!(
                "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                t0, b.y, t1, b.y, bo0, b.y + b.h, bo1, b.y + b.h,
            )
        }
        Shape::Cylinder => {
            let ry = (b.h * 0.12).min(10.0);
            format!(
                "  <path d=\"M{:.1},{:.1} L{:.1},{:.1} A{:.1},{ry:.1} 0 0 0 {:.1},{:.1} L{:.1},{:.1} \
                 A{:.1},{ry:.1} 0 0 0 {:.1},{:.1} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n  \
                 <ellipse cx=\"{cx:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{ry:.1}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                b.x, b.y + ry, b.x, b.y + b.h - ry, b.w / 2.0, b.x + b.w, b.y + b.h - ry,
                b.x + b.w, b.y + ry, b.w / 2.0, b.x, b.y + ry,
                b.y + ry, b.w / 2.0,
            )
        }
        Shape::Note => {
            let f = 14.0;
            format!(
                "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                b.x, b.y, b.x + b.w - f, b.y, b.x + b.w, b.y + f,
                b.x + b.w, b.y + b.h, b.x, b.y + b.h,
            )
        }
        // Rectangle family (incl. stadium/round/subroutine/card/document) → rects.
        _ => {
            let rx = match shape {
                Shape::Stadium => b.h / 2.0,
                Shape::RoundRect | Shape::Card => 8.0,
                _ if style.rounded == Some(true) => 8.0,
                _ => 0.0,
            };
            let mut s = format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{rx:.1}\" \
                 fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                b.x, b.y, b.w, b.h,
            );
            if shape == Shape::Subroutine {
                s.push_str(&format!(
                    "  <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n  \
                     <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>\n",
                    b.x + 8.0, b.y, b.x + 8.0, b.y + b.h,
                    b.x + b.w - 8.0, b.y, b.x + b.w - 8.0, b.y + b.h,
                ));
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Arrow, ContainerKind, Direction, Shape};

    fn sample() -> Flowchart {
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_node("a", "Start", Shape::Stadium).unwrap();
        fc.add_node("b", "Decide", Shape::Diamond).unwrap();
        fc.add_node("c", "End", Shape::Stadium).unwrap();
        fc.add_edge("a", "b", None, LineStyle::Solid, true).unwrap();
        fc.add_edge("b", "c", Some("yes".into()), LineStyle::Dotted, true)
            .unwrap();
        fc
    }

    #[test]
    fn mermaid_has_header_and_edges() {
        let m = to_mermaid(&sample());
        assert!(m.starts_with("flowchart TD"));
        assert!(m.contains("a([Start])"));
        assert!(m.contains("b{Decide}"));
    }

    #[test]
    fn dot_is_digraph() {
        let d = to_dot(&sample());
        assert!(d.contains("digraph G"));
        assert!(d.contains("rankdir=TB"));
        assert!(d.contains("->"));
    }

    #[test]
    fn drawio_is_mxfile_multipage() {
        let mut doc = Document::new(Direction::TB);
        *doc.chart() = sample();
        doc.add_page(Some("Second".into()), Direction::LR);
        doc.chart().add_node("x", "X", Shape::Rectangle).unwrap();
        let x = to_drawio(&doc);
        assert!(x.contains("<mxfile"));
        assert_eq!(x.matches("<diagram").count(), 2);
        assert!(x.contains("rhombus"));
    }

    #[test]
    fn drawio_swimlane_renders_as_band() {
        // Swimlanes now render as full-length bands at the root layer (nodes are
        // placed absolutely on top), which avoids the old lane-overlap problem.
        let mut fc = Flowchart::new(Direction::LR);
        fc.add_node("n", "N", Shape::Rectangle).unwrap();
        fc.add_subgraph("pool", "Pool", vec![], ContainerKind::Pool, None, None).unwrap();
        fc.add_subgraph("lane", "Lane", vec!["n".into()], ContainerKind::Swimlane, None, Some("pool".into())).unwrap();
        let mut doc = Document::new(Direction::LR);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        // Lane is emitted as a swimlane band with its label.
        assert!(x.contains("swimlane"));
        assert!(x.contains("value=\"Lane\""));
        // Bands attach to the root layer, not nested via relative geometry.
        assert!(x.contains("id=\"lane\" value=\"Lane\""));
    }

    #[test]
    fn drawio_edge_arrowheads() {
        let mut fc = sample();
        fc.style_edge(0, Some(Arrow::Diamond), Some(Arrow::Block), None, None).unwrap();
        let mut doc = Document::new(Direction::TB);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        assert!(x.contains("startArrow=diamond"));
        assert!(x.contains("endArrow=block"));
    }

    #[test]
    fn svg_well_formed() {
        let s = to_svg(&sample());
        assert!(s.starts_with("<svg"));
        assert!(s.contains("marker id=\"arrow\""));
        assert!(s.contains("polygon"));
    }

    #[test]
    fn html_label_stripped_in_text_kept_in_drawio() {
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_node("a", "<b>Title</b><br>line two", Shape::Rectangle).unwrap();
        fc.set_node_html("a", Some(true)).unwrap();
        let m = to_mermaid(&fc);
        assert!(m.contains("Title line two"), "got: {m}");
        assert!(!m.contains("<b>"));
        let mut doc = Document::new(Direction::TB);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        assert!(x.contains("&lt;b&gt;Title&lt;/b&gt;"));
    }

    #[test]
    fn uml_class_and_self_loop_in_drawio() {
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_class_node("c", "User", vec![vec!["+ id: int".into()], vec!["+ login(): void".into()]]).unwrap();
        fc.add_edge("c", "c", Some("refresh".into()), LineStyle::Solid, true).unwrap();
        let mut doc = Document::new(Direction::TB);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        assert!(x.contains("&lt;b&gt;User&lt;/b&gt;"));
        assert!(x.contains("+ login(): void"));
        assert!(x.contains("loopEdgeStyle"));
        let s = to_svg(doc.chart_ref());
        assert!(s.contains("+ id: int"));
    }

    #[test]
    fn wave4_styles_layers_labels_in_drawio() {
        use crate::engine::{Style, Theme};
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_node("a", "A", Shape::Rectangle).unwrap();
        fc.add_node("b", "B", Shape::Rectangle).unwrap();
        // gradient + sketch + glass on a node.
        fc.style_node("a", Style {
            gradient: Some("#7EA6E0".into()),
            sketch: Some(true),
            glass: Some(true),
            ..Default::default()
        }).unwrap();
        // a hidden layer with a node on it.
        fc.add_layer("bg", "Background", false).unwrap();
        fc.set_node_layer("b", Some("bg".into())).unwrap();
        // an edge with a positioned, backed label.
        let i = fc.add_edge("a", "b", Some("go".into()), LineStyle::Solid, true).unwrap();
        fc.label_edge(i, Some(-0.4), Some(12.0), Some("#FFFFCC".into()), Some("#D6B656".into())).unwrap();
        // theme recolors nodes.
        fc.apply_theme(Theme::Green);

        let mut doc = Document::new(Direction::TB);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        assert!(x.contains("gradientColor=#7EA6E0"));
        assert!(x.contains("sketch=1"));
        assert!(x.contains("glass=1"));
        assert!(x.contains("id=\"bg\" value=\"Background\""));
        assert!(x.contains("visible=\"0\""));
        assert!(x.contains("parent=\"bg\""));
        assert!(x.contains("labelBackgroundColor=#FFFFCC"));
        assert!(x.contains("labelBorderColor=#D6B656"));
        // theme green fill applied to a plain node.
        assert!(x.contains("fillColor=#D5E8D4"));
    }

    #[test]
    fn drawio_emits_fixed_ports_and_waypoints() {
        let mut fc = sample();
        fc.route_edge(0, Some(vec![[300.0, 150.0], [300.0, 250.0]]), Some([1.0, 0.5]), Some([0.0, 0.5]), false)
            .unwrap();
        let mut doc = Document::new(Direction::TB);
        *doc.chart() = fc;
        let x = to_drawio(&doc);
        assert!(x.contains("exitX=1;exitY=0.5"));
        assert!(x.contains("entryX=0;entryY=0.5"));
        assert!(x.contains("<Array as=\"points\">"));
        assert!(x.contains("<mxPoint x=\"300\""));
    }
}
