//! Built-in draw.io stencil catalog.
//!
//! draw.io's shape libraries (AWS, Azure, GCP, network, Kubernetes, UML, ER,
//! BPMN, mockups…) are bundled in the app and referenced purely by a `shape=`
//! style token. We therefore don't ship any art: we emit the correct token and
//! the stencil renders when the file is opened in diagrams.net.
//!
//! A node's stencil is a friendly catalog key (e.g. `aws.ec2`) or a raw
//! `mxgraph.<lib>.<name>` token passed straight through.

use serde_json::{json, Value};

/// A resolved stencil: the draw.io `shape=` token body and the library it
/// belongs to (drives the base style).
pub struct Resolved {
    /// e.g. "mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.ec2" or "mxgraph.kubernetes.pod".
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
    fn from_path(path: &str) -> Self {
        let p = path.strip_prefix("mxgraph.").unwrap_or(path);
        match p.split('.').next().unwrap_or("") {
            "aws4" | "aws3" | "aws" => Self::Aws,
            "azure" | "mscae" => Self::Azure,
            "gcp2" | "gcp" => Self::Gcp,
            "cisco" | "cisco19" | "networks" | "rack" => Self::Network,
            "kubernetes" => Self::Kubernetes,
            "uml" => Self::Uml,
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

    fn label(self) -> &'static str {
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

/// Catalog entry: friendly key → (mxgraph path, human description).
/// For AWS the path is the bare resource name; `resolve` wraps it in the
/// `resourceIcon`/`resIcon` form that draw.io expects.
struct Entry {
    key: &'static str,
    path: &'static str,
    desc: &'static str,
}

/// Curated subset of the most-used stencils. The long tail is reachable via the
/// raw `mxgraph.<lib>.<name>` passthrough in [`resolve`].
const CATALOG: &[Entry] = &[
    // AWS (aws4) — path is the resource icon name.
    Entry { key: "aws.ec2", path: "mxgraph.aws4.ec2", desc: "EC2 instance" },
    Entry { key: "aws.lambda", path: "mxgraph.aws4.lambda", desc: "Lambda function" },
    Entry { key: "aws.s3", path: "mxgraph.aws4.s3", desc: "S3 bucket" },
    Entry { key: "aws.rds", path: "mxgraph.aws4.rds", desc: "RDS database" },
    Entry { key: "aws.dynamodb", path: "mxgraph.aws4.dynamodb", desc: "DynamoDB" },
    Entry { key: "aws.vpc", path: "mxgraph.aws4.virtual_private_cloud", desc: "VPC" },
    Entry { key: "aws.api_gateway", path: "mxgraph.aws4.api_gateway", desc: "API Gateway" },
    Entry { key: "aws.cloudfront", path: "mxgraph.aws4.cloudfront", desc: "CloudFront CDN" },
    Entry { key: "aws.sns", path: "mxgraph.aws4.simple_notification_service", desc: "SNS" },
    Entry { key: "aws.sqs", path: "mxgraph.aws4.simple_queue_service", desc: "SQS" },
    Entry { key: "aws.elb", path: "mxgraph.aws4.elastic_load_balancing", desc: "Elastic Load Balancing" },
    Entry { key: "aws.user", path: "mxgraph.aws4.user", desc: "User" },
    // Azure
    Entry { key: "azure.vm", path: "mxgraph.azure.virtual_machine", desc: "Virtual machine" },
    Entry { key: "azure.sql", path: "mxgraph.azure.sql_database", desc: "SQL database" },
    Entry { key: "azure.storage", path: "mxgraph.azure.storage", desc: "Storage account" },
    Entry { key: "azure.functions", path: "mxgraph.azure.function_apps", desc: "Function app" },
    // GCP
    Entry { key: "gcp.compute", path: "mxgraph.gcp2.compute_engine", desc: "Compute Engine" },
    Entry { key: "gcp.storage", path: "mxgraph.gcp2.cloud_storage", desc: "Cloud Storage" },
    Entry { key: "gcp.functions", path: "mxgraph.gcp2.cloud_functions", desc: "Cloud Functions" },
    // Network (cisco19)
    Entry { key: "net.router", path: "mxgraph.cisco19.routers.router", desc: "Router" },
    Entry { key: "net.switch", path: "mxgraph.cisco19.switches.layer_3_switch", desc: "Switch" },
    Entry { key: "net.firewall", path: "mxgraph.cisco19.security.firewall", desc: "Firewall" },
    Entry { key: "net.server", path: "mxgraph.cisco19.servers.standard_host", desc: "Server" },
    Entry { key: "net.cloud", path: "mxgraph.cisco19.misc.cloud", desc: "Cloud" },
    // Kubernetes
    Entry { key: "k8s.pod", path: "mxgraph.kubernetes.pod", desc: "Pod" },
    Entry { key: "k8s.deploy", path: "mxgraph.kubernetes.deploy", desc: "Deployment" },
    Entry { key: "k8s.svc", path: "mxgraph.kubernetes.svc", desc: "Service" },
    Entry { key: "k8s.ing", path: "mxgraph.kubernetes.ing", desc: "Ingress" },
    Entry { key: "k8s.node", path: "mxgraph.kubernetes.node", desc: "Node" },
    // UML
    Entry { key: "uml.actor", path: "umlActor", desc: "UML actor (stick figure)" },
    Entry { key: "uml.component", path: "component", desc: "UML component" },
    Entry { key: "uml.lifeline", path: "umlLifeline", desc: "UML lifeline" },
    Entry { key: "uml.boundary", path: "umlBoundary", desc: "UML boundary" },
    // BPMN
    Entry { key: "bpmn.task", path: "mxgraph.bpmn.task", desc: "BPMN task" },
    Entry { key: "bpmn.gateway", path: "mxgraph.bpmn.gateway", desc: "BPMN gateway" },
    Entry { key: "bpmn.event", path: "mxgraph.bpmn.event", desc: "BPMN event" },
    // Mockup
    Entry { key: "mockup.button", path: "mxgraph.mockup.forms.button", desc: "UI button" },
    Entry { key: "mockup.textbox", path: "mxgraph.mockup.forms.textBox", desc: "UI text box" },
];

/// Resolve a stencil key or raw token into a draw.io `shape=` body + library.
/// Returns `None` only for an empty string.
pub fn resolve(key: &str) -> Option<Resolved> {
    let k = key.trim();
    if k.is_empty() {
        return None;
    }
    // Catalog hit.
    if let Some(e) = CATALOG.iter().find(|e| e.key.eq_ignore_ascii_case(k)) {
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
    let items: Vec<Value> = CATALOG
        .iter()
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
        assert!(v["count"].as_u64().unwrap() >= 5);
        let v = list(None, Some("lambda"));
        assert_eq!(v["count"], 1);
    }
}
