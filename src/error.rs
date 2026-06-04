//! Engine error type and mapping to the response error taxonomy.

use thiserror::Error;

use crate::types::responses::error;

/// Errors produced by the flowchart engine.
#[derive(Debug, Error)]
pub enum FlowError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    Duplicate(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Error categories surfaced to the agent.
pub mod category {
    pub const NOT_FOUND: &str = "not_found";
    pub const DUPLICATE: &str = "duplicate";
    pub const INVALID_INPUT: &str = "invalid_input";
    pub const PARSE_ERROR: &str = "parse_error";
    pub const IO_ERROR: &str = "io_error";
}

fn classify(e: &FlowError) -> (&'static str, &'static str) {
    match e {
        FlowError::NotFound(_) => (category::NOT_FOUND, "Check the node id, edge index, or handle."),
        FlowError::Duplicate(_) => (category::DUPLICATE, "Use a unique id or update the existing item."),
        FlowError::InvalidInput(_) => (category::INVALID_INPUT, "Check the tool parameters."),
        FlowError::Parse(_) => (category::PARSE_ERROR, "Check the source diagram syntax."),
    }
}

/// Render a `FlowError` as a structured error response string.
pub fn engine_error(e: FlowError) -> String {
    let (cat, suggestion) = classify(&e);
    error(cat, e.to_string(), suggestion)
}

/// Convenience: a `not_found` error for an unknown flowchart handle.
pub fn unknown_handle(handle: &str) -> String {
    error(
        category::NOT_FOUND,
        format!("No flowchart for handle '{handle}'"),
        "Call create_flowchart or import_mermaid first.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn maps_variants() {
        let v: Value =
            serde_json::from_str(&engine_error(FlowError::NotFound("node 'a'".into()))).unwrap();
        assert_eq!(v["category"], category::NOT_FOUND);

        let v: Value =
            serde_json::from_str(&engine_error(FlowError::Duplicate("node 'a'".into()))).unwrap();
        assert_eq!(v["category"], category::DUPLICATE);

        let v: Value = serde_json::from_str(&unknown_handle("abc")).unwrap();
        assert_eq!(v["category"], category::NOT_FOUND);
    }
}
