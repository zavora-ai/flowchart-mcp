//! MCP server with tool routing for flowchart authoring and export.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;

use crate::engine::{
    export, import, Arrow, ContainerKind, Direction, Document, EdgeRouting, LayoutKind, LineStyle,
    Shape,
};
use crate::error::{category, engine_error, unknown_handle};
use crate::store::{new_store, Shared};
use crate::types::inputs::{
    AddEdgeInput, AddNodeInput, AddPageInput, AddSubgraphInput, BuildDocumentInput, CreateInput,
    ExportInput, ExportPagesInput, HandleInput, ImportJsonInput, ImportMermaidInput,
    ListStencilsInput, MoveNodeInput, PageSpec, RemoveEdgeInput, RemoveNodeInput, RouteEdgeInput,
    SelectPageInput, SetDirectionInput, SetLayoutInput, SetNodeImageInput, SetNodeStencilInput,
    StyleEdgeInput, StyleNodeInput, UpdateEdgeInput, UpdateNodeInput,
};
use crate::types::responses::{error, success};

/// The Flowchart MCP server. Holds open documents in an in-memory handle store.
#[derive(Clone)]
pub struct FlowchartServer {
    store: Shared,
}

impl FlowchartServer {
    pub fn new() -> Self {
        Self { store: new_store() }
    }
}

impl Default for FlowchartServer {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_direction(s: &str) -> Result<Direction, String> {
    Direction::parse(s).ok_or_else(|| {
        error(category::INVALID_INPUT, format!("Unknown direction '{s}'"), "Use TB, BT, LR, or RL.")
    })
}

fn parse_shape(s: &str) -> Result<Shape, String> {
    Shape::parse(s).ok_or_else(|| {
        error(category::INVALID_INPUT, format!("Unknown shape '{s}'"), "See add_node for the list of shapes.")
    })
}

/// Parse an optional arrow string, mapping a parse failure to an error response.
fn opt_arrow(s: &Option<String>) -> Result<Option<Arrow>, String> {
    match s {
        None => Ok(None),
        Some(v) => Arrow::parse(v).map(Some).ok_or_else(|| {
            error(
                category::INVALID_INPUT,
                format!("Unknown arrow '{v}'"),
                "Use none, open, block, classic, diamond, oval, cross, er_one, er_many, er_zero_to_one, er_zero_to_many, or er_one_to_many.",
            )
        }),
    }
}

fn opt_routing(s: &Option<String>) -> Result<Option<EdgeRouting>, String> {
    match s {
        None => Ok(None),
        Some(v) => EdgeRouting::parse(v).map(Some).ok_or_else(|| {
            error(category::INVALID_INPUT, format!("Unknown routing '{v}'"), "Use orthogonal, straight, curved, or entity_relation.")
        }),
    }
}

/// Build a single page's `Flowchart` from a `PageSpec`. Returns the chart and
/// the resolved page name. Validates shapes, ids, edge endpoints, and lane
/// membership; errors are returned as ready-to-send response strings.
fn build_page_chart(
    spec: PageSpec,
    default_dir: Direction,
    page_index: usize,
) -> Result<(crate::engine::Flowchart, String), String> {
    use crate::engine::Flowchart;

    let dir = match spec.direction.as_deref() {
        None => default_dir,
        Some(s) => parse_direction(s)?,
    };
    let mut fc = Flowchart::new(dir);
    fc.title = spec.title;

    // Nodes
    for n in &spec.nodes {
        let shape = match n.shape.as_deref() {
            None => Shape::Rectangle,
            Some(s) => parse_shape(s)?,
        };
        let label = n.label.clone().unwrap_or_else(|| n.id.clone());
        fc.add_node(&n.id, &label, shape).map_err(engine_error)?;
        if let Some(img) = &n.image {
            let _ = fc.set_node_image(&n.id, Some(img.clone()));
        }
        if let Some(st) = &n.stencil {
            let _ = fc.set_node_stencil(&n.id, Some(st.clone()));
        }
    }
    // Styles (NodeSpec.style is flattened; clone per node since into_style consumes)
    for n in spec.nodes.iter() {
        let style = crate::engine::Style {
            fill: n.style.fill.clone(),
            stroke: n.style.stroke.clone(),
            text_color: n.style.text_color.clone(),
            stroke_width: n.style.stroke_width,
            font_family: n.style.font_family.clone(),
            font_size: n.style.font_size,
            bold: n.style.bold,
            italic: n.style.italic,
            align: n.style.align.clone(),
            opacity: n.style.opacity,
            rounded: n.style.rounded,
            shadow: n.style.shadow,
            dashed: n.style.dashed,
        };
        if !style.is_empty() {
            let _ = fc.style_node(&n.id, style);
        }
    }

    // Edges
    for e in &spec.edges {
        let line = match e.line.as_deref() {
            None => LineStyle::Solid,
            Some(s) => LineStyle::parse(s).ok_or_else(|| {
                error(category::INVALID_INPUT, format!("Unknown line style '{s}'"), "Use solid, dotted, or thick.")
            })?,
        };
        let start = opt_arrow(&e.start_arrow)?;
        let end = opt_arrow(&e.end_arrow)?;
        let routing = opt_routing(&e.routing)?;
        let idx = fc
            .add_edge(&e.from, &e.to, e.label.clone(), line, e.arrow.unwrap_or(true))
            .map_err(engine_error)?;
        if start.is_some() || end.is_some() || routing.is_some() || e.color.is_some() {
            let _ = fc.style_edge(idx, start, end, routing, e.color.clone());
        }
    }

    // Decision consistency: a decision (diamond) that branches (2+ outgoing
    // edges) must label every branch. This catches Yes/No and N-way splits
    // (e.g. full/half/empty) that would otherwise be ambiguous.
    {
        use std::collections::HashMap;
        let decision_ids: std::collections::HashSet<&str> = spec
            .nodes
            .iter()
            .filter(|n| n.shape.as_deref().map(Shape::parse) == Some(Some(Shape::Diamond)))
            .map(|n| n.id.as_str())
            .collect();
        let mut out_counts: HashMap<&str, (usize, usize)> = HashMap::new(); // id -> (total, labeled)
        for e in &spec.edges {
            if decision_ids.contains(e.from.as_str()) {
                let entry = out_counts.entry(e.from.as_str()).or_insert((0, 0));
                entry.0 += 1;
                if e.label.as_deref().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                    entry.1 += 1;
                }
            }
        }
        for (id, (total, labeled)) in out_counts {
            if total >= 2 && labeled < total {
                return Err(error(
                    category::INVALID_INPUT,
                    format!(
                        "Decision '{id}' has {total} outgoing branches but only {labeled} are labeled"
                    ),
                    "Label every branch of a decision (e.g. Yes/No, or full/half/empty). Each outgoing edge from a diamond needs a `label`.",
                ));
            }
        }
    }

    // Swimlanes: one container per lane label, members grouped by node.lane.
    if !spec.lanes.is_empty() {
        use std::collections::HashSet;
        let lane_set: HashSet<&str> = spec.lanes.iter().map(|s| s.as_str()).collect();
        // Validate every node's lane reference up front.
        for n in &spec.nodes {
            match &n.lane {
                Some(l) if lane_set.contains(l.as_str()) => {}
                Some(l) => {
                    return Err(error(
                        category::INVALID_INPUT,
                        format!("Node '{}' references unknown lane '{}'", n.id, l),
                        "Add the lane to the page's `lanes` list or fix the node's `lane`.",
                    ))
                }
                None => {
                    return Err(error(
                        category::INVALID_INPUT,
                        format!("Node '{}' has no lane but the page declares lanes", n.id),
                        "Give every node a `lane` matching one of the page's `lanes`.",
                    ))
                }
            }
        }
        for (li, lane) in spec.lanes.iter().enumerate() {
            let members: Vec<String> = spec
                .nodes
                .iter()
                .filter(|n| n.lane.as_deref() == Some(lane.as_str()))
                .map(|n| n.id.clone())
                .collect();
            fc.add_subgraph(
                &format!("lane{li}"),
                lane,
                members,
                ContainerKind::Swimlane,
                None,
                None,
            )
            .map_err(engine_error)?;
        }
    }

    let name = spec.name.unwrap_or_else(|| format!("Page-{}", page_index + 1));
    Ok((fc, name))
}

#[tool_router(server_handler)]
impl FlowchartServer {
    #[tool(
        description = "Create a new flowchart document. direction: TB (default), BT, LR, RL. \
        Optionally pass a template id (see list_templates) to pre-populate it. Returns a handle."
    )]
    async fn create_flowchart(&self, Parameters(input): Parameters<CreateInput>) -> String {
        let direction = match input.direction.as_deref() {
            None => Direction::TB,
            Some(s) => match parse_direction(s) {
                Ok(d) => d,
                Err(e) => return e,
            },
        };
        let mut doc = match input.template.as_deref() {
            None => Document::new(direction),
            Some(t) => match crate::templates::build(t) {
                Some(mut d) => {
                    if input.direction.is_some() {
                        d.chart().set_direction(direction);
                    }
                    d
                }
                None => {
                    return error(category::INVALID_INPUT, format!("Unknown template '{t}'"), "Call list_templates for valid ids.")
                }
            },
        };
        doc.chart().title = input.title;
        if let Some(lk) = input.layout.as_deref() {
            match LayoutKind::parse(lk) {
                Some(k) => doc.chart().set_layout(k),
                None => return error(category::INVALID_INPUT, format!("Unknown layout '{lk}'"), "Use layered, tree, or mind_map."),
            }
        }
        let chart = doc.chart_ref();
        let (n, e) = (chart.nodes.len(), chart.edges.len());
        let handle = self.store.write().await.insert(doc);
        success("Created flowchart", json!({ "handle": handle, "node_count": n, "edge_count": e }))
    }

    #[tool(description = "List available flowchart templates for create_flowchart.")]
    async fn list_templates(&self) -> String {
        success("Flowchart templates", json!({ "templates": crate::templates::catalog() }))
    }

    #[tool(description = "Close a flowchart and free its memory.")]
    async fn close_flowchart(&self, Parameters(input): Parameters<HandleInput>) -> String {
        if self.store.write().await.remove(&input.handle) {
            success("Closed flowchart", json!({ "handle": input.handle }))
        } else {
            unknown_handle(&input.handle)
        }
    }

    #[tool(
        description = "Describe the current page: direction, title, page list, and full \
        node/edge/subgraph listing (edges include their index for remove_edge/style_edge)."
    )]
    async fn describe_flowchart(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let pages: Vec<_> = doc.pages.iter().map(|p| p.name.clone()).collect();
        let current = doc.current;
        let fc = doc.chart_ref();
        let nodes: Vec<_> = fc
            .nodes
            .iter()
            .map(|n| json!({ "id": n.id, "label": n.label, "shape": n.shape.label(), "image": n.image, "stencil": n.stencil, "pos": n.pos, "size": n.size, "compartments": n.compartments }))
            .collect();
        let edges: Vec<_> = fc
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| json!({ "index": i, "from": e.from, "to": e.to, "label": e.label }))
            .collect();
        let subgraphs: Vec<_> = fc
            .subgraphs
            .iter()
            .map(|s| json!({ "id": s.id, "label": s.label, "kind": s.kind.label(), "parent": s.parent, "members": s.members }))
            .collect();
        success(
            "Flowchart described",
            json!({
                "direction": fc.direction.as_mermaid(),
                "layout": fc.layout.label(),
                "title": fc.title,
                "pages": pages,
                "current_page": current,
                "node_count": fc.nodes.len(),
                "edge_count": fc.edges.len(),
                "nodes": nodes,
                "edges": edges,
                "subgraphs": subgraphs,
            }),
        )
    }

    #[tool(
        description = "Add a node to the current page. shape: rectangle (default), round_rect, \
        stadium, subroutine, cylinder, circle, double_circle, diamond, hexagon, parallelogram, \
        parallelogram_alt, trapezoid, trapezoid_alt, note, card, document, uml_class. Optional \
        image (path/URI), stencil, html (rich-text label), compartments (uml_class sections), and \
        rich style (fill/stroke/text_color/stroke_width/font_family/font_size/bold/italic/align/\
        opacity/rounded/shadow/dashed)."
    )]
    async fn add_node(&self, Parameters(input): Parameters<AddNodeInput>) -> String {
        let shape = match input.shape.as_deref() {
            None => Shape::Rectangle,
            Some(s) => match parse_shape(s) {
                Ok(sh) => sh,
                Err(e) => return e,
            },
        };
        let label = input.label.clone().unwrap_or_else(|| input.id.clone());
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let fc = doc.chart();
        if let Err(e) = fc.add_node(&input.id, &label, shape) {
            return engine_error(e);
        }
        if let Some(img) = input.image {
            let _ = fc.set_node_image(&input.id, Some(img));
        }
        if let Some(st) = input.stencil {
            let _ = fc.set_node_stencil(&input.id, Some(st));
        }
        if input.html.is_some() {
            let _ = fc.set_node_html(&input.id, input.html);
        }
        if let Some(comp) = input.compartments {
            let _ = fc.set_compartments(&input.id, comp);
        }
        let style = input.style.into_style();
        if !style.is_empty() {
            let _ = fc.style_node(&input.id, style);
        }
        success("Added node", json!({ "id": input.id, "node_count": fc.nodes.len() }))
    }

    #[tool(description = "Update a node's label, shape, and/or html (rich-text) flag.")]
    async fn update_node(&self, Parameters(input): Parameters<UpdateNodeInput>) -> String {
        let shape = match input.shape.as_deref() {
            None => None,
            Some(s) => match parse_shape(s) {
                Ok(sh) => Some(sh),
                Err(e) => return e,
            },
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let fc = doc.chart();
        if input.html.is_some() {
            let _ = fc.set_node_html(&input.id, input.html);
        }
        match fc.update_node(&input.id, input.label.as_deref(), shape) {
            Ok(()) => success("Updated node", json!({ "id": input.id })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set a node's visual style. Any of: fill, stroke, text_color (hex), \
        stroke_width, font_family, font_size, bold, italic, align (left/center/right), opacity \
        (0–100), rounded, shadow, dashed. Only provided fields change."
    )]
    async fn style_node(&self, Parameters(input): Parameters<StyleNodeInput>) -> String {
        let style = input.style.into_style();
        if style.is_empty() {
            return error(category::INVALID_INPUT, "No style fields provided", "Provide at least one style field.");
        }
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().style_node(&input.id, style) {
            Ok(()) => success("Styled node", json!({ "id": input.id })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Set or clear a node's image (path or URI). Omit `image` to clear it.")]
    async fn set_node_image(&self, Parameters(input): Parameters<SetNodeImageInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().set_node_image(&input.id, input.image) {
            Ok(()) => success("Set node image", json!({ "id": input.id })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "List built-in draw.io stencils (AWS/Azure/GCP/network/Kubernetes/UML/BPMN/\
        mockup) for set_node_stencil. Filter by `category` and/or free-text `query`. Any raw \
        `mxgraph.<lib>.<name>` token also works even if not listed."
    )]
    async fn list_stencils(&self, Parameters(input): Parameters<ListStencilsInput>) -> String {
        success(
            "Stencils",
            crate::stencils::list(input.category.as_deref(), input.query.as_deref()),
        )
    }

    #[tool(
        description = "Set or clear a node's draw.io stencil (a key from list_stencils, e.g. \
        'aws.ec2', or a raw 'mxgraph.<lib>.<name>' token). Renders faithfully in the drawio export; \
        other formats show a labeled placeholder. Omit `stencil` to clear it."
    )]
    async fn set_node_stencil(&self, Parameters(input): Parameters<SetNodeStencilInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().set_node_stencil(&input.id, input.stencil) {
            Ok(()) => success("Set node stencil", json!({ "id": input.id })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Remove a node and every edge touching it.")]
    async fn remove_node(&self, Parameters(input): Parameters<RemoveNodeInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let fc = doc.chart();
        match fc.remove_node(&input.id) {
            Ok(removed) => success(
                "Removed node",
                json!({ "id": input.id, "edges_removed": removed, "node_count": fc.nodes.len() }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Manually place/size a node, overriding auto-layout. Provide `x`+`y` for the \
        top-left and/or `w`+`h` for the size (canvas pixels); `clear: true` returns it to \
        auto-layout. Other nodes still auto-lay-out around it; the canvas grows to fit."
    )]
    async fn move_node(&self, Parameters(input): Parameters<MoveNodeInput>) -> String {
        let pos = match (input.x, input.y) {
            (Some(x), Some(y)) => Some([x, y]),
            (None, None) => None,
            _ => {
                return error(category::INVALID_INPUT, "x and y must be provided together", "Pass both x and y, or neither.")
            }
        };
        let size = match (input.w, input.h) {
            (Some(w), Some(h)) => Some([w, h]),
            (None, None) => None,
            _ => {
                return error(category::INVALID_INPUT, "w and h must be provided together", "Pass both w and h, or neither.")
            }
        };
        if !input.clear && pos.is_none() && size.is_none() {
            return error(category::INVALID_INPUT, "Nothing to set", "Provide x+y, w+h, or clear:true.");
        }
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().move_node(&input.id, pos, size, input.clear) {
            Ok(()) => success("Moved node", json!({ "id": input.id, "cleared": input.clear })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Add a directed edge between two existing nodes. line: solid (default), \
        dotted, thick. arrow: arrowhead at target (default true). Optional end_arrow/start_arrow \
        (none/open/block/classic/diamond/oval/cross/er_one/er_many/...), routing \
        (orthogonal/straight/curved/entity_relation), and color hex. Returns the edge index."
    )]
    async fn add_edge(&self, Parameters(input): Parameters<AddEdgeInput>) -> String {
        let line = match input.line.as_deref() {
            None => LineStyle::Solid,
            Some(s) => match LineStyle::parse(s) {
                Some(l) => l,
                None => return error(category::INVALID_INPUT, format!("Unknown line style '{s}'"), "Use solid, dotted, or thick."),
            },
        };
        let start = match opt_arrow(&input.start_arrow) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let end = match opt_arrow(&input.end_arrow) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let routing = match opt_routing(&input.routing) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let fc = doc.chart();
        match fc.add_edge(&input.from, &input.to, input.label, line, input.arrow.unwrap_or(true)) {
            Ok(idx) => {
                if start.is_some() || end.is_some() || routing.is_some() || input.color.is_some() {
                    let _ = fc.style_edge(idx, start, end, routing, input.color);
                }
                if input.exit.is_some() || input.entry.is_some() || input.waypoints.is_some() {
                    let _ = fc.route_edge(idx, input.waypoints, input.exit, input.entry, false);
                }
                success("Added edge", json!({ "index": idx, "edge_count": fc.edges.len() }))
            }
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Style an existing edge: start_arrow/end_arrow, routing, and color. \
        Only provided fields change."
    )]
    async fn style_edge(&self, Parameters(input): Parameters<StyleEdgeInput>) -> String {
        let start = match opt_arrow(&input.start_arrow) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let end = match opt_arrow(&input.end_arrow) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let routing = match opt_routing(&input.routing) {
            Ok(r) => r,
            Err(e) => return e,
        };
        if start.is_none() && end.is_none() && routing.is_none() && input.color.is_none() {
            return error(category::INVALID_INPUT, "No edge style fields provided", "Provide start_arrow, end_arrow, routing, or color.");
        }
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().style_edge(input.index, start, end, routing, input.color) {
            Ok(()) => success("Styled edge", json!({ "index": input.index })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Update an existing edge's label and/or line style (solid/dotted/thick). Only provided fields change.")]
    async fn update_edge(&self, Parameters(input): Parameters<UpdateEdgeInput>) -> String {
        let line = match input.line.as_deref() {
            None => None,
            Some(s) => match LineStyle::parse(s) {
                Some(l) => Some(l),
                None => return error(category::INVALID_INPUT, format!("Unknown line style '{s}'"), "Use solid, dotted, or thick."),
            },
        };
        if input.label.is_none() && line.is_none() {
            return error(category::INVALID_INPUT, "Nothing to update", "Provide label and/or line.");
        }
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().update_edge(input.index, input.label, line) {
            Ok(()) => success("Updated edge", json!({ "index": input.index })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Manually route an edge: `waypoints` ([[x,y],...] canvas pixels) it passes \
        through, and/or fixed `exit`/`entry` ports ([x,y] in 0..1 on the source/target). \
        `clear: true` resets to automatic routing."
    )]
    async fn route_edge(&self, Parameters(input): Parameters<RouteEdgeInput>) -> String {
        if !input.clear && input.waypoints.is_none() && input.exit.is_none() && input.entry.is_none() {
            return error(category::INVALID_INPUT, "Nothing to set", "Provide waypoints, exit, entry, or clear:true.");
        }
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().route_edge(input.index, input.waypoints, input.exit, input.entry, input.clear) {
            Ok(()) => success("Routed edge", json!({ "index": input.index, "cleared": input.clear })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Remove the edge at the given index (see describe_flowchart).")]
    async fn remove_edge(&self, Parameters(input): Parameters<RemoveEdgeInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let fc = doc.chart();
        match fc.remove_edge(input.index) {
            Ok(()) => success("Removed edge", json!({ "edge_count": fc.edges.len() })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Set the current page's flow direction: TB, BT, LR, or RL.")]
    async fn set_direction(&self, Parameters(input): Parameters<SetDirectionInput>) -> String {
        let dir = match parse_direction(&input.direction) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        doc.chart().set_direction(dir);
        success("Set direction", json!({ "direction": dir.as_mermaid() }))
    }

    #[tool(
        description = "Set the current page's auto-layout: 'layered' (default; flowcharts/swimlanes), \
        'tree' (hierarchy/org chart), or 'mind_map' (central root radiating both ways)."
    )]
    async fn set_layout(&self, Parameters(input): Parameters<SetLayoutInput>) -> String {
        let Some(kind) = LayoutKind::parse(&input.layout) else {
            return error(category::INVALID_INPUT, format!("Unknown layout '{}'", input.layout), "Use layered, tree, or mind_map.");
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        doc.chart().set_layout(kind);
        success("Set layout", json!({ "layout": kind.label() }))
    }

    #[tool(
        description = "Group nodes into a container on the current page. kind: group (default, \
        dashed), container (titled box), swimlane, or pool. For lanes, pass parent=<pool id>; \
        pools accept orientation horizontal (default) or vertical. members may be empty for a pool."
    )]
    async fn add_subgraph(&self, Parameters(input): Parameters<AddSubgraphInput>) -> String {
        let kind = match input.kind.as_deref() {
            None => ContainerKind::Group,
            Some(s) => match ContainerKind::parse(s) {
                Some(k) => k,
                None => return error(category::INVALID_INPUT, format!("Unknown container kind '{s}'"), "Use group, container, swimlane, or pool."),
            },
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.chart().add_subgraph(&input.id, &input.label, input.members, kind, input.orientation, input.parent) {
            Ok(()) => success("Added container", json!({ "id": input.id, "kind": kind.label() })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Add a new page to the document and select it. Returns the page index.")]
    async fn add_page(&self, Parameters(input): Parameters<AddPageInput>) -> String {
        let dir = match input.direction.as_deref() {
            None => Direction::TB,
            Some(s) => match parse_direction(s) {
                Ok(d) => d,
                Err(e) => return e,
            },
        };
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let idx = doc.add_page(input.name, dir);
        success("Added page", json!({ "page_index": idx, "page_count": doc.pages.len() }))
    }

    #[tool(description = "Select the active page by 0-based index. Subsequent edits target it.")]
    async fn select_page(&self, Parameters(input): Parameters<SelectPageInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match doc.select_page(input.index) {
            Ok(()) => success("Selected page", json!({ "current_page": input.index })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Export the document. format: 'drawio' (diagrams.net mxGraph XML, all pages), \
        'mermaid', 'dot' (Graphviz), 'svg', 'pdf' (vector, current page, requires output_path), or \
        'json'. Mermaid/dot/svg/pdf render the current page. With output_path the content is written \
        to disk; otherwise returned inline under data.content."
    )]
    async fn export_flowchart(&self, Parameters(input): Parameters<ExportInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        // PDF is binary (vector); it must be written to a file.
        if matches!(input.format.to_ascii_lowercase().as_str(), "pdf") {
            let Some(path) = &input.output_path else {
                return error(category::INVALID_INPUT, "PDF export requires output_path", "PDF is binary; pass output_path (e.g. \"diagram.pdf\").");
            };
            let bytes = crate::engine::pdf::to_pdf(doc.chart_ref());
            return match std::fs::write(path, &bytes) {
                Ok(()) => success(
                    format!("Exported pdf to {path}"),
                    json!({ "format": "pdf", "output_path": path, "bytes": bytes.len() }),
                ),
                Err(e) => error(category::IO_ERROR, e.to_string(), "Check the output path and permissions."),
            };
        }
        let content = match input.format.to_ascii_lowercase().as_str() {
            "drawio" | "xml" => export::to_drawio(doc),
            "mermaid" | "mmd" => export::to_mermaid(doc.chart_ref()),
            "dot" | "graphviz" => export::to_dot(doc.chart_ref()),
            "svg" => export::to_svg(doc.chart_ref()),
            "json" => match serde_json::to_string_pretty(doc) {
                Ok(s) => s,
                Err(e) => return error(category::INVALID_INPUT, e.to_string(), "Could not serialize the document."),
            },
            other => {
                return error(category::INVALID_INPUT, format!("Unknown format '{other}'"), "Use drawio, mermaid, dot, svg, pdf, or json.")
            }
        };
        match &input.output_path {
            Some(path) => match std::fs::write(path, &content) {
                Ok(()) => success(
                    format!("Exported {} to {}", input.format, path),
                    json!({ "format": input.format, "output_path": path, "bytes": content.len() }),
                ),
                Err(e) => error(category::IO_ERROR, e.to_string(), "Check the output path and permissions."),
            },
            None => success(
                format!("Exported {}", input.format),
                json!({ "format": input.format, "content": content }),
            ),
        }
    }

    #[tool(
        description = "Import a Mermaid flowchart (provide `source` text or `file_path`) into a new \
        document. Returns a handle. Supports nodes, all shapes, edges (solid/dotted/thick, labels) \
        and subgraphs."
    )]
    async fn import_mermaid(&self, Parameters(input): Parameters<ImportMermaidInput>) -> String {
        let src = match (input.source, input.file_path) {
            (Some(s), _) => s,
            (None, Some(path)) => match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => return error(category::IO_ERROR, e.to_string(), "Check the file path."),
            },
            (None, None) => {
                return error(category::INVALID_INPUT, "Provide either 'source' or 'file_path'", "Pass the Mermaid text inline or a path to a .mmd file.")
            }
        };
        match import::from_mermaid(&src) {
            Ok(fc) => {
                let (n, e) = (fc.nodes.len(), fc.edges.len());
                let mut doc = Document::new(fc.direction);
                *doc.chart() = fc;
                let handle = self.store.write().await.insert(doc);
                success("Imported Mermaid flowchart", json!({ "handle": handle, "node_count": n, "edge_count": e }))
            }
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Import a full document from JSON (the exact shape produced by \
        export_flowchart format=json). Provide `json` inline or a `file_path`. Returns a handle. \
        Round-trips multi-page documents, styles, containers, images, and stencils."
    )]
    async fn import_json(&self, Parameters(input): Parameters<ImportJsonInput>) -> String {
        let src = match (input.json, input.file_path) {
            (Some(s), _) => s,
            (None, Some(path)) => match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => return error(category::IO_ERROR, e.to_string(), "Check the file path."),
            },
            (None, None) => {
                return error(category::INVALID_INPUT, "Provide either 'json' or 'file_path'", "Pass the document JSON inline or a path to a .json file.")
            }
        };
        let doc: Document = match serde_json::from_str(&src) {
            Ok(d) => d,
            Err(e) => return error(category::PARSE_ERROR, e.to_string(), "Provide JSON matching the export_flowchart format=json shape."),
        };
        let pages = doc.pages.len();
        let (n, e) = {
            let c = doc.chart_ref();
            (c.nodes.len(), c.edges.len())
        };
        let handle = self.store.write().await.insert(doc);
        success("Imported JSON document", json!({ "handle": handle, "page_count": pages, "node_count": n, "edge_count": e }))
    }

    #[tool(
        description = "Build a complete multi-page document in one call. Each page declares nodes \
        (id/label/shape/lane), edges (from/to/label/arrows/routing/color), and optional swimlane \
        `lanes` (stacked bands; every node must name a lane when lanes are present). Geometry is \
        auto-laid-out. Returns a handle. This replaces hundreds of incremental calls for large \
        diagrams."
    )]
    async fn build_document(&self, Parameters(input): Parameters<BuildDocumentInput>) -> String {
        let default_dir = match input.direction.as_deref() {
            None => Direction::TB,
            Some(s) => match parse_direction(s) {
                Ok(d) => d,
                Err(e) => return e,
            },
        };
        if input.pages.is_empty() {
            return error(category::INVALID_INPUT, "No pages provided", "Provide at least one page in `pages`.");
        }

        // Build every page chart first so a failure leaves nothing half-created.
        let mut built: Vec<(crate::engine::Flowchart, String)> = Vec::with_capacity(input.pages.len());
        for (i, page) in input.pages.into_iter().enumerate() {
            match build_page_chart(page, default_dir, i) {
                Ok(pair) => built.push(pair),
                Err(e) => return e, // already a structured error response
            }
        }

        // Assemble the document: first page replaces the initial empty page.
        let mut doc = Document::new(default_dir);
        let total_pages = built.len();
        let mut total_nodes = 0usize;
        let mut total_edges = 0usize;
        for (idx, (fc, name)) in built.into_iter().enumerate() {
            total_nodes += fc.nodes.len();
            total_edges += fc.edges.len();
            if idx == 0 {
                doc.pages[0].name = name;
                *doc.chart() = fc;
            } else {
                doc.pages.push(crate::engine::Page { name, chart: fc });
            }
        }
        doc.current = 0;
        let handle = self.store.write().await.insert(doc);
        success(
            "Built document",
            json!({ "handle": handle, "page_count": total_pages, "node_count": total_nodes, "edge_count": total_edges }),
        )
    }

    #[tool(
        description = "Export each page of the document to its own file in `output_dir` (created \
        if missing). format: drawio, mermaid, dot, svg, or json. name_pattern tokens: {index} \
        (1-based, 2-digit), {name} (page name), {ext}. Default '{index}-{name}.{ext}'. Returns the \
        list of written files."
    )]
    async fn export_pages(&self, Parameters(input): Parameters<ExportPagesInput>) -> String {
        let fmt = input.format.to_ascii_lowercase();
        if !matches!(fmt.as_str(), "drawio" | "xml" | "mermaid" | "mmd" | "dot" | "graphviz" | "svg" | "json") {
            return error(category::INVALID_INPUT, format!("Unknown format '{}'", input.format), "Use drawio, mermaid, dot, svg, or json.");
        }
        let ext = export::format_ext(&fmt);
        let pattern = input.name_pattern.as_deref().unwrap_or("{index}-{name}.{ext}");

        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        if let Err(e) = std::fs::create_dir_all(&input.output_dir) {
            return error(category::IO_ERROR, e.to_string(), "Check the output_dir path and permissions.");
        }

        let mut written: Vec<String> = Vec::new();
        for (i, page) in doc.pages.iter().enumerate() {
            let content = match fmt.as_str() {
                "drawio" | "xml" => export::to_drawio_page(&page.name, &page.chart),
                "mermaid" | "mmd" => export::to_mermaid(&page.chart),
                "dot" | "graphviz" => export::to_dot(&page.chart),
                "svg" => export::to_svg(&page.chart),
                "json" => match serde_json::to_string_pretty(&page.chart) {
                    Ok(s) => s,
                    Err(e) => return error(category::INVALID_INPUT, e.to_string(), "Could not serialize the page."),
                },
                _ => unreachable!(),
            };
            let safe_name = sanitize_filename(&page.name);
            let fname = pattern
                .replace("{index}", &format!("{:02}", i + 1))
                .replace("{name}", &safe_name)
                .replace("{ext}", ext);
            let path = std::path::Path::new(&input.output_dir).join(&fname);
            if let Err(e) = std::fs::write(&path, &content) {
                return error(category::IO_ERROR, e.to_string(), "Check the output_dir path and permissions.");
            }
            written.push(path.to_string_lossy().to_string());
        }
        success("Exported pages", json!({ "format": fmt, "count": written.len(), "files": written }))
    }

    #[tool(
        description = "Validate the document against flowchart correctness properties and return \
        a report. Checks: decision branches labeled (G1), non-decision steps have <=1 exit (G2), \
        all nodes reachable from a start and able to reach an end (G3), no node overlaps (L1), \
        decisions sized for their text (L2). Returns { valid, violation_count, violations[] } \
        across all pages; does not modify the document."
    )]
    async fn validate_flowchart(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut violations = Vec::new();
        for (i, page) in doc.pages.iter().enumerate() {
            crate::engine::validate::check_chart(&page.chart, i, &mut violations);
        }
        let items: Vec<_> = violations
            .iter()
            .map(|v| json!({ "page": v.page, "property": v.property, "message": v.message }))
            .collect();
        success(
            if violations.is_empty() { "All correctness properties hold" } else { "Validation found issues" },
            json!({ "valid": violations.is_empty(), "violation_count": violations.len(), "violations": items }),
        )
    }
}

/// Sanitize a page name into a filesystem-safe token.
fn sanitize_filename(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_whitespace() => '-',
            c => c,
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}
