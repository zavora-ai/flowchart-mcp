//! MCP server with tool routing for flowchart authoring and export.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;

use crate::engine::{
    export, import, Arrow, ContainerKind, Direction, Document, EdgeRouting, LineStyle, Shape,
};
use crate::error::{category, engine_error, unknown_handle};
use crate::store::{new_store, Shared};
use crate::types::inputs::{
    AddEdgeInput, AddNodeInput, AddPageInput, AddSubgraphInput, CreateInput, ExportInput,
    HandleInput, ImportMermaidInput, ListStencilsInput, RemoveEdgeInput, RemoveNodeInput,
    SelectPageInput, SetDirectionInput, SetNodeImageInput, SetNodeStencilInput, StyleEdgeInput,
    StyleNodeInput, UpdateNodeInput,
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
            .map(|n| json!({ "id": n.id, "label": n.label, "shape": n.shape.label(), "image": n.image, "stencil": n.stencil }))
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
        parallelogram_alt, trapezoid, trapezoid_alt, note, card, document. Optional image (path/URI) \
        and rich style (fill/stroke/text_color/stroke_width/font_family/font_size/bold/italic/align/\
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
        let style = input.style.into_style();
        if !style.is_empty() {
            let _ = fc.style_node(&input.id, style);
        }
        success("Added node", json!({ "id": input.id, "node_count": fc.nodes.len() }))
    }

    #[tool(description = "Update a node's label and/or shape.")]
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
        match doc.chart().update_node(&input.id, input.label.as_deref(), shape) {
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
        'mermaid', 'dot' (Graphviz), 'svg', or 'json'. Mermaid/dot/svg render the current page. \
        With output_path the content is written to disk; otherwise returned inline under data.content."
    )]
    async fn export_flowchart(&self, Parameters(input): Parameters<ExportInput>) -> String {
        let mut store = self.store.write().await;
        let Some(doc) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
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
                return error(category::INVALID_INPUT, format!("Unknown format '{other}'"), "Use drawio, mermaid, dot, svg, or json.")
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
}
