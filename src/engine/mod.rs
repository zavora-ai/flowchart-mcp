//! Self-contained flowchart model and operations.
//!
//! The model is `serde`-serializable, so JSON export is free. Geometry is not
//! stored; it is computed on demand by [`layout`] for the draw.io and SVG
//! exporters.

pub mod export;
pub mod import;
pub mod layout;
pub mod pdf;
pub mod validate;

use serde::{Deserialize, Serialize};

use crate::error::FlowError;

/// Plain-text label for a node (HTML tags stripped when the label is rich).
/// Shared by the PDF exporter and re-exposed from `export`.
pub fn export_plain_label(node: &Node) -> String {
    export::plain_label(node)
}

/// Flow direction (rank axis + growth orientation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Top to bottom.
    TB,
    /// Bottom to top.
    BT,
    /// Left to right.
    LR,
    /// Right to left.
    RL,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "TB" | "TD" | "DOWN" => Some(Self::TB),
            "BT" | "UP" => Some(Self::BT),
            "LR" | "RIGHT" => Some(Self::LR),
            "RL" | "LEFT" => Some(Self::RL),
            _ => None,
        }
    }
    /// True when ranks stack vertically (TB/BT).
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::TB | Self::BT)
    }
    pub fn as_mermaid(self) -> &'static str {
        match self {
            Self::TB => "TD",
            Self::BT => "BT",
            Self::LR => "LR",
            Self::RL => "RL",
        }
    }
    pub fn as_dot(self) -> &'static str {
        if self.is_vertical() {
            "TB"
        } else {
            "LR"
        }
    }
}

/// Node geometry preset. Names mirror the common Mermaid node shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Rectangle,
    RoundRect,
    Stadium,
    Subroutine,
    Cylinder,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    Note,
    Card,
    Document,
    /// UML class box (name / attributes / methods compartments).
    UmlClass,
}

impl Shape {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "rect" | "rectangle" | "box" | "process" => Some(Self::Rectangle),
            "round" | "round_rect" | "rounded" => Some(Self::RoundRect),
            "stadium" | "pill" | "terminal" | "start_end" => Some(Self::Stadium),
            "subroutine" | "predefined" => Some(Self::Subroutine),
            "cylinder" | "database" | "db" => Some(Self::Cylinder),
            "circle" => Some(Self::Circle),
            "double_circle" | "doublecircle" => Some(Self::DoubleCircle),
            "diamond" | "rhombus" | "decision" => Some(Self::Diamond),
            "hexagon" | "hex" | "prepare" => Some(Self::Hexagon),
            "parallelogram" | "data" | "io" => Some(Self::Parallelogram),
            "parallelogram_alt" | "data_alt" => Some(Self::ParallelogramAlt),
            "trapezoid" => Some(Self::Trapezoid),
            "trapezoid_alt" => Some(Self::TrapezoidAlt),
            "note" | "comment" => Some(Self::Note),
            "card" => Some(Self::Card),
            "document" | "doc" => Some(Self::Document),
            "uml_class" | "class" | "umlclass" => Some(Self::UmlClass),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::RoundRect => "round_rect",
            Self::Stadium => "stadium",
            Self::Subroutine => "subroutine",
            Self::Cylinder => "cylinder",
            Self::Circle => "circle",
            Self::DoubleCircle => "double_circle",
            Self::Diamond => "diamond",
            Self::Hexagon => "hexagon",
            Self::Parallelogram => "parallelogram",
            Self::ParallelogramAlt => "parallelogram_alt",
            Self::Trapezoid => "trapezoid",
            Self::TrapezoidAlt => "trapezoid_alt",
            Self::Note => "note",
            Self::Card => "card",
            Self::Document => "document",
            Self::UmlClass => "uml_class",
        }
    }
    /// Wrap a label in this shape's Mermaid delimiters. Shapes without a native
    /// Mermaid form fall back to a rectangle.
    pub fn mermaid_wrap(self, label: &str) -> String {
        match self {
            Self::Rectangle | Self::Note | Self::Card | Self::Document | Self::UmlClass => {
                format!("[{label}]")
            }
            Self::RoundRect => format!("({label})"),
            Self::Stadium => format!("([{label}])"),
            Self::Subroutine => format!("[[{label}]]"),
            Self::Cylinder => format!("[({label})]"),
            Self::Circle => format!("(({label}))"),
            Self::DoubleCircle => format!("((({label})))"),
            Self::Diamond => format!("{{{label}}}"),
            Self::Hexagon => format!("{{{{{label}}}}}"),
            Self::Parallelogram => format!("[/{label}/]"),
            Self::ParallelogramAlt => format!("[\\{label}\\]"),
            Self::Trapezoid => format!("[/{label}\\]"),
            Self::TrapezoidAlt => format!("[\\{label}/]"),
        }
    }
}

/// Arrowhead style for an edge end (maps to draw.io arrow names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arrow {
    None,
    Open,
    Block,
    Classic,
    Diamond,
    Oval,
    Cross,
    /// Crow's-foot "one" (ERD).
    ErOne,
    /// Crow's-foot "many" (ERD).
    ErMany,
    ErZeroToOne,
    ErZeroToMany,
    ErOneToMany,
}

impl Arrow {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "none" => Some(Self::None),
            "open" => Some(Self::Open),
            "block" => Some(Self::Block),
            "classic" | "arrow" => Some(Self::Classic),
            "diamond" => Some(Self::Diamond),
            "oval" | "circle" => Some(Self::Oval),
            "cross" => Some(Self::Cross),
            "er_one" | "one" => Some(Self::ErOne),
            "er_many" | "many" | "crowsfoot" => Some(Self::ErMany),
            "er_zero_to_one" | "zero_or_one" => Some(Self::ErZeroToOne),
            "er_zero_to_many" | "zero_or_many" => Some(Self::ErZeroToMany),
            "er_one_to_many" | "one_or_many" => Some(Self::ErOneToMany),
            _ => None,
        }
    }
    pub fn drawio(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Open => "open",
            Self::Block => "block",
            Self::Classic => "classic",
            Self::Diamond => "diamond",
            Self::Oval => "oval",
            Self::Cross => "cross",
            Self::ErOne => "ERone",
            Self::ErMany => "ERmany",
            Self::ErZeroToOne => "ERzeroToOne",
            Self::ErZeroToMany => "ERzeroToMany",
            Self::ErOneToMany => "ERoneToMany",
        }
    }
}

/// Edge routing style (maps to draw.io edgeStyle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeRouting {
    Orthogonal,
    Straight,
    Curved,
    EntityRelation,
}

impl EdgeRouting {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "orthogonal" | "ortho" => Some(Self::Orthogonal),
            "straight" | "line" => Some(Self::Straight),
            "curved" | "curve" => Some(Self::Curved),
            "entity_relation" | "entity" | "er" => Some(Self::EntityRelation),
            _ => None,
        }
    }
    /// draw.io edge style fragment (trailing `;`).
    pub fn drawio(self) -> &'static str {
        match self {
            Self::Orthogonal => "edgeStyle=orthogonalEdgeStyle;rounded=0;",
            Self::Straight => "edgeStyle=none;rounded=0;",
            Self::Curved => "edgeStyle=orthogonalEdgeStyle;curved=1;rounded=0;",
            Self::EntityRelation => "edgeStyle=entityRelationEdgeStyle;rounded=0;",
        }
    }
}

/// Kind of node grouping container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerKind {
    /// Dashed, title-less grouping (default).
    Group,
    /// Solid titled box.
    Container,
    /// A single titled swimlane.
    Swimlane,
    /// An outer pool that holds swimlanes.
    Pool,
}

impl Default for ContainerKind {
    fn default() -> Self {
        Self::Group
    }
}

impl ContainerKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "group" => Some(Self::Group),
            "container" | "box" => Some(Self::Container),
            "swimlane" | "lane" => Some(Self::Swimlane),
            "pool" => Some(Self::Pool),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Container => "container",
            Self::Swimlane => "swimlane",
            Self::Pool => "pool",
        }
    }
}

/// Visual styling for a node (all optional; hex colors like `#RRGGBB`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Style {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub stroke_width: Option<f64>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    /// Text alignment: "left", "center", or "right".
    pub align: Option<String>,
    /// 0–100 opacity.
    pub opacity: Option<f64>,
    pub rounded: Option<bool>,
    pub shadow: Option<bool>,
    pub dashed: Option<bool>,
    /// Gradient end color (hex); fills from `fill` to this color.
    pub gradient: Option<String>,
    /// Hand-drawn / sketch style (draw.io rough.js).
    pub sketch: Option<bool>,
    /// Glossy "glass" overlay.
    pub glass: Option<bool>,
}

impl Style {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none()
            && self.stroke.is_none()
            && self.text_color.is_none()
            && self.stroke_width.is_none()
            && self.font_family.is_none()
            && self.font_size.is_none()
            && self.bold.is_none()
            && self.italic.is_none()
            && self.align.is_none()
            && self.opacity.is_none()
            && self.rounded.is_none()
            && self.shadow.is_none()
            && self.dashed.is_none()
            && self.gradient.is_none()
            && self.sketch.is_none()
            && self.glass.is_none()
    }

    /// Overlay `other`'s set fields onto `self`.
    pub fn merge(&mut self, other: Style) {
        if other.fill.is_some() {
            self.fill = other.fill;
        }
        if other.stroke.is_some() {
            self.stroke = other.stroke;
        }
        if other.text_color.is_some() {
            self.text_color = other.text_color;
        }
        if other.stroke_width.is_some() {
            self.stroke_width = other.stroke_width;
        }
        if other.font_family.is_some() {
            self.font_family = other.font_family;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.bold.is_some() {
            self.bold = other.bold;
        }
        if other.italic.is_some() {
            self.italic = other.italic;
        }
        if other.align.is_some() {
            self.align = other.align;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.rounded.is_some() {
            self.rounded = other.rounded;
        }
        if other.shadow.is_some() {
            self.shadow = other.shadow;
        }
        if other.dashed.is_some() {
            self.dashed = other.dashed;
        }
        if other.gradient.is_some() {
            self.gradient = other.gradient;
        }
        if other.sketch.is_some() {
            self.sketch = other.sketch;
        }
        if other.glass.is_some() {
            self.glass = other.glass;
        }
    }
}

/// A flowchart node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    #[serde(default)]
    pub style: Style,
    /// Optional image: a file path or data/remote URI shown on the node.
    #[serde(default)]
    pub image: Option<String>,
    /// Optional draw.io stencil key or raw `mxgraph.*` token (renders in the
    /// drawio export; takes precedence over `shape` there).
    #[serde(default)]
    pub stencil: Option<String>,
    /// Manual top-left `[x, y]` override. When set, the node is placed here
    /// instead of by auto-layout.
    #[serde(default)]
    pub pos: Option<[f64; 2]>,
    /// Manual `[width, height]` override.
    #[serde(default)]
    pub size: Option<[f64; 2]>,
    /// When true, the label is treated as rich HTML (`<b>`, `<i>`, `<br>`,
    /// `<font>`…): it renders formatted in the drawio export, and tags are
    /// stripped to plain text (with `<br>` as line breaks) in svg/mermaid/dot.
    #[serde(default)]
    pub html: Option<bool>,
    /// UML class compartments below the title (e.g. attributes, then methods).
    /// Each inner `Vec` is one compartment's member lines. Only used by the
    /// `uml_class` shape.
    #[serde(default)]
    pub compartments: Vec<Vec<String>>,
    /// Id of the layer this node belongs to (drawio layers). Default layer when
    /// unset.
    #[serde(default)]
    pub layer: Option<String>,
}

/// Edge line rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dotted,
    Thick,
}

impl LineStyle {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "solid" | "normal" => Some(Self::Solid),
            "dotted" | "dashed" | "dash" => Some(Self::Dotted),
            "thick" | "bold" => Some(Self::Thick),
            _ => None,
        }
    }
}

/// A directed connection between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_line")]
    pub line: LineStyle,
    /// Whether to draw an arrowhead at the target end.
    #[serde(default = "default_true")]
    pub arrow: bool,
    /// Arrowhead at the target end. When set, overrides `arrow`.
    #[serde(default)]
    pub end_arrow: Option<Arrow>,
    /// Arrowhead at the source end.
    #[serde(default)]
    pub start_arrow: Option<Arrow>,
    /// Edge routing.
    #[serde(default)]
    pub routing: Option<EdgeRouting>,
    /// Stroke color hex.
    #[serde(default)]
    pub color: Option<String>,
    /// Manual routing waypoints `[[x, y], ...]` the edge passes through.
    #[serde(default)]
    pub waypoints: Vec<[f64; 2]>,
    /// Fixed exit port on the source `[x, y]` in 0..1 (e.g. [1.0, 0.5] = right-middle).
    #[serde(default)]
    pub exit: Option<[f64; 2]>,
    /// Fixed entry port on the target `[x, y]` in 0..1.
    #[serde(default)]
    pub entry: Option<[f64; 2]>,
    /// Label position along the edge in `-1..1` (0 = middle, -1 = near source,
    /// 1 = near target).
    #[serde(default)]
    pub label_pos: Option<f64>,
    /// Label perpendicular offset in pixels.
    #[serde(default)]
    pub label_offset: Option<f64>,
    /// Label background color hex (`none` for transparent).
    #[serde(default)]
    pub label_bg: Option<String>,
    /// Label border color hex.
    #[serde(default)]
    pub label_border: Option<String>,
}

fn default_line() -> LineStyle {
    LineStyle::Solid
}
fn default_true() -> bool {
    true
}

impl Edge {
    /// Resolved target arrowhead, honoring `end_arrow` then the `arrow` flag.
    pub fn resolved_end(&self) -> Arrow {
        self.end_arrow
            .unwrap_or(if self.arrow { Arrow::Classic } else { Arrow::None })
    }
    pub fn resolved_start(&self) -> Arrow {
        self.start_arrow.unwrap_or(Arrow::None)
    }
}

/// A visual grouping of member nodes. Containers may nest via `parent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgraph {
    pub id: String,
    pub label: String,
    pub members: Vec<String>,
    #[serde(default)]
    pub kind: ContainerKind,
    /// For pools: "horizontal" (default) or "vertical" lane stacking.
    #[serde(default)]
    pub orientation: Option<String>,
    /// Parent container id, for nesting (e.g. a lane inside a pool).
    #[serde(default)]
    pub parent: Option<String>,
}

/// Auto-layout algorithm for a chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayoutKind {
    /// Layered ranks by longest path (default; flowcharts, swimlanes).
    #[default]
    Layered,
    /// Hierarchical tree: a root fans out to children along the flow direction.
    Tree,
    /// Mind map: a central root radiates branches both ways on the cross axis.
    MindMap,
}

impl LayoutKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "layered" | "flow" | "default" => Some(Self::Layered),
            "tree" | "hierarchy" | "org" => Some(Self::Tree),
            "mind_map" | "mindmap" | "radial" => Some(Self::MindMap),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Layered => "layered",
            Self::Tree => "tree",
            Self::MindMap => "mind_map",
        }
    }
}

/// A named draw.io layer. Nodes reference a layer by id; hidden layers are
/// emitted with `visible="0"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub label: String,
    #[serde(default = "default_true")]
    pub visible: bool,
}

/// A named color theme applied across a chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Blue,
    Green,
    Gray,
    Purple,
    Orange,
    Dark,
}

/// Resolved color set for a theme.
pub struct Palette {
    pub node_fill: &'static str,
    pub node_stroke: &'static str,
    pub accent_fill: &'static str,
    pub accent_stroke: &'static str,
    pub decision_fill: &'static str,
    pub decision_stroke: &'static str,
    pub text: &'static str,
    pub edge: &'static str,
}

impl Theme {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "blue" | "default" => Some(Self::Blue),
            "green" => Some(Self::Green),
            "gray" | "grey" | "mono" => Some(Self::Gray),
            "purple" => Some(Self::Purple),
            "orange" => Some(Self::Orange),
            "dark" | "midnight" => Some(Self::Dark),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Gray => "gray",
            Self::Purple => "purple",
            Self::Orange => "orange",
            Self::Dark => "dark",
        }
    }
    pub fn palette(self) -> Palette {
        match self {
            Self::Blue => Palette {
                node_fill: "#DAE8FC", node_stroke: "#6C8EBF",
                accent_fill: "#1F6FB8", accent_stroke: "#15527F",
                decision_fill: "#FFF2CC", decision_stroke: "#D6B656",
                text: "#1F2A37", edge: "#44515E",
            },
            Self::Green => Palette {
                node_fill: "#D5E8D4", node_stroke: "#82B366",
                accent_fill: "#2E7D32", accent_stroke: "#1B5E20",
                decision_fill: "#FFF2CC", decision_stroke: "#D6B656",
                text: "#1B3A1B", edge: "#4A6B4A",
            },
            Self::Gray => Palette {
                node_fill: "#F5F5F5", node_stroke: "#999999",
                accent_fill: "#5A5A5A", accent_stroke: "#333333",
                decision_fill: "#E8E8E8", decision_stroke: "#999999",
                text: "#222222", edge: "#666666",
            },
            Self::Purple => Palette {
                node_fill: "#E1D5E7", node_stroke: "#9673A6",
                accent_fill: "#6A1B9A", accent_stroke: "#4A148C",
                decision_fill: "#F3E5F5", decision_stroke: "#9673A6",
                text: "#2E1A33", edge: "#6E5577",
            },
            Self::Orange => Palette {
                node_fill: "#FFE6CC", node_stroke: "#D79B00",
                accent_fill: "#E65100", accent_stroke: "#BF360C",
                decision_fill: "#FFF2CC", decision_stroke: "#D6B656",
                text: "#3A2410", edge: "#8A5A2B",
            },
            Self::Dark => Palette {
                node_fill: "#2D3748", node_stroke: "#1A202C",
                accent_fill: "#3182CE", accent_stroke: "#2B6CB0",
                decision_fill: "#4A5568", decision_stroke: "#2D3748",
                text: "#F7FAFC", edge: "#A0AEC0",
            },
        }
    }
}

/// The full flowchart document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flowchart {
    pub direction: Direction,
    pub title: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    #[serde(default)]
    pub layout: LayoutKind,
    #[serde(default)]
    pub layers: Vec<Layer>,
}

impl Flowchart {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            title: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            layout: LayoutKind::Layered,
            layers: Vec::new(),
        }
    }

    pub fn set_layout(&mut self, layout: LayoutKind) {
        self.layout = layout;
    }

    /// Add a named layer (or update its label/visibility if it exists).
    pub fn add_layer(&mut self, id: &str, label: &str, visible: bool) -> Result<(), FlowError> {
        if id.trim().is_empty() {
            return Err(FlowError::InvalidInput("layer id must not be empty".into()));
        }
        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) {
            l.label = label.to_string();
            l.visible = visible;
        } else {
            self.layers.push(Layer {
                id: id.to_string(),
                label: label.to_string(),
                visible,
            });
        }
        Ok(())
    }

    /// Assign a node to a layer (or clear with `None`). The layer must exist.
    pub fn set_node_layer(&mut self, id: &str, layer: Option<String>) -> Result<(), FlowError> {
        if let Some(lid) = &layer {
            if !self.layers.iter().any(|l| &l.id == lid) {
                return Err(FlowError::NotFound(format!("layer '{lid}'")));
            }
        }
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].layer = layer;
        Ok(())
    }

    /// Apply a named color palette to every node (fill + matching stroke) and
    /// every edge (stroke). Returns the number of nodes restyled.
    pub fn apply_theme(&mut self, theme: Theme) -> usize {
        let p = theme.palette();
        for n in &mut self.nodes {
            // Terminators/decisions get the accent; others the base tint.
            let (fill, stroke) = match n.shape {
                Shape::Stadium | Shape::Circle | Shape::DoubleCircle => (p.accent_fill, p.accent_stroke),
                Shape::Diamond => (p.decision_fill, p.decision_stroke),
                _ => (p.node_fill, p.node_stroke),
            };
            n.style.fill = Some(fill.to_string());
            n.style.stroke = Some(stroke.to_string());
            n.style.text_color = Some(p.text.to_string());
        }
        for e in &mut self.edges {
            if e.color.is_none() {
                e.color = Some(p.edge.to_string());
            }
        }
        self.nodes.len()
    }

    fn node_index(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn has_node(&self, id: &str) -> bool {
        self.node_index(id).is_some()
    }

    /// Add a node. Errors if `id` already exists.
    pub fn add_node(&mut self, id: &str, label: &str, shape: Shape) -> Result<(), FlowError> {
        if id.trim().is_empty() {
            return Err(FlowError::InvalidInput("node id must not be empty".into()));
        }
        if self.has_node(id) {
            return Err(FlowError::Duplicate(format!("node '{id}'")));
        }
        self.nodes.push(Node {
            id: id.to_string(),
            label: label.to_string(),
            shape,
            style: Style::default(),
            image: None,
            stencil: None,
            pos: None,
            size: None,
            html: None,
            compartments: Vec::new(),
            layer: None,
        });
        Ok(())
    }

    /// Add a UML class node: a title plus ordered compartments (e.g. attributes
    /// then methods). Each compartment is a list of member lines.
    pub fn add_class_node(
        &mut self,
        id: &str,
        name: &str,
        compartments: Vec<Vec<String>>,
    ) -> Result<(), FlowError> {
        self.add_node(id, name, Shape::UmlClass)?;
        let idx = self.node_index(id).expect("just added");
        self.nodes[idx].compartments = compartments;
        Ok(())
    }

    /// Set (replace) a node's UML compartments.
    pub fn set_compartments(
        &mut self,
        id: &str,
        compartments: Vec<Vec<String>>,
    ) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].compartments = compartments;
        Ok(())
    }

    /// Set (or clear) a node's manual position/size override.
    pub fn move_node(
        &mut self,
        id: &str,
        pos: Option<[f64; 2]>,
        size: Option<[f64; 2]>,
        clear: bool,
    ) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        if clear {
            self.nodes[idx].pos = None;
            self.nodes[idx].size = None;
            return Ok(());
        }
        if pos.is_some() {
            self.nodes[idx].pos = pos;
        }
        if size.is_some() {
            self.nodes[idx].size = size;
        }
        Ok(())
    }

    /// Set (or clear) a node's image.
    pub fn set_node_image(&mut self, id: &str, image: Option<String>) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].image = image;
        Ok(())
    }

    /// Set (or clear) a node's stencil.
    pub fn set_node_stencil(&mut self, id: &str, stencil: Option<String>) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].stencil = stencil;
        Ok(())
    }

    /// Mark a node's label as rich HTML (or clear the flag).
    pub fn set_node_html(&mut self, id: &str, html: Option<bool>) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].html = html;
        Ok(())
    }

    /// Update a node's label and/or shape in place.
    pub fn update_node(
        &mut self,
        id: &str,
        label: Option<&str>,
        shape: Option<Shape>,
    ) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        if let Some(l) = label {
            self.nodes[idx].label = l.to_string();
        }
        if let Some(s) = shape {
            self.nodes[idx].shape = s;
        }
        Ok(())
    }

    /// Merge the provided style fields into a node's style.
    pub fn style_node(&mut self, id: &str, style: Style) -> Result<(), FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes[idx].style.merge(style);
        Ok(())
    }

    /// Remove a node and every edge touching it. Returns edges removed.
    pub fn remove_node(&mut self, id: &str) -> Result<usize, FlowError> {
        let idx = self
            .node_index(id)
            .ok_or_else(|| FlowError::NotFound(format!("node '{id}'")))?;
        self.nodes.remove(idx);
        let before = self.edges.len();
        self.edges.retain(|e| e.from != id && e.to != id);
        for sg in &mut self.subgraphs {
            sg.members.retain(|m| m != id);
        }
        Ok(before - self.edges.len())
    }

    /// Add an edge. Both endpoints must exist.
    pub fn add_edge(
        &mut self,
        from: &str,
        to: &str,
        label: Option<String>,
        line: LineStyle,
        arrow: bool,
    ) -> Result<usize, FlowError> {
        if !self.has_node(from) {
            return Err(FlowError::NotFound(format!("node '{from}'")));
        }
        if !self.has_node(to) {
            return Err(FlowError::NotFound(format!("node '{to}'")));
        }
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
            label,
            line,
            arrow,
            end_arrow: None,
            start_arrow: None,
            routing: None,
            color: None,
            waypoints: Vec::new(),
            exit: None,
            entry: None,
            label_pos: None,
            label_offset: None,
            label_bg: None,
            label_border: None,
        });
        Ok(self.edges.len() - 1)
    }

    /// Set arrowheads / routing / color on an existing edge (set fields only).
    pub fn style_edge(
        &mut self,
        index: usize,
        start_arrow: Option<Arrow>,
        end_arrow: Option<Arrow>,
        routing: Option<EdgeRouting>,
        color: Option<String>,
    ) -> Result<(), FlowError> {
        let e = self
            .edges
            .get_mut(index)
            .ok_or_else(|| FlowError::NotFound(format!("edge index {index}")))?;
        if start_arrow.is_some() {
            e.start_arrow = start_arrow;
        }
        if end_arrow.is_some() {
            e.end_arrow = end_arrow;
        }
        if routing.is_some() {
            e.routing = routing;
        }
        if color.is_some() {
            e.color = color;
        }
        Ok(())
    }

    /// Update an edge's label and/or line style (set fields only).
    pub fn update_edge(
        &mut self,
        index: usize,
        label: Option<String>,
        line: Option<LineStyle>,
    ) -> Result<(), FlowError> {
        let e = self
            .edges
            .get_mut(index)
            .ok_or_else(|| FlowError::NotFound(format!("edge index {index}")))?;
        if label.is_some() {
            e.label = label;
        }
        if let Some(l) = line {
            e.line = l;
        }
        Ok(())
    }

    /// Set manual waypoints and/or fixed exit/entry ports on an edge. With
    /// `clear`, all three are reset to auto.
    pub fn route_edge(
        &mut self,
        index: usize,
        waypoints: Option<Vec<[f64; 2]>>,
        exit: Option<[f64; 2]>,
        entry: Option<[f64; 2]>,
        clear: bool,
    ) -> Result<(), FlowError> {
        let e = self
            .edges
            .get_mut(index)
            .ok_or_else(|| FlowError::NotFound(format!("edge index {index}")))?;
        if clear {
            e.waypoints.clear();
            e.exit = None;
            e.entry = None;
            return Ok(());
        }
        if let Some(w) = waypoints {
            e.waypoints = w;
        }
        if exit.is_some() {
            e.exit = exit;
        }
        if entry.is_some() {
            e.entry = entry;
        }
        Ok(())
    }

    /// Set edge label placement/style (set fields only).
    pub fn label_edge(
        &mut self,
        index: usize,
        pos: Option<f64>,
        offset: Option<f64>,
        bg: Option<String>,
        border: Option<String>,
    ) -> Result<(), FlowError> {
        let e = self
            .edges
            .get_mut(index)
            .ok_or_else(|| FlowError::NotFound(format!("edge index {index}")))?;
        if pos.is_some() {
            e.label_pos = pos;
        }
        if offset.is_some() {
            e.label_offset = offset;
        }
        if bg.is_some() {
            e.label_bg = bg;
        }
        if border.is_some() {
            e.label_border = border;
        }
        Ok(())
    }

    /// Remove the edge at `index`.
    pub fn remove_edge(&mut self, index: usize) -> Result<(), FlowError> {
        if index >= self.edges.len() {
            return Err(FlowError::NotFound(format!("edge index {index}")));
        }
        self.edges.remove(index);
        Ok(())
    }

    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /// Add a container. Member ids and the optional parent must exist.
    pub fn add_subgraph(
        &mut self,
        id: &str,
        label: &str,
        members: Vec<String>,
        kind: ContainerKind,
        orientation: Option<String>,
        parent: Option<String>,
    ) -> Result<(), FlowError> {
        if self.subgraphs.iter().any(|s| s.id == id) {
            return Err(FlowError::Duplicate(format!("subgraph '{id}'")));
        }
        for m in &members {
            if !self.has_node(m) {
                return Err(FlowError::NotFound(format!("node '{m}'")));
            }
        }
        if let Some(p) = &parent {
            if !self.subgraphs.iter().any(|s| &s.id == p) {
                return Err(FlowError::NotFound(format!("parent container '{p}'")));
            }
        }
        self.subgraphs.push(Subgraph {
            id: id.to_string(),
            label: label.to_string(),
            members,
            kind,
            orientation,
            parent,
        });
        Ok(())
    }
}

/// A multi-page flowchart document. Page 0 always exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub pages: Vec<Page>,
    pub current: usize,
}

/// A named page wrapping one flowchart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub name: String,
    #[serde(flatten)]
    pub chart: Flowchart,
}

impl Document {
    pub fn new(direction: Direction) -> Self {
        Self {
            pages: vec![Page {
                name: "Page-1".to_string(),
                chart: Flowchart::new(direction),
            }],
            current: 0,
        }
    }

    /// The currently selected page's chart.
    pub fn chart(&mut self) -> &mut Flowchart {
        &mut self.pages[self.current].chart
    }

    pub fn chart_ref(&self) -> &Flowchart {
        &self.pages[self.current].chart
    }

    /// Add a page; returns its index and selects it.
    pub fn add_page(&mut self, name: Option<String>, direction: Direction) -> usize {
        let idx = self.pages.len();
        let name = name.unwrap_or_else(|| format!("Page-{}", idx + 1));
        self.pages.push(Page {
            name,
            chart: Flowchart::new(direction),
        });
        self.current = idx;
        idx
    }

    /// Select an existing page by index.
    pub fn select_page(&mut self, index: usize) -> Result<(), FlowError> {
        if index >= self.pages.len() {
            return Err(FlowError::NotFound(format!("page index {index}")));
        }
        self.current = index;
        Ok(())
    }
}
