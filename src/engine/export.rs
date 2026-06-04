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
                node.shape.mermaid_wrap(&node.label)
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
                    node.shape.mermaid_wrap(&node.label)
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
            dot_escape(&node.label),
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

    // Nodes — absolute coordinates at the root layer (stable, no reflow).
    for node in &fc.nodes {
        let b = l.get(&node.id);
        cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"1\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
            ident(&node.id),
            xml_escape(&node.label),
            drawio_node_style(node),
            b.x,
            b.y + header,
            b.w,
            b.h,
        ));
    }

    for (i, e) in fc.edges.iter().enumerate() {
        cells.push_str(&format!(
            "        <mxCell id=\"edge{i}\" value=\"{}\" style=\"{}\" edge=\"1\" parent=\"1\" \
             source=\"{}\" target=\"{}\">\n          <mxGeometry relative=\"1\" as=\"geometry\"/>\n        </mxCell>\n",
            xml_escape(e.label.as_deref().unwrap_or("")),
            drawio_edge_style(e),
            ident(&e.from),
            ident(&e.to),
        ));
    }

    format!(
        "  <diagram id=\"{}\" name=\"{}\">\n    \
         <mxGraphModel dx=\"{:.0}\" dy=\"{:.0}\" grid=\"1\" gridSize=\"10\" guides=\"1\" \
         tooltips=\"1\" connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" \
         math=\"0\" shadow=\"0\">\n      <root>\n        \
         <mxCell id=\"0\"/>\n        <mxCell id=\"1\" parent=\"0\"/>\n{cells}      </root>\n    \
         </mxGraphModel>\n  </diagram>\n",
        ident(name),
        xml_escape(name),
        (l.width + 40.0).max(800.0),
        (l.height + header + 40.0).max(600.0),
    )
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
}

fn drawio_edge_style(e: &Edge) -> String {
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

pub fn to_svg(fc: &Flowchart) -> String {
    let l = layout::compute(fc);
    let mut body = String::new();

    // Container backdrops, parents first (so children draw on top).
    for i in ordered_containers(fc) {
        let sg = &fc.subgraphs[i];
        if let Some(b) = container_abs_box(fc, &l, &sg.id) {
            let (stroke, dash, fill) = match sg.kind {
                ContainerKind::Group => ("#999", " stroke-dasharray=\"4 3\"", "none"),
                _ => ("#666", "", "#00000008"),
            };
            body.push_str(&format!(
                "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" \
                 fill=\"{fill}\" stroke=\"{stroke}\"{dash}/>\n  \
                 <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"12\" fill=\"#555\">{}</text>\n",
                b.x,
                b.y,
                b.w,
                b.h,
                b.x + 6.0,
                b.y + 16.0,
                xml_escape(&sg.label),
            ));
        }
    }

    for e in &fc.edges {
        let (a, b) = (l.get(&e.from), l.get(&e.to));
        let (x1, y1) = (a.x + a.w / 2.0, a.y + a.h / 2.0);
        let (x2, y2) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        let (sx, sy) = clip_to_box(x2, y2, a);
        let (tx, ty) = clip_to_box(x1, y1, b);
        let dash = match e.line {
            LineStyle::Dotted => " stroke-dasharray=\"5 4\"",
            _ => "",
        };
        let width = if e.line == LineStyle::Thick { 3.0 } else { 1.5 };
        let color = e.color.as_deref().unwrap_or("#333");
        let mut markers = String::new();
        if e.resolved_end() != super::Arrow::None {
            markers.push_str(" marker-end=\"url(#arrow)\"");
        }
        if e.resolved_start() != super::Arrow::None {
            markers.push_str(" marker-start=\"url(#arrow-start)\"");
        }
        body.push_str(&format!(
            "  <line x1=\"{sx:.1}\" y1=\"{sy:.1}\" x2=\"{tx:.1}\" y2=\"{ty:.1}\" \
             stroke=\"{color}\" stroke-width=\"{width}\"{dash}{markers}/>\n",
        ));
        if let Some(lbl) = &e.label {
            if !lbl.is_empty() {
                body.push_str(&format!(
                    "  <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"11\" \
                     fill=\"#333\" text-anchor=\"middle\">{}</text>\n",
                    (sx + tx) / 2.0,
                    (sy + ty) / 2.0 - 3.0,
                    xml_escape(lbl),
                ));
            }
        }
    }

    for node in &fc.nodes {
        let b = l.get(&node.id);
        if let Some(uri) = &node.image {
            body.push_str(&format!(
                "  <image href=\"{}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 preserveAspectRatio=\"xMidYMid meet\"/>\n",
                xml_escape(uri),
                b.x,
                b.y,
                b.w,
                b.h,
            ));
        } else {
            body.push_str(&svg_shape(node.shape, b, &node.style));
        }
        body.push_str(&svg_label(node, b));
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" \
         viewBox=\"0 0 {:.0} {:.0}\">\n  <defs>\n    \
         <marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" \
         orient=\"auto\" markerUnits=\"strokeWidth\">\n      \
         <path d=\"M0,0 L8,3 L0,6 z\" fill=\"#333\"/>\n    </marker>\n    \
         <marker id=\"arrow-start\" markerWidth=\"10\" markerHeight=\"10\" refX=\"0\" refY=\"3\" \
         orient=\"auto\" markerUnits=\"strokeWidth\">\n      \
         <path d=\"M8,0 L0,3 L8,6 z\" fill=\"#333\"/>\n    </marker>\n  </defs>\n  \
         <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n{body}</svg>\n",
        l.width, l.height, l.width, l.height,
    )
}

fn svg_label(node: &super::Node, b: Box) -> String {
    let s = &node.style;
    let (anchor, tx) = match s.align.as_deref() {
        Some("left") => ("start", b.x + 6.0),
        Some("right") => ("end", b.x + b.w - 6.0),
        _ => ("middle", b.x + b.w / 2.0),
    };
    let baseline = if node.image.is_some() {
        b.y + b.h + 12.0
    } else {
        b.y + b.h / 2.0
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
    format!(
        "  <text x=\"{tx:.1}\" y=\"{baseline:.1}\" font-family=\"{family}\" font-size=\"{size}\" \
         text-anchor=\"{anchor}\" dominant-baseline=\"middle\" fill=\"{}\"{extra}>{}</text>\n",
        s.text_color.as_deref().unwrap_or("#000"),
        xml_escape(&node.label),
    )
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

/// Clip the segment from `(fx,fy)` toward box `b`'s center to the box border.
fn clip_to_box(fx: f64, fy: f64, b: Box) -> (f64, f64) {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let dx = fx - cx;
    let dy = fy - cy;
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let hw = b.w / 2.0;
    let hh = b.h / 2.0;
    let scale = (hw / dx.abs()).min(hh / dy.abs());
    (cx + dx * scale, cy + dy * scale)
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
}
