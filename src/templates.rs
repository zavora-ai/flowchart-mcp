//! Built-in starter charts for `create_flowchart`'s `template` option. Each
//! returns a full [`Document`] (single page unless noted).

use serde_json::{json, Value};

use crate::engine::{
    Arrow, ContainerKind, Direction, Document, EdgeRouting, Flowchart, LineStyle, Shape,
};

/// Build a template by id, or `None` if unknown.
pub fn build(id: &str) -> Option<Document> {
    let fc = match id {
        "basic" => basic(),
        "decision" => decision(),
        "approval" => approval(),
        "etl" => etl(),
        "swimlane" => return Some(wrap(swimlane())),
        "org_chart" => org_chart(),
        "mind_map" => mind_map(),
        "uml_class" => uml_class(),
        "erd" => erd(),
        "bpmn" => bpmn(),
        "state_machine" => state_machine(),
        _ => return None,
    };
    Some(wrap(fc))
}

fn wrap(fc: Flowchart) -> Document {
    let mut doc = Document::new(fc.direction);
    *doc.chart() = fc;
    doc
}

/// Catalog for `list_templates`.
pub fn catalog() -> Value {
    json!([
        { "id": "basic", "description": "Start → Process → End (top-down)" },
        { "id": "decision", "description": "Start → decision diamond with yes/no branches" },
        { "id": "approval", "description": "Request → review decision → approve/reject paths" },
        { "id": "etl", "description": "Extract → Transform → Load pipeline (left-right)" },
        { "id": "swimlane", "description": "Two-lane pool with a handoff between lanes" },
        { "id": "org_chart", "description": "Org hierarchy (CEO → VPs → teams), tree layout" },
        { "id": "mind_map", "description": "Central idea with radiating branches, mind-map layout" },
        { "id": "uml_class", "description": "UML class boxes with compartments + inheritance/association arrows" },
        { "id": "erd", "description": "Entity-relationship diagram with crow's-foot arrows" },
        { "id": "bpmn", "description": "BPMN-style lanes with start/task/gateway/end" },
        { "id": "state_machine", "description": "State machine: start → states → end, with a self-loop transition" }
    ])
}

fn basic() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.add_node("start", "Start", Shape::Stadium).unwrap();
    fc.add_node("process", "Process", Shape::Rectangle).unwrap();
    fc.add_node("end", "End", Shape::Stadium).unwrap();
    fc.add_edge("start", "process", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("process", "end", None, LineStyle::Solid, true).unwrap();
    fc
}

fn decision() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.add_node("start", "Start", Shape::Stadium).unwrap();
    fc.add_node("check", "Condition?", Shape::Diamond).unwrap();
    fc.add_node("yes", "Do A", Shape::Rectangle).unwrap();
    fc.add_node("no", "Do B", Shape::Rectangle).unwrap();
    fc.add_node("end", "End", Shape::Stadium).unwrap();
    fc.add_edge("start", "check", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("check", "yes", Some("yes".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("check", "no", Some("no".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("yes", "end", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("no", "end", None, LineStyle::Solid, true).unwrap();
    fc
}

fn approval() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.add_node("req", "Submit Request", Shape::Stadium).unwrap();
    fc.add_node("review", "Review", Shape::Diamond).unwrap();
    fc.add_node("approve", "Approve", Shape::Rectangle).unwrap();
    fc.add_node("reject", "Reject", Shape::Rectangle).unwrap();
    fc.add_node("notify", "Notify Requester", Shape::Parallelogram).unwrap();
    fc.add_edge("req", "review", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("review", "approve", Some("ok".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("review", "reject", Some("no".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("approve", "notify", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("reject", "notify", None, LineStyle::Solid, true).unwrap();
    fc
}

fn etl() -> Flowchart {
    let mut fc = Flowchart::new(Direction::LR);
    fc.add_node("src", "Source", Shape::Cylinder).unwrap();
    fc.add_node("extract", "Extract", Shape::Rectangle).unwrap();
    fc.add_node("transform", "Transform", Shape::Rectangle).unwrap();
    fc.add_node("load", "Load", Shape::Rectangle).unwrap();
    fc.add_node("dw", "Warehouse", Shape::Cylinder).unwrap();
    fc.add_edge("src", "extract", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("extract", "transform", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("transform", "load", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("load", "dw", None, LineStyle::Solid, true).unwrap();
    fc
}

fn swimlane() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.add_node("a1", "Submit", Shape::Stadium).unwrap();
    fc.add_node("a2", "Prepare", Shape::Rectangle).unwrap();
    fc.add_node("b1", "Review", Shape::Rectangle).unwrap();
    fc.add_node("b2", "Approve", Shape::Stadium).unwrap();
    fc.add_edge("a1", "a2", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("a2", "b1", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("b1", "b2", None, LineStyle::Solid, true).unwrap();
    fc.add_subgraph("pool", "Process", vec![], ContainerKind::Pool, None, None).unwrap();
    fc.add_subgraph("customer", "Customer", vec!["a1".into(), "a2".into()], ContainerKind::Swimlane, None, Some("pool".into())).unwrap();
    fc.add_subgraph("staff", "Staff", vec!["b1".into(), "b2".into()], ContainerKind::Swimlane, None, Some("pool".into())).unwrap();
    fc
}

fn org_chart() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.set_layout(crate::engine::LayoutKind::Tree);
    fc.add_node("ceo", "CEO", Shape::Rectangle).unwrap();
    fc.add_node("cto", "CTO", Shape::Rectangle).unwrap();
    fc.add_node("cfo", "CFO", Shape::Rectangle).unwrap();
    fc.add_node("eng", "Engineering", Shape::Rectangle).unwrap();
    fc.add_node("ops", "Operations", Shape::Rectangle).unwrap();
    for (f, t) in [("ceo", "cto"), ("ceo", "cfo"), ("cto", "eng"), ("cfo", "ops")] {
        let i = fc.add_edge(f, t, None, LineStyle::Solid, false).unwrap();
        fc.style_edge(i, None, None, Some(EdgeRouting::Orthogonal), None).unwrap();
    }
    fc
}

fn mind_map() -> Flowchart {
    let mut fc = Flowchart::new(Direction::LR);
    fc.set_layout(crate::engine::LayoutKind::MindMap);
    fc.add_node("root", "Central Idea", Shape::Circle).unwrap();
    fc.style_node("root", style_fill("#FFE6CC")).unwrap();
    for (id, label) in [("b1", "Topic A"), ("b2", "Topic B"), ("b3", "Topic C"), ("b4", "Topic D")] {
        fc.add_node(id, label, Shape::RoundRect).unwrap();
        let i = fc.add_edge("root", id, None, LineStyle::Solid, false).unwrap();
        fc.style_edge(i, None, Some(Arrow::None), Some(EdgeRouting::Curved), None).unwrap();
    }
    // A couple of sub-branches to exercise subtree packing.
    for (p, id, label) in [("b1", "a1", "Detail 1"), ("b1", "a2", "Detail 2"), ("b3", "c1", "Detail 3")] {
        fc.add_node(id, label, Shape::RoundRect).unwrap();
        let i = fc.add_edge(p, id, None, LineStyle::Solid, false).unwrap();
        fc.style_edge(i, None, Some(Arrow::None), Some(EdgeRouting::Curved), None).unwrap();
    }
    fc
}

fn uml_class() -> Flowchart {
    let mut fc = Flowchart::new(Direction::TB);
    fc.set_layout(crate::engine::LayoutKind::Tree);
    fc.add_class_node(
        "animal",
        "Animal",
        vec![
            vec!["+ name: String".into(), "+ age: int".into()],
            vec!["+ eat(): void".into(), "+ move(): void".into()],
        ],
    )
    .unwrap();
    fc.add_class_node(
        "dog",
        "Dog",
        vec![vec!["+ breed: String".into()], vec!["+ bark(): void".into()]],
    )
    .unwrap();
    fc.add_class_node(
        "owner",
        "Owner",
        vec![vec!["+ name: String".into()], vec!["+ adopt(d: Dog): void".into()]],
    )
    .unwrap();
    // Inheritance: Dog → Animal (hollow triangle ~ block, unfilled).
    let i = fc.add_edge("animal", "dog", None, LineStyle::Solid, false).unwrap();
    fc.style_edge(i, Some(Arrow::Block), Some(Arrow::None), Some(EdgeRouting::Orthogonal), None).unwrap();
    // Association: Owner → Dog.
    let j = fc.add_edge("owner", "dog", Some("owns".into()), LineStyle::Solid, false).unwrap();
    fc.style_edge(j, None, Some(Arrow::Open), Some(EdgeRouting::Orthogonal), None).unwrap();
    fc
}

fn state_machine() -> Flowchart {
    let mut fc = Flowchart::new(Direction::LR);
    fc.add_node("start", "●", Shape::Circle).unwrap();
    fc.style_node("start", style_fill("#333333")).unwrap();
    fc.add_node("idle", "Idle", Shape::RoundRect).unwrap();
    fc.add_node("running", "Running", Shape::RoundRect).unwrap();
    fc.add_node("done", "Done", Shape::DoubleCircle).unwrap();
    fc.add_edge("start", "idle", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("idle", "running", Some("start".into()), LineStyle::Solid, true).unwrap();
    // Self-loop transition on Running.
    fc.add_edge("running", "running", Some("tick".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("running", "idle", Some("pause".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("running", "done", Some("finish".into()), LineStyle::Solid, true).unwrap();
    fc
}

fn erd() -> Flowchart {
    let mut fc = Flowchart::new(Direction::LR);
    fc.add_node("customer", "Customer", Shape::Rectangle).unwrap();
    fc.add_node("order", "Order", Shape::Rectangle).unwrap();
    fc.add_node("product", "Product", Shape::Rectangle).unwrap();
    for n in ["customer", "order", "product"] {
        fc.style_node(n, style_fill("#DAE8FC")).unwrap();
    }
    // Customer (1) ──< Order (many).
    let i = fc.add_edge("customer", "order", Some("places".into()), LineStyle::Solid, false).unwrap();
    fc.style_edge(i, Some(Arrow::ErOne), Some(Arrow::ErMany), Some(EdgeRouting::EntityRelation), None).unwrap();
    // Order (many) >──< Product (many).
    let j = fc.add_edge("order", "product", Some("contains".into()), LineStyle::Solid, false).unwrap();
    fc.style_edge(j, Some(Arrow::ErMany), Some(Arrow::ErMany), Some(EdgeRouting::EntityRelation), None).unwrap();
    fc
}

fn bpmn() -> Flowchart {
    let mut fc = Flowchart::new(Direction::LR);
    fc.add_node("start", "Start", Shape::Circle).unwrap();
    fc.add_node("task1", "Receive Order", Shape::RoundRect).unwrap();
    fc.add_node("gw", "In stock?", Shape::Diamond).unwrap();
    fc.add_node("ship", "Ship", Shape::RoundRect).unwrap();
    fc.add_node("back", "Backorder", Shape::RoundRect).unwrap();
    fc.add_node("end", "End", Shape::DoubleCircle).unwrap();
    fc.add_edge("start", "task1", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("task1", "gw", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("gw", "ship", Some("yes".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("gw", "back", Some("no".into()), LineStyle::Solid, true).unwrap();
    fc.add_edge("ship", "end", None, LineStyle::Solid, true).unwrap();
    fc.add_edge("back", "end", None, LineStyle::Solid, true).unwrap();
    fc.add_subgraph("pool", "Fulfillment", vec![], ContainerKind::Pool, None, None).unwrap();
    fc.add_subgraph(
        "lane",
        "Warehouse",
        vec!["start".into(), "task1".into(), "gw".into(), "ship".into(), "back".into(), "end".into()],
        ContainerKind::Swimlane,
        None,
        Some("pool".into()),
    )
    .unwrap();
    fc
}

fn style_fill(hex: &str) -> crate::engine::Style {
    crate::engine::Style {
        fill: Some(hex.to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_build() {
        for t in [
            "basic", "decision", "approval", "etl", "swimlane", "org_chart", "mind_map",
            "uml_class", "erd", "bpmn", "state_machine",
        ] {
            let mut doc = build(t).unwrap_or_else(|| panic!("template {t} missing"));
            assert!(!doc.chart().nodes.is_empty(), "{t} has no nodes");
        }
        assert!(build("nope").is_none());
    }

    #[test]
    fn uml_class_has_compartments_and_tree_layout() {
        let mut doc = build("uml_class").unwrap();
        let fc = doc.chart();
        assert_eq!(fc.layout, crate::engine::LayoutKind::Tree);
        let animal = fc.nodes.iter().find(|n| n.id == "animal").unwrap();
        assert_eq!(animal.shape, Shape::UmlClass);
        assert_eq!(animal.compartments.len(), 2);
    }

    #[test]
    fn state_machine_has_self_loop() {
        let mut doc = build("state_machine").unwrap();
        assert!(doc.chart().edges.iter().any(|e| e.from == e.to));
    }

    #[test]
    fn erd_uses_crowsfoot() {
        let mut doc = build("erd").unwrap();
        assert!(doc.chart().edges.iter().any(|e| e.end_arrow == Some(Arrow::ErMany)));
    }
}
