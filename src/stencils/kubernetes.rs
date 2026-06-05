//! Kubernetes (mxgraph.kubernetes) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "k8s.pod", path: "mxgraph.kubernetes.pod", desc: "Pod" },
    Entry { key: "k8s.deploy", path: "mxgraph.kubernetes.deploy", desc: "Deployment" },
    Entry { key: "k8s.rs", path: "mxgraph.kubernetes.rs", desc: "ReplicaSet" },
    Entry { key: "k8s.ds", path: "mxgraph.kubernetes.ds", desc: "DaemonSet" },
    Entry { key: "k8s.sts", path: "mxgraph.kubernetes.sts", desc: "StatefulSet" },
    Entry { key: "k8s.job", path: "mxgraph.kubernetes.job", desc: "Job" },
    Entry { key: "k8s.cronjob", path: "mxgraph.kubernetes.cronjob", desc: "CronJob" },
    Entry { key: "k8s.svc", path: "mxgraph.kubernetes.svc", desc: "Service" },
    Entry { key: "k8s.ing", path: "mxgraph.kubernetes.ing", desc: "Ingress" },
    Entry { key: "k8s.ep", path: "mxgraph.kubernetes.ep", desc: "Endpoints" },
    Entry { key: "k8s.node", path: "mxgraph.kubernetes.node", desc: "Node" },
    Entry { key: "k8s.ns", path: "mxgraph.kubernetes.ns", desc: "Namespace" },
    Entry { key: "k8s.cm", path: "mxgraph.kubernetes.cm", desc: "ConfigMap" },
    Entry { key: "k8s.secret", path: "mxgraph.kubernetes.secret", desc: "Secret" },
    Entry { key: "k8s.pv", path: "mxgraph.kubernetes.pv", desc: "PersistentVolume" },
    Entry { key: "k8s.pvc", path: "mxgraph.kubernetes.pvc", desc: "PersistentVolumeClaim" },
    Entry { key: "k8s.sa", path: "mxgraph.kubernetes.sa", desc: "ServiceAccount" },
    Entry { key: "k8s.hpa", path: "mxgraph.kubernetes.hpa", desc: "HorizontalPodAutoscaler" },
];
