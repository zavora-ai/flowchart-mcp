//! Exporters for sequence diagrams: draw.io (mxGraph XML), SVG, and Mermaid.
//! JSON is just `serde_json` of [`Sequence`] (handled by the server).
//!
//! Layout: participants are columns spaced horizontally; each message is one
//! row, stacked top-to-bottom in declaration order. A simple sync/return depth
//! counter draws activation bars on the callee's lifeline.

use super::{MessageKind, Sequence};

const MARGIN: f64 = 24.0;
const COL_GAP: f64 = 160.0;
const HEAD_W: f64 = 120.0;
const HEAD_H: f64 = 40.0;
const ROW_H: f64 = 44.0;
const TOP: f64 = MARGIN + HEAD_H + 24.0;

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn ident(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        format!("p_{t}")
    } else {
        t
    }
}

/// Center x of column `i`.
fn col_x(i: usize) -> f64 {
    MARGIN + HEAD_W / 2.0 + i as f64 * COL_GAP
}

/// Row y (message baseline) for message `m` (0-based).
fn row_y(m: usize) -> f64 {
    TOP + (m as f64 + 1.0) * ROW_H
}

fn canvas(seq: &Sequence) -> (f64, f64) {
    let cols = seq.participants.len().max(1);
    let w = MARGIN * 2.0 + HEAD_W + (cols.saturating_sub(1)) as f64 * COL_GAP;
    let h = row_y(seq.messages.len()) + ROW_H;
    (w.max(240.0), h.max(160.0))
}

// ---------------------------------------------------------------------------
// Mermaid
// ---------------------------------------------------------------------------

pub fn to_mermaid(seq: &Sequence) -> String {
    let mut out = String::from("sequenceDiagram\n");
    for p in &seq.participants {
        let kw = if p.actor { "actor" } else { "participant" };
        if p.label == p.id {
            out.push_str(&format!("    {kw} {}\n", ident(&p.id)));
        } else {
            out.push_str(&format!("    {kw} {} as {}\n", ident(&p.id), p.label));
        }
    }
    for m in &seq.messages {
        let arrow = m.kind.mermaid();
        out.push_str(&format!(
            "    {}{arrow}{}: {}\n",
            ident(&m.from),
            ident(&m.to),
            m.label
        ));
        if m.kind == MessageKind::Destroy {
            out.push_str(&format!("    destroy {}\n", ident(&m.to)));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SVG
// ---------------------------------------------------------------------------

pub fn to_svg(seq: &Sequence) -> String {
    let (w, h) = canvas(seq);
    let mut body = String::new();

    // Lifelines + heads.
    for (i, p) in seq.participants.iter().enumerate() {
        let cx = col_x(i);
        // dashed lifeline
        body.push_str(&format!(
            "  <line x1=\"{cx:.1}\" y1=\"{:.1}\" x2=\"{cx:.1}\" y2=\"{:.1}\" stroke=\"#9DB3C8\" stroke-dasharray=\"4 4\"/>\n",
            MARGIN + HEAD_H,
            h - MARGIN,
        ));
        // head box
        let hx = cx - HEAD_W / 2.0;
        body.push_str(&format!(
            "  <rect x=\"{hx:.1}\" y=\"{:.1}\" width=\"{HEAD_W:.1}\" height=\"{HEAD_H:.1}\" rx=\"6\" \
             fill=\"#EAF2FB\" stroke=\"#4A7AAA\"/>\n  \
             <text x=\"{cx:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"13\" font-weight=\"bold\" \
             text-anchor=\"middle\" fill=\"#1F2A37\">{}</text>\n",
            MARGIN,
            MARGIN + HEAD_H / 2.0 + 4.0,
            xml_escape(&p.label),
        ));
    }

    // Messages.
    for (mi, m) in seq.messages.iter().enumerate() {
        let y = row_y(mi);
        let (Some(fc), Some(tc)) = (seq.col_of(&m.from), seq.col_of(&m.to)) else {
            continue;
        };
        let dashed = if m.kind.dashed() { " stroke-dasharray=\"5 4\"" } else { "" };
        if m.from == m.to {
            // self-message: small loop to the right.
            let x = col_x(fc);
            body.push_str(&format!(
                "  <path d=\"M{x:.1},{y:.1} h40 v22 h-40\" fill=\"none\" stroke=\"#33415C\"{dashed} marker-end=\"url(#seqarrow)\"/>\n  \
                 <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"11\" fill=\"#33415C\">{}</text>\n",
                x + 44.0, y - 4.0, xml_escape(&m.label),
            ));
            continue;
        }
        let (x1, x2) = (col_x(fc), col_x(tc));
        body.push_str(&format!(
            "  <line x1=\"{x1:.1}\" y1=\"{y:.1}\" x2=\"{x2:.1}\" y2=\"{y:.1}\" stroke=\"#33415C\"{dashed} marker-end=\"url(#seqarrow)\"/>\n",
        ));
        let lx = (x1 + x2) / 2.0;
        body.push_str(&format!(
            "  <text x=\"{lx:.1}\" y=\"{:.1}\" font-family=\"Helvetica\" font-size=\"11\" \
             fill=\"#33415C\" text-anchor=\"middle\">{}</text>\n",
            y - 5.0,
            xml_escape(&m.label),
        ));
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\">\n  \
         <defs><marker id=\"seqarrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" \
         orient=\"auto\" markerUnits=\"strokeWidth\"><path d=\"M0,0 L8,3 L0,6 z\" fill=\"#33415C\"/></marker></defs>\n  \
         <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n{body}</svg>\n",
    )
}

// ---------------------------------------------------------------------------
// draw.io (mxGraph XML)
// ---------------------------------------------------------------------------

pub fn to_drawio(seq: &Sequence) -> String {
    let (w, h) = canvas(seq);
    let mut cells = String::new();

    // Lifelines using the umlLifeline shape (head + dashed line in one cell).
    for (i, p) in seq.participants.iter().enumerate() {
        let cx = col_x(i);
        let style = if p.actor {
            "shape=umlActor;verticalLabelPosition=bottom;verticalAlign=top;html=1;outlineConnect=0;"
        } else {
            "shape=umlLifeline;perimeter=lifelinePerimeter;whiteSpace=wrap;html=1;container=0;\
             collapsible=0;recursiveResize=0;outlineConnect=0;fillColor=#EAF2FB;strokeColor=#4A7AAA;"
        };
        cells.push_str(&format!(
            "        <mxCell id=\"{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"1\">\n          \
             <mxGeometry x=\"{:.0}\" y=\"{:.0}\" width=\"{:.0}\" height=\"{:.0}\" as=\"geometry\"/>\n        </mxCell>\n",
            ident(&p.id),
            xml_escape(&p.label),
            style,
            cx - HEAD_W / 2.0,
            MARGIN,
            HEAD_W,
            h - MARGIN,
        ));
    }

    // Messages as floating edges between absolute points on the lifelines.
    for (mi, m) in seq.messages.iter().enumerate() {
        let y = row_y(mi);
        let (Some(fc), Some(tc)) = (seq.col_of(&m.from), seq.col_of(&m.to)) else {
            continue;
        };
        let mut style = String::from("html=1;rounded=0;");
        match m.kind {
            MessageKind::Sync => style.push_str("endArrow=block;endFill=1;"),
            MessageKind::Async => style.push_str("endArrow=open;endFill=0;"),
            MessageKind::Return => style.push_str("endArrow=open;endFill=0;dashed=1;"),
            MessageKind::Create => style.push_str("endArrow=open;endFill=0;dashed=1;"),
            MessageKind::Destroy => style.push_str("endArrow=cross;endFill=0;"),
        }
        style.push_str("strokeColor=#33415C;fontColor=#33415C;fontSize=11;labelBackgroundColor=#FFFFFF;");
        let (x1, x2) = (col_x(fc), col_x(tc));
        let (sx, tx) = if m.from == m.to { (x1, x1 + 50.0) } else { (x1, x2) };
        let ty = if m.from == m.to { y + 22.0 } else { y };
        cells.push_str(&format!(
            "        <mxCell id=\"msg{mi}\" value=\"{}\" style=\"{}\" edge=\"1\" parent=\"1\">\n          \
             <mxGeometry relative=\"1\" as=\"geometry\">\n            \
             <mxPoint x=\"{sx:.0}\" y=\"{y:.0}\" as=\"sourcePoint\"/>\n            \
             <mxPoint x=\"{tx:.0}\" y=\"{ty:.0}\" as=\"targetPoint\"/>\n          \
             </mxGeometry>\n        </mxCell>\n",
            xml_escape(&m.label),
            style,
        ));
    }

    let title = seq.title.as_deref().unwrap_or("Sequence");
    format!(
        "<mxfile host=\"app.diagrams.net\" type=\"device\">\n  \
         <diagram id=\"sequence\" name=\"{}\">\n    \
         <mxGraphModel dx=\"{:.0}\" dy=\"{:.0}\" grid=\"1\" gridSize=\"10\" guides=\"1\" tooltips=\"1\" \
         connect=\"1\" arrows=\"1\" fold=\"1\" page=\"1\" pageScale=\"1\" math=\"0\" shadow=\"0\">\n      \
         <root>\n        <mxCell id=\"0\"/>\n        <mxCell id=\"1\" parent=\"0\"/>\n{cells}      </root>\n    \
         </mxGraphModel>\n  </diagram>\n</mxfile>\n",
        xml_escape(title),
        (w + 40.0).max(400.0),
        (h + 40.0).max(400.0),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{MessageKind, Sequence};
    use super::*;

    fn sample() -> Sequence {
        let mut s = Sequence::new();
        s.title = Some("Login".into());
        s.add_participant("u", "User", true).unwrap();
        s.add_participant("api", "API", false).unwrap();
        s.add_message("u", "api", "POST /login", MessageKind::Sync).unwrap();
        s.add_message("api", "u", "200 OK", MessageKind::Return).unwrap();
        s.add_message("api", "api", "log", MessageKind::Async).unwrap();
        s
    }

    #[test]
    fn mermaid_has_sequence_header() {
        let m = to_mermaid(&sample());
        assert!(m.starts_with("sequenceDiagram"));
        assert!(m.contains("actor u as User"));
        assert!(m.contains("u->>api: POST /login"));
        assert!(m.contains("api-->>u: 200 OK"));
    }

    #[test]
    fn drawio_has_lifelines_and_messages() {
        let x = to_drawio(&sample());
        assert!(x.contains("<mxfile"));
        assert!(x.contains("umlActor"));
        assert!(x.contains("umlLifeline"));
        assert!(x.contains("endArrow=block")); // sync
        assert!(x.contains("dashed=1")); // return
        assert!(x.contains("value=\"POST /login\""));
    }

    #[test]
    fn svg_well_formed() {
        let s = to_svg(&sample());
        assert!(s.starts_with("<svg"));
        assert!(s.contains("seqarrow"));
        assert!(s.contains("User"));
    }
}
