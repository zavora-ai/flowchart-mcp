//! Google Cloud (mxgraph.gcp2) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "gcp.compute", path: "mxgraph.gcp2.compute_engine", desc: "Compute Engine" },
    Entry { key: "gcp.gke", path: "mxgraph.gcp2.kubernetes_engine", desc: "Kubernetes Engine" },
    Entry { key: "gcp.functions", path: "mxgraph.gcp2.cloud_functions", desc: "Cloud Functions" },
    Entry { key: "gcp.run", path: "mxgraph.gcp2.cloud_run", desc: "Cloud Run" },
    Entry { key: "gcp.app_engine", path: "mxgraph.gcp2.app_engine", desc: "App Engine" },
    Entry { key: "gcp.storage", path: "mxgraph.gcp2.cloud_storage", desc: "Cloud Storage" },
    Entry { key: "gcp.sql", path: "mxgraph.gcp2.cloud_sql", desc: "Cloud SQL" },
    Entry { key: "gcp.spanner", path: "mxgraph.gcp2.cloud_spanner", desc: "Cloud Spanner" },
    Entry { key: "gcp.bigtable", path: "mxgraph.gcp2.cloud_bigtable", desc: "Cloud Bigtable" },
    Entry { key: "gcp.firestore", path: "mxgraph.gcp2.cloud_firestore", desc: "Firestore" },
    Entry { key: "gcp.bigquery", path: "mxgraph.gcp2.bigquery", desc: "BigQuery" },
    Entry { key: "gcp.pubsub", path: "mxgraph.gcp2.cloud_pubsub", desc: "Pub/Sub" },
    Entry { key: "gcp.dataflow", path: "mxgraph.gcp2.cloud_dataflow", desc: "Dataflow" },
    Entry { key: "gcp.load_balancing", path: "mxgraph.gcp2.cloud_load_balancing", desc: "Cloud Load Balancing" },
    Entry { key: "gcp.vpc", path: "mxgraph.gcp2.virtual_private_cloud", desc: "VPC" },
    Entry { key: "gcp.cdn", path: "mxgraph.gcp2.cloud_cdn", desc: "Cloud CDN" },
    Entry { key: "gcp.iam", path: "mxgraph.gcp2.cloud_iam", desc: "Cloud IAM" },
];
