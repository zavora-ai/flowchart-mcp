//! Self-contained flowchart model and operations.
//!
//! The model is `serde`-serializable, so JSON export is free. Geometry is not
//! stored; it is computed on demand by [`layout`] for the draw.io and SVG
//! exporters.

pub mod export;
pub mod import;
pub mod layout;
pub mod validate;

use serde::{Deserialize, Serialize};

use crate::error::FlowError;

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
        }
    }
    /// Wrap a label in this shape's Mermaid delimiters. Shapes without a native
    /// Mermaid form fall back to a rectangle.
    pub fn mermaid_wrap(self, label: &str) -> String {
        match self {
            Self::Rectangle | Self::Note | Self::Card | Self::Document => format!("[{label}]"),
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

/// The full flowchart document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flowchart {
    pub direction: Direction,
    pub title: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
}

impl Flowchart {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            title: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
        }
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
        });
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
