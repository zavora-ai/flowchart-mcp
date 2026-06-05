//! Zero-dependency vector PDF export.
//!
//! Reuses [`layout::compute`] geometry and draws the diagram into a single-page
//! PDF using content-stream operators and the built-in Helvetica font (no font
//! embedding needed). PDF is a vector format, so this needs no rasterizer.

use std::fmt::Write as _;

use super::layout::{self, Box};
use super::{Flowchart, LineStyle, Shape};

/// Parse `#RRGGBB` (or `#RGB`) to PDF rgb in 0..1; default mid-grey.
fn rgb(hex: Option<&str>, default: (f64, f64, f64)) -> (f64, f64, f64) {
    let Some(h) = hex else { return default };
    let h = h.trim_start_matches('#');
    let full = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => h.to_string(),
        _ => return default,
    };
    let p = |i: usize| u8::from_str_radix(&full[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
    (p(0), p(2), p(4))
}

/// Escape text for a PDF literal string.
fn pdf_text(s: &str) -> String {
    s.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

/// Approximate Helvetica string width in points at the given font size.
fn text_width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * 0.5
}

/// Export the current flowchart as a single-page PDF (bytes).
pub fn to_pdf(fc: &Flowchart) -> Vec<u8> {
    let l = layout::compute(fc);
    let (w, h) = (l.width, l.height);
    // PDF y-axis points up; our layout y points down. Flip via `fy`.
    let fy = |y: f64| h - y;

    let mut c = String::new();
    // White background.
    let _ = writeln!(c, "1 1 1 rg 0 0 {w:.1} {h:.1} re f");

    // Edges first (under nodes).
    for e in &fc.edges {
        let a = l.get(&e.from);
        let b = l.get(&e.to);
        let (x1, y1) = clip(a, center(b));
        let (x2, y2) = clip(b, center(a));
        let (sr, sg, sb) = rgb(e.color.as_deref(), (0.27, 0.32, 0.37));
        let lw = if e.line == LineStyle::Thick { 2.5 } else { 1.2 };
        let _ = writeln!(c, "{sr:.3} {sg:.3} {sb:.3} RG {lw:.1} w");
        if e.line == LineStyle::Dotted {
            let _ = writeln!(c, "[3 3] 0 d");
        } else {
            let _ = writeln!(c, "[] 0 d");
        }
        let _ = writeln!(c, "{x1:.1} {fy1:.1} m {x2:.1} {fy2:.1} l S", fy1 = fy(y1), fy2 = fy(y2));
        if e.resolved_end() != super::Arrow::None {
            arrowhead(&mut c, x1, fy(y1), x2, fy(y2), (sr, sg, sb));
        }
        if let Some(lbl) = &e.label {
            if !lbl.is_empty() {
                let mx = (x1 + x2) / 2.0;
                let my = fy((y1 + y2) / 2.0) + 2.0;
                let _ = writeln!(
                    c,
                    "BT /F1 9 Tf 0.27 0.32 0.37 rg {mx:.1} {my:.1} Td ({}) Tj ET",
                    pdf_text(lbl)
                );
            }
        }
    }

    // Nodes.
    for node in &fc.nodes {
        let b = l.get(&node.id);
        let fill = rgb(node.style.fill.as_deref(), (0.92, 0.95, 0.98));
        let stroke = rgb(node.style.stroke.as_deref(), (0.29, 0.48, 0.67));
        let txt = rgb(node.style.text_color.as_deref(), (0.12, 0.16, 0.22));
        node_path(&mut c, node.shape, b, &fy, fill, stroke);

        // Centred single-line label (plain text; rich HTML is stripped).
        let label = crate::engine::export_plain_label(node);
        if !label.is_empty() {
            let size = node.style.font_size.unwrap_or(12.0);
            let tx = b.x + (b.w - text_width(&label, size)) / 2.0;
            let ty = fy(b.y + b.h / 2.0) - size / 3.0;
            let _ = writeln!(
                c,
                "BT /F1 {size:.0} Tf {tr:.3} {tg:.3} {tb:.3} rg {tx:.1} {ty:.1} Td ({}) Tj ET",
                pdf_text(&label),
                tr = txt.0,
                tg = txt.1,
                tb = txt.2,
            );
        }
    }

    assemble(&c, w, h)
}

fn center(b: Box) -> (f64, f64) {
    (b.x + b.w / 2.0, b.y + b.h / 2.0)
}

/// Clip a segment from box `b`'s centre toward `(fx,fy)` to the box border.
fn clip(b: Box, to: (f64, f64)) -> (f64, f64) {
    let (cx, cy) = center(b);
    let (dx, dy) = (to.0 - cx, to.1 - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let s = (b.w / 2.0 / dx.abs()).min(b.h / 2.0 / dy.abs());
    (cx + dx * s, cy + dy * s)
}

/// Draw a short filled triangle arrowhead pointing from (x1,y1) toward (x2,y2).
fn arrowhead(c: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, col: (f64, f64, f64)) {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / len, dy / len);
    let size = 8.0;
    let (bx, by) = (x2 - ux * size, y2 - uy * size);
    let (px, py) = (-uy, ux);
    let _ = writeln!(
        c,
        "{r:.3} {g:.3} {b:.3} rg {x2:.1} {y2:.1} m {:.1} {:.1} l {:.1} {:.1} l f",
        bx + px * size * 0.4,
        by + py * size * 0.4,
        bx - px * size * 0.4,
        by - py * size * 0.4,
        r = col.0,
        g = col.1,
        b = col.2,
    );
}

/// Emit the fill+stroke path for a node shape (y already flipped via `fy`).
fn node_path(c: &mut String, shape: Shape, b: Box, fy: &dyn Fn(f64) -> f64, fill: (f64, f64, f64), stroke: (f64, f64, f64)) {
    let _ = writeln!(c, "{:.3} {:.3} {:.3} rg {:.3} {:.3} {:.3} RG 1.2 w [] 0 d", fill.0, fill.1, fill.2, stroke.0, stroke.1, stroke.2);
    let (cx, _) = center(b);
    match shape {
        Shape::Diamond => {
            let _ = writeln!(
                c,
                "{cx:.1} {:.1} m {:.1} {:.1} l {cx:.1} {:.1} l {:.1} {:.1} l h B",
                fy(b.y),
                b.x + b.w,
                fy(b.y + b.h / 2.0),
                fy(b.y + b.h),
                b.x,
                fy(b.y + b.h / 2.0),
            );
        }
        Shape::Circle | Shape::DoubleCircle | Shape::Stadium => {
            // Approximate an ellipse/pill with a rounded outline via 4 Béziers.
            ellipse(c, b, fy);
        }
        // Everything else → rectangle (close enough for a print/preview PDF).
        _ => {
            let _ = writeln!(c, "{:.1} {:.1} {:.1} {:.1} re B", b.x, fy(b.y + b.h), b.w, b.h);
        }
    }
}

/// Approximate an ellipse inscribed in `b` using four cubic Béziers.
fn ellipse(c: &mut String, b: Box, fy: &dyn Fn(f64) -> f64) {
    let k = 0.5523;
    let cx = b.x + b.w / 2.0;
    let cy = fy(b.y + b.h / 2.0);
    let (rx, ry) = (b.w / 2.0, b.h / 2.0);
    let l = cx - rx;
    let r = cx + rx;
    let t = cy + ry;
    let bot = cy - ry;
    let kx = rx * k;
    let ky = ry * k;
    let _ = writeln!(c, "{l:.1} {cy:.1} m");
    let (a, b1) = (cy + ky, cx - kx);
    let _ = writeln!(c, "{l:.1} {a:.1} {b1:.1} {t:.1} {cx:.1} {t:.1} c");
    let (a, b1) = (cx + kx, cy + ky);
    let _ = writeln!(c, "{a:.1} {t:.1} {r:.1} {b1:.1} {r:.1} {cy:.1} c");
    let (a, b1) = (cy - ky, cx + kx);
    let _ = writeln!(c, "{r:.1} {a:.1} {b1:.1} {bot:.1} {cx:.1} {bot:.1} c");
    let (a, b1) = (cx - kx, cy - ky);
    let _ = writeln!(c, "{a:.1} {bot:.1} {l:.1} {b1:.1} {l:.1} {cy:.1} c h B");
}

/// Wrap the content stream in a minimal PDF document with an xref table.
fn assemble(content: &str, w: f64, h: f64) -> Vec<u8> {
    let objs: Vec<String> = vec![
        "<</Type/Catalog/Pages 2 0 R>>".to_string(),
        "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_string(),
        format!("<</Type/Page/Parent 2 0 R/MediaBox[0 0 {w:.0} {h:.0}]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>"),
        "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_string(),
        format!("<</Length {}>>\nstream\n{content}\nendstream", content.len()),
    ];

    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objs.len());
    for (i, body) in objs.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref_pos = out.len();
    let n = objs.len() + 1;
    let _ = write!(out, "xref\n0 {n}\n0000000000 65535 f \n");
    for off in &offsets {
        let _ = write!(out, "{off:010} 00000 n \n");
    }
    let _ = write!(out, "trailer\n<</Size {n}/Root 1 0 R>>\nstartxref\n{xref_pos}\n%%EOF\n");
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Direction, Shape};

    #[test]
    fn pdf_has_header_and_trailer() {
        let mut fc = Flowchart::new(Direction::TB);
        fc.add_node("a", "Start", Shape::Stadium).unwrap();
        fc.add_node("b", "Go?", Shape::Diamond).unwrap();
        fc.add_edge("a", "b", Some("yes".into()), LineStyle::Solid, true).unwrap();
        let bytes = to_pdf(&fc);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("/Type/Catalog"));
        assert!(s.contains("(Start) Tj"));
        assert!(s.contains("startxref"));
    }
}
