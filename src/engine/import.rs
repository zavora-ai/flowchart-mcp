//! Mermaid `flowchart` / `graph` parser. Handles the common subset: direction
//! header, node shape definitions, edges (solid/dotted/thick, with/without
//! labels and arrows), and `subgraph ... end` blocks.

use super::{ContainerKind, Direction, Flowchart, LineStyle, Shape};
use crate::error::FlowError;

/// Parse Mermaid flowchart text into a [`Flowchart`].
pub fn from_mermaid(src: &str) -> Result<Flowchart, FlowError> {
    let mut lines = src.lines().map(strip_comment).filter(|l| !l.trim().is_empty());

    let header = lines
        .next()
        .ok_or_else(|| FlowError::Parse("empty input".into()))?;
    let direction = parse_header(header.trim())?;
    let mut fc = Flowchart::new(direction);

    let mut subgraph_stack: Vec<(String, Vec<String>)> = Vec::new();

    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();

        if let Some(rest) = lower.strip_prefix("subgraph") {
            let title = line[line.len() - rest.len()..].trim();
            let (id, label) = parse_subgraph_header(title);
            subgraph_stack.push((id.unwrap_or(label.clone()), Vec::new()));
            // Stash label by reusing add later; store via marker map.
            subgraph_labels_push(&mut fc, &subgraph_stack.last().unwrap().0, &label);
            continue;
        }
        if lower == "end" {
            if let Some((id, members)) = subgraph_stack.pop() {
                let label = subgraph_label_take(&mut fc, &id).unwrap_or_else(|| id.clone());
                let _ = fc.add_subgraph(&id, &label, members, ContainerKind::Group, None, None);
            }
            continue;
        }
        if lower.starts_with("direction ") {
            if let Some(d) = Direction::parse(line[10..].trim()) {
                fc.set_direction(d);
            }
            continue;
        }
        if lower.starts_with("style ") || lower.starts_with("classdef") || lower.starts_with("class ")
        {
            continue; // styling directives ignored on import
        }

        // Edge or node statement.
        let touched = parse_statement(&mut fc, line)?;
        if let Some((_, members)) = subgraph_stack.last_mut() {
            for id in touched {
                if !members.contains(&id) {
                    members.push(id);
                }
            }
        }
    }

    if fc.nodes.is_empty() {
        return Err(FlowError::Parse("no nodes found".into()));
    }
    Ok(fc)
}

fn strip_comment(line: &str) -> &str {
    match line.find("%%") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn parse_header(h: &str) -> Result<Direction, FlowError> {
    let rest = h
        .strip_prefix("flowchart")
        .or_else(|| h.strip_prefix("graph"))
        .ok_or_else(|| FlowError::Parse("expected 'flowchart' or 'graph' header".into()))?;
    let dir = rest.trim();
    if dir.is_empty() {
        Ok(Direction::TB)
    } else {
        Direction::parse(dir).ok_or_else(|| FlowError::Parse(format!("unknown direction '{dir}'")))
    }
}

fn parse_subgraph_header(s: &str) -> (Option<String>, String) {
    // Forms: `subgraph one`, `subgraph id [Title]`, `subgraph "Title"`.
    if let Some(open) = s.find('[') {
        let id = s[..open].trim().to_string();
        let label = s[open + 1..]
            .trim_end_matches(']')
            .trim()
            .trim_matches('"')
            .to_string();
        return (Some(id), label);
    }
    let cleaned = s.trim().trim_matches('"').to_string();
    (None, cleaned)
}

// Subgraph labels are stashed on the flowchart via a temporary edge-free
// mechanism: we keep them in a side Vec keyed by id using the title field of a
// throwaway. Simpler: use a thread-local-free approach with a small map stored
// in the Flowchart's subgraphs as we go is awkward, so use a static-free helper.
use std::cell::RefCell;
thread_local! {
    static SG_LABELS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}
fn subgraph_labels_push(_fc: &mut Flowchart, id: &str, label: &str) {
    SG_LABELS.with(|m| m.borrow_mut().push((id.to_string(), label.to_string())));
}
fn subgraph_label_take(_fc: &mut Flowchart, id: &str) -> Option<String> {
    SG_LABELS.with(|m| {
        let mut v = m.borrow_mut();
        if let Some(pos) = v.iter().position(|(k, _)| k == id) {
            Some(v.remove(pos).1)
        } else {
            None
        }
    })
}

/// Parse a single non-block statement. Returns the node ids it referenced.
fn parse_statement(fc: &mut Flowchart, line: &str) -> Result<Vec<String>, FlowError> {
    if let Some((left, conn, right)) = split_edge(line) {
        let (from_id, from_shape, from_label) = parse_node_token(&left)?;
        let (to_id, to_shape, to_label) = parse_node_token(&right)?;
        ensure_node(fc, &from_id, from_shape, from_label)?;
        ensure_node(fc, &to_id, to_shape, to_label)?;
        let (line_style, arrow, edge_label) = conn;
        fc.add_edge(&from_id, &to_id, edge_label, line_style, arrow)?;
        Ok(vec![from_id, to_id])
    } else {
        let (id, shape, label) = parse_node_token(line)?;
        ensure_node(fc, &id, shape, label)?;
        Ok(vec![id])
    }
}

fn ensure_node(
    fc: &mut Flowchart,
    id: &str,
    shape: Option<Shape>,
    label: Option<String>,
) -> Result<(), FlowError> {
    if fc.has_node(id) {
        if shape.is_some() || label.is_some() {
            fc.update_node(id, label.as_deref(), shape)?;
        }
        Ok(())
    } else {
        fc.add_node(
            id,
            label.as_deref().unwrap_or(id),
            shape.unwrap_or(Shape::Rectangle),
        )
    }
}

/// Connector descriptor: (line style, has arrow, optional label).
type Conn = (LineStyle, bool, Option<String>);

/// Split a statement into `(left, connector, right)` if it contains an edge.
fn split_edge(line: &str) -> Option<(String, Conn, String)> {
    // Operators ordered so longer/thicker matches win.
    // Returns byte range of the operator and its descriptor (without inline label).
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &line[i..];
        // Detect a run of '-', '.', '=' that forms a connector, possibly with
        // a pipe label `|...|` or inline `-- text -->`.
        if rest.starts_with("--")
            || rest.starts_with("-.")
            || rest.starts_with("==")
            || rest.starts_with("-->")
            || rest.starts_with("-.->")
        {
            if let Some((conn, op_len)) = match_connector(rest) {
                let left = line[..i].trim().to_string();
                let after = &rest[op_len..];
                // Optional pipe label right after the operator.
                let (label, right) = take_pipe_label(after);
                let conn = (conn.0, conn.1, label.or(conn.2));
                let right = right.trim().to_string();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, conn, right));
                }
            }
        }
        i += 1;
    }
    None
}

/// Match a connector at the start of `s`. Returns (descriptor, consumed bytes),
/// where the descriptor's label is an inline label embedded in the operator
/// (e.g. `-- text -->` or `-. text .->` or `== text ==>`).
fn match_connector(s: &str) -> Option<(Conn, usize)> {
    // Inline-label forms first.
    // -- label --> / -- label ---
    if let Some(rest) = s.strip_prefix("--") {
        if !rest.starts_with('-') && !rest.starts_with('>') {
            // could be `-- label -->` or `-- label ---`
            if let Some(end) = find_inline_end(rest, "--") {
                let label = rest[..end].trim().to_string();
                let tail = &rest[end..];
                let (arrow, op_extra) = arrow_after(tail, "--");
                let consumed = 2 + end + op_extra;
                return Some(((LineStyle::Solid, arrow, Some(label)), consumed));
            }
        }
    }
    if let Some(rest) = s.strip_prefix("-.") {
        // `-. label .-> ` or `-.->`
        if let Some(end) = rest.find(".-") {
            let label = rest[..end].trim().to_string();
            let tail = &rest[end..]; // starts with ".-"
            let (arrow, op_extra) = if tail.starts_with(".->") {
                (true, 3)
            } else {
                (false, 2)
            };
            let consumed = 2 + end + op_extra;
            let lbl = if label.is_empty() { None } else { Some(label) };
            return Some(((LineStyle::Dotted, arrow, lbl), consumed));
        }
    }
    if let Some(rest) = s.strip_prefix("==") {
        if !rest.starts_with('=') && !rest.starts_with('>') {
            if let Some(end) = find_inline_end(rest, "==") {
                let label = rest[..end].trim().to_string();
                let tail = &rest[end..];
                let (arrow, op_extra) = arrow_after(tail, "==");
                let consumed = 2 + end + op_extra;
                return Some(((LineStyle::Thick, arrow, Some(label)), consumed));
            }
        }
    }

    // Plain connectors (no inline label), longest first.
    for (op, style, arrow) in [
        ("-.->", LineStyle::Dotted, true),
        ("-.-", LineStyle::Dotted, false),
        ("==>", LineStyle::Thick, true),
        ("===", LineStyle::Thick, false),
        ("-->", LineStyle::Solid, true),
        ("---", LineStyle::Solid, false),
    ] {
        if s.starts_with(op) {
            return Some(((style, arrow, None), op.len()));
        }
    }
    None
}

/// Find where an inline label ends, i.e. the next occurrence of the closing
/// dashes/equals run `marker`.
fn find_inline_end(s: &str, marker: &str) -> Option<usize> {
    s.find(marker)
}

/// Given the tail starting at the closing marker run, determine if an arrow
/// follows and how many bytes the closing operator consumes.
fn arrow_after(tail: &str, marker: &str) -> (bool, usize) {
    // tail begins with marker (e.g. "--" or "=="); may be followed by '>'
    let after = &tail[marker.len()..];
    if after.starts_with('>') {
        (true, marker.len() + 1)
    } else {
        (false, marker.len())
    }
}

/// Take a leading `|label|` from `s`, returning (label, remainder).
fn take_pipe_label(s: &str) -> (Option<String>, &str) {
    let t = s.trim_start();
    if let Some(rest) = t.strip_prefix('|') {
        if let Some(end) = rest.find('|') {
            let label = rest[..end].trim().to_string();
            return (Some(label), &rest[end + 1..]);
        }
    }
    (None, s)
}

/// Parse a node token like `A[Label]`, `A(Round)`, `B{Decide}`, or bare `A`.
fn parse_node_token(tok: &str) -> Result<(String, Option<Shape>, Option<String>), FlowError> {
    let tok = tok.trim();
    if tok.is_empty() {
        return Err(FlowError::Parse("empty node token".into()));
    }
    // Find the first shape-opening delimiter.
    let open = tok.find(['[', '(', '{']);
    let Some(open) = open else {
        return Ok((tok.to_string(), None, None));
    };
    let id = tok[..open].trim().to_string();
    if id.is_empty() {
        return Err(FlowError::Parse(format!("node token missing id: '{tok}'")));
    }
    let body = &tok[open..];
    let (shape, label) = parse_shape_body(body)
        .ok_or_else(|| FlowError::Parse(format!("unrecognized node shape: '{body}'")))?;
    Ok((id, Some(shape), Some(label)))
}

/// Parse the delimiter-wrapped body, longest delimiters first.
fn parse_shape_body(b: &str) -> Option<(Shape, String)> {
    let pairs: &[(&str, &str, Shape)] = &[
        ("(((", ")))", Shape::DoubleCircle),
        ("((", "))", Shape::Circle),
        ("([", "])", Shape::Stadium),
        ("[[", "]]", Shape::Subroutine),
        ("[(", ")]", Shape::Cylinder),
        ("{{", "}}", Shape::Hexagon),
        ("[/", "/]", Shape::Parallelogram),
        ("[\\", "\\]", Shape::ParallelogramAlt),
        ("[/", "\\]", Shape::Trapezoid),
        ("[\\", "/]", Shape::TrapezoidAlt),
        ("(", ")", Shape::RoundRect),
        ("{", "}", Shape::Diamond),
        ("[", "]", Shape::Rectangle),
    ];
    for (o, c, shape) in pairs {
        if b.starts_with(o) && b.ends_with(c) && b.len() >= o.len() + c.len() {
            let inner = &b[o.len()..b.len() - c.len()];
            return Some((*shape, unquote(inner)));
        }
    }
    None
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(t)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direction_and_nodes() {
        let fc = from_mermaid("flowchart LR\n  A[Start] --> B{Go?}\n  B -->|yes| C(Done)").unwrap();
        assert_eq!(fc.direction, Direction::LR);
        assert_eq!(fc.nodes.len(), 3);
        assert_eq!(fc.edges.len(), 2);
        let b = fc.nodes.iter().find(|n| n.id == "B").unwrap();
        assert_eq!(b.shape, Shape::Diamond);
        let e = &fc.edges[1];
        assert_eq!(e.label.as_deref(), Some("yes"));
    }

    #[test]
    fn parses_dotted_and_thick() {
        let fc = from_mermaid("graph TD\n A-.->B\n B==>C").unwrap();
        assert_eq!(fc.edges[0].line, LineStyle::Dotted);
        assert_eq!(fc.edges[1].line, LineStyle::Thick);
    }

    #[test]
    fn parses_inline_labels() {
        let fc = from_mermaid("flowchart TD\n A -- next --> B\n B -. maybe .-> C").unwrap();
        assert_eq!(fc.edges[0].label.as_deref(), Some("next"));
        assert_eq!(fc.edges[1].label.as_deref(), Some("maybe"));
        assert_eq!(fc.edges[1].line, LineStyle::Dotted);
    }

    #[test]
    fn parses_subgraph() {
        let src = "flowchart TB\n subgraph one [Group One]\n  A --> B\n end\n B --> C";
        let fc = from_mermaid(src).unwrap();
        assert_eq!(fc.subgraphs.len(), 1);
        assert_eq!(fc.subgraphs[0].label, "Group One");
        assert!(fc.subgraphs[0].members.contains(&"A".to_string()));
    }

    #[test]
    fn rejects_garbage() {
        assert!(from_mermaid("not a diagram").is_err());
    }
}
