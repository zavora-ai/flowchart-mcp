//! Built-in draw.io stencil catalog.
//!
//! draw.io's shape libraries (AWS, Azure, GCP, network, Kubernetes, UML, ER,
//! BPMN, mockups…) are bundled in the app and referenced purely by a `shape=`
//! style token. We therefore don't ship any art: we emit the correct token and
//! the stencil renders when the file is opened in diagrams.net.
//!
//! The catalog is split into one data file per library (see the modules below);
//! this file holds the shared logic that resolves, styles, and lists them.

mod aws;
mod azure;
mod bpmn;
mod gcp;
mod kubernetes;
mod mockup;
mod network;
mod uml;

use serde_json::{json, Value};

/// Catalog entry: friendly key → (mxgraph path, human description).
/// For AWS the path is the bare resource name; [`make`] wraps it in the
/// `resourceIcon`/`resIcon` form that draw.io expects.
pub struct Entry {
    pub key: &'static str,
    pub path: &'static str,
    pub desc: &'static str,
}

/// All per-library entry tables, assembled at compile time.
const LIBS: &[&[Entry]] = &[
    aws::ENTRIES,
    azure::ENTRIES,
    gcp::ENTRIES,
    network::ENTRIES,
    kubernetes::ENTRIES,
    uml::ENTRIES,
    bpmn::ENTRIES,
    mockup::ENTRIES,
];

/// Iterate every catalog entry across all libraries.
fn entries() -> impl Iterator<Item = &'static Entry> {
    LIBS.iter().flat_map(|s| s.iter())
}

/// A resolved stencil: the draw.io `shape=` token body and the library it
/// belongs to (drives the base style).
pub struct Resolved {
    /// e.g. "shape=mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.ec2" or "shape=mxgraph.kubernetes.pod".
    pub shape: String,
    pub library: Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Library {
    Aws,
    Azure,
    Gcp,
    Network,
    Kubernetes,
    Uml,
    Er,
    Bpmn,
    Mockup,
    Other,
}

impl Library {
    pub fn from_path(path: &str) -> Self {
        let p = path.strip_prefix("mxgraph.").unwrap_or(path);
        match p.split('.').next().unwrap_or("") {
            "aws4" | "aws3" | "aws" => Self::Aws,
            "azure" | "mscae" => Self::Azure,
            "gcp2" | "gcp" => Self::Gcp,
            "cisco" | "cisco19" | "networks" | "rack" => Self::Network,
            "kubernetes" => Self::Kubernetes,
            "uml" | "umlActor" | "umlLifeline" | "umlBoundary" | "umlControl" | "umlEntity"
            | "umlFrame" => Self::Uml,
            "er" => Self::Er,
            "bpmn" | "bpmn2" => Self::Bpmn,
            "mockup" => Self::Mockup,
            _ => Self::Other,
        }
    }

    /// The draw.io base style fragment for the library (trailing `;`).
    pub fn base_style(self) -> &'static str {
        match self {
            // AWS resource icons need these flags to render cleanly.
            Self::Aws => "sketch=0;outlineConnect=0;fontColor=#232F3E;gradientColor=none;\
                          fillColor=#E7157B;strokeColor=none;dashed=0;verticalLabelPosition=bottom;\
                          verticalAlign=top;align=center;html=1;",
            Self::Azure | Self::Gcp => {
                "sketch=0;outlineConnect=0;html=1;align=center;verticalLabelPosition=bottom;verticalAlign=top;"
            }
            Self::Network => "outlineConnect=0;html=1;align=center;verticalLabelPosition=bottom;verticalAlign=top;",
            Self::Kubernetes => "html=1;align=center;verticalLabelPosition=bottom;verticalAlign=top;",
            _ => "html=1;whiteSpace=wrap;",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Aws => "aws",
            Self::Azure => "azure",
            Self::Gcp => "gcp",
            Self::Network => "network",
            Self::Kubernetes => "kubernetes",
            Self::Uml => "uml",
            Self::Er => "er",
            Self::Bpmn => "bpmn",
            Self::Mockup => "mockup",
            Self::Other => "other",
        }
    }
}

/// Resolve a stencil key or raw token into a draw.io `shape=` body + library.
/// Returns `None` only for an empty string.
pub fn resolve(key: &str) -> Option<Resolved> {
    let k = key.trim();
    if k.is_empty() {
        return None;
    }
    // Catalog hit.
    if let Some(e) = entries().find(|e| e.key.eq_ignore_ascii_case(k)) {
        return Some(make(e.path));
    }
    // Raw passthrough: accept "mxgraph.<lib>.<name>", "shape=mxgraph...", or a
    // bare draw.io shape name (e.g. "umlActor").
    let raw = k.strip_prefix("shape=").unwrap_or(k);
    Some(make(raw))
}

/// Build a `Resolved` from an mxgraph path, applying AWS's resourceIcon wrapper.
fn make(path: &str) -> Resolved {
    let library = Library::from_path(path);
    let shape = if library == Library::Aws && path.starts_with("mxgraph.aws4.") {
        // AWS4 resource icons render via the resourceIcon shape + resIcon ref.
        format!("shape=mxgraph.aws4.resourceIcon;resIcon={path}")
    } else if path.contains('.') {
        format!("shape={path}")
    } else {
        // bare draw.io shape name (umlActor, component, …)
        path.to_string()
    };
    Resolved { shape, library }
}

/// Full draw.io style for a resolved stencil (token + library base style).
pub fn drawio_base(r: &Resolved) -> String {
    format!("{};{}", r.shape, r.library.base_style())
}

/// Catalog for `list_stencils`, optionally filtered by category and/or query.
pub fn list(category: Option<&str>, query: Option<&str>) -> Value {
    let cat = category.map(|c| c.to_ascii_lowercase());
    let q = query.map(|q| q.to_ascii_lowercase());
    let items: Vec<Value> = entries()
        .filter(|e| {
            let lib = Library::from_path(e.path).label();
            cat.as_deref().map(|c| lib == c || e.key.starts_with(c)).unwrap_or(true)
                && q
                    .as_deref()
                    .map(|q| e.key.contains(q) || e.desc.to_ascii_lowercase().contains(q))
                    .unwrap_or(true)
        })
        .map(|e| {
            json!({ "key": e.key, "library": Library::from_path(e.path).label(), "description": e.desc, "token": make(e.path).shape })
        })
        .collect();
    json!({
        "categories": ["aws", "azure", "gcp", "network", "kubernetes", "uml", "bpmn", "mockup"],
        "count": items.len(),
        "stencils": items,
        "note": "Pass any of these keys (or a raw 'mxgraph.<lib>.<name>' token) to add_node/set_node_stencil. Stencils render in the drawio export; svg/dot/mermaid show a labeled placeholder."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aws_with_resicon() {
        let r = resolve("aws.ec2").unwrap();
        assert!(r.shape.contains("resourceIcon"));
        assert!(r.shape.contains("resIcon=mxgraph.aws4.ec2"));
        assert_eq!(r.library, Library::Aws);
        assert!(drawio_base(&r).contains("aws4"));
    }

    #[test]
    fn resolves_kubernetes_key() {
        let r = resolve("k8s.pod").unwrap();
        assert_eq!(r.shape, "shape=mxgraph.kubernetes.pod");
        assert_eq!(r.library, Library::Kubernetes);
    }

    #[test]
    fn raw_passthrough() {
        let r = resolve("mxgraph.azure.load_balancer").unwrap();
        assert_eq!(r.shape, "shape=mxgraph.azure.load_balancer");
        assert_eq!(r.library, Library::Azure);
        // bare shape name
        assert_eq!(resolve("umlActor").unwrap().shape, "umlActor");
    }

    #[test]
    fn list_filters() {
        let v = list(Some("aws"), None);
        assert!(v["count"].as_u64().unwrap() >= 20);
        let v = list(None, Some("lambda"));
        assert_eq!(v["count"], 1);
    }

    #[test]
    fn catalog_is_substantial_and_unique() {
        let all: Vec<&str> = entries().map(|e| e.key).collect();
        assert!(all.len() >= 150, "catalog has {} entries", all.len());
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "duplicate stencil keys present");
    }
}
