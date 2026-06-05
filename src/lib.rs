//! flowchart-mcp-server — MCP server for authoring flowcharts and exporting
//! them to draw.io (mxGraph XML), Mermaid, Graphviz DOT, SVG, and JSON, with
//! Mermaid import. Self-contained engine; no external services.

pub mod engine;
pub mod error;
pub mod sequence;
pub mod server;
pub mod stencils;
pub mod store;
pub mod templates;
pub mod types;

pub use server::FlowchartServer;
