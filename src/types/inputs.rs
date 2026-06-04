//! Input structs for the MCP tools. All deny unknown fields so agents get a
//! clear error on typos rather than silently-ignored parameters.

use rmcp::schemars;
use serde::Deserialize;

use crate::engine::Style;

/// Reusable visual-style fields shared by add_node and style_node.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct StyleFields {
    /// Fill color hex (e.g. "#DAE8FC").
    pub fill: Option<String>,
    /// Stroke/border color hex.
    pub stroke: Option<String>,
    /// Text color hex.
    pub text_color: Option<String>,
    /// Stroke width in pixels.
    pub stroke_width: Option<f64>,
    /// Font family (e.g. "Helvetica").
    pub font_family: Option<String>,
    /// Font size in points.
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    /// Text alignment: "left", "center", or "right".
    pub align: Option<String>,
    /// Opacity 0–100.
    pub opacity: Option<f64>,
    /// Force rounded corners.
    pub rounded: Option<bool>,
    /// Drop shadow.
    pub shadow: Option<bool>,
    /// Dashed border.
    pub dashed: Option<bool>,
}

impl StyleFields {
    pub fn into_style(self) -> Style {
        Style {
            fill: self.fill,
            stroke: self.stroke,
            text_color: self.text_color,
            stroke_width: self.stroke_width,
            font_family: self.font_family,
            font_size: self.font_size,
            bold: self.bold,
            italic: self.italic,
            align: self.align,
            opacity: self.opacity,
            rounded: self.rounded,
            shadow: self.shadow,
            dashed: self.dashed,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    /// Flow direction: TB (default), BT, LR, or RL.
    pub direction: Option<String>,
    /// Optional diagram title.
    pub title: Option<String>,
    /// Optional template id (see list_templates) to pre-populate the chart.
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HandleInput {
    pub handle: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddNodeInput {
    pub handle: String,
    /// Unique node id (used to reference the node in edges).
    pub id: String,
    /// Display label. Defaults to the id when omitted.
    pub label: Option<String>,
    /// Shape: rectangle, round_rect, stadium, subroutine, cylinder, circle,
    /// double_circle, diamond, hexagon, parallelogram, parallelogram_alt,
    /// trapezoid, trapezoid_alt, note, card, document. Default rectangle.
    pub shape: Option<String>,
    /// Optional image (path or URI) rendered as the node.
    pub image: Option<String>,
    /// Optional draw.io stencil key (see list_stencils) or raw `mxgraph.*`
    /// token. Renders in the drawio export.
    pub stencil: Option<String>,
    #[serde(flatten)]
    pub style: StyleFields,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateNodeInput {
    pub handle: String,
    pub id: String,
    pub label: Option<String>,
    pub shape: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StyleNodeInput {
    pub handle: String,
    pub id: String,
    #[serde(flatten)]
    pub style: StyleFields,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetNodeImageInput {
    pub handle: String,
    pub id: String,
    /// Image path or URI. Omit to clear the image.
    pub image: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetNodeStencilInput {
    pub handle: String,
    pub id: String,
    /// Stencil key (see list_stencils) or raw `mxgraph.*` token. Omit to clear.
    pub stencil: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListStencilsInput {
    /// Filter by category: aws, azure, gcp, network, kubernetes, uml, bpmn, mockup.
    pub category: Option<String>,
    /// Free-text filter over keys and descriptions.
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveNodeInput {
    pub handle: String,
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddEdgeInput {
    pub handle: String,
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Optional edge label.
    pub label: Option<String>,
    /// Line style: solid (default), dotted, or thick.
    pub line: Option<String>,
    /// Draw an arrowhead at the target end (default true).
    pub arrow: Option<bool>,
    /// Target arrowhead: none, open, block, classic, diamond, oval, cross,
    /// er_one, er_many, er_zero_to_one, er_zero_to_many, er_one_to_many.
    pub end_arrow: Option<String>,
    /// Source arrowhead (same set as end_arrow).
    pub start_arrow: Option<String>,
    /// Routing: orthogonal (default), straight, curved, or entity_relation.
    pub routing: Option<String>,
    /// Edge color hex.
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StyleEdgeInput {
    pub handle: String,
    /// Edge index (see describe_flowchart).
    pub index: usize,
    pub start_arrow: Option<String>,
    pub end_arrow: Option<String>,
    pub routing: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveEdgeInput {
    pub handle: String,
    /// Edge index (as listed by describe_flowchart).
    pub index: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDirectionInput {
    pub handle: String,
    /// TB, BT, LR, or RL.
    pub direction: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSubgraphInput {
    pub handle: String,
    /// Unique container id.
    pub id: String,
    /// Container label.
    pub label: String,
    /// Member node ids (must already exist). May be empty for a pool.
    pub members: Vec<String>,
    /// Kind: group (default, dashed), container (titled box), swimlane, or pool.
    pub kind: Option<String>,
    /// For pools: "horizontal" (default) or "vertical" lane stacking.
    pub orientation: Option<String>,
    /// Parent container id, for nesting a lane inside a pool.
    pub parent: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddPageInput {
    pub handle: String,
    /// Optional page name.
    pub name: Option<String>,
    /// Flow direction for the new page: TB (default), BT, LR, RL.
    pub direction: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectPageInput {
    pub handle: String,
    /// Page index (0-based).
    pub index: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportInput {
    pub handle: String,
    /// Export format: drawio, mermaid, dot, svg, or json.
    pub format: String,
    /// Optional path to write the export to. When omitted, the content is
    /// returned inline in the response.
    pub output_path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportMermaidInput {
    /// Mermaid flowchart source text. Provide this or `file_path`.
    pub source: Option<String>,
    /// Path to a file containing Mermaid flowchart text.
    pub file_path: Option<String>,
}

// ---------------------------------------------------------------------------
// JSON round-trip + batch authoring
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportJsonInput {
    /// Document JSON (the exact shape produced by `export_flowchart format=json`).
    /// Provide this or `file_path`.
    pub json: Option<String>,
    /// Path to a `.json` file containing a serialized document.
    pub file_path: Option<String>,
}

/// One node in a batch page spec. Geometry is computed by auto-layout; you only
/// declare the id, label, shape, optional lane membership, image/stencil/style.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Unique node id within the page.
    pub id: String,
    /// Display label. Defaults to the id when omitted.
    pub label: Option<String>,
    /// Shape name (see add_node). Default rectangle.
    pub shape: Option<String>,
    /// Swimlane this node belongs to (must match one of the page's `lanes`).
    pub lane: Option<String>,
    /// Optional image path/URI.
    pub image: Option<String>,
    /// Optional draw.io stencil key or raw `mxgraph.*` token.
    pub stencil: Option<String>,
    #[serde(flatten)]
    pub style: StyleFields,
}

/// One edge in a batch page spec.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSpec {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Optional edge label.
    pub label: Option<String>,
    /// Line style: solid (default), dotted, thick.
    pub line: Option<String>,
    /// Draw an arrowhead at the target end (default true).
    pub arrow: Option<bool>,
    /// Target arrowhead (none/open/block/classic/diamond/oval/cross/er_*).
    pub end_arrow: Option<String>,
    /// Source arrowhead (same set as end_arrow).
    pub start_arrow: Option<String>,
    /// Routing: orthogonal (default), straight, curved, entity_relation.
    pub routing: Option<String>,
    /// Edge color hex.
    pub color: Option<String>,
}

/// One page of a batch document build.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    /// Optional page name (shown on the draw.io page tab). Defaults to Page-N.
    pub name: Option<String>,
    /// Optional in-diagram title banner.
    pub title: Option<String>,
    /// Flow direction for this page: TB, BT, LR, RL. Falls back to the
    /// document-level direction, then TB.
    pub direction: Option<String>,
    /// Nodes on the page.
    pub nodes: Vec<NodeSpec>,
    /// Edges on the page.
    #[serde(default)]
    pub edges: Vec<EdgeSpec>,
    /// Swimlane labels in stacking order. When non-empty, every node must
    /// declare a `lane` matching one of these labels.
    #[serde(default)]
    pub lanes: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildDocumentInput {
    /// Default flow direction for pages that don't set their own: TB (default),
    /// BT, LR, RL.
    pub direction: Option<String>,
    /// Pages to build, in order. The first page replaces the document's
    /// initial empty page; the rest are appended.
    pub pages: Vec<PageSpec>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPagesInput {
    pub handle: String,
    /// Export format: drawio, mermaid, dot, svg, or json (one file per page).
    pub format: String,
    /// Directory to write page files into (created if missing).
    pub output_dir: String,
    /// File name pattern. Tokens: {index} (1-based, zero-padded to 2),
    /// {name} (page name), {ext} (format extension). Default
    /// "{index}-{name}.{ext}".
    pub name_pattern: Option<String>,
}
