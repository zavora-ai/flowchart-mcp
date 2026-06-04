//! Structured JSON response builders shared by all tools.
//!
//! Success: `{ "status": "success", "message": ..., "data": { ... } }`
//! Error:   `{ "status": "error", "category": ..., "message": ..., "suggestion": ... }`

use serde_json::{json, Value};

/// Build a success response and serialize it to a JSON string.
pub fn success(message: impl Into<String>, data: Value) -> String {
    json!({ "status": "success", "message": message.into(), "data": data }).to_string()
}

/// Build an error response and serialize it to a JSON string.
pub fn error(category: &str, message: impl Into<String>, suggestion: impl Into<String>) -> String {
    json!({
        "status": "error",
        "category": category,
        "message": message.into(),
        "suggestion": suggestion.into(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        let s: Value = serde_json::from_str(&success("ok", json!({"n": 1}))).unwrap();
        assert_eq!(s["status"], "success");
        assert_eq!(s["data"]["n"], 1);

        let e: Value = serde_json::from_str(&error("not_found", "missing", "create first")).unwrap();
        assert_eq!(e["status"], "error");
        assert_eq!(e["category"], "not_found");
        assert_eq!(e["suggestion"], "create first");
    }
}
