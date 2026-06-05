//! BPMN (mxgraph.bpmn) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "bpmn.task", path: "mxgraph.bpmn.task", desc: "Task" },
    Entry { key: "bpmn.gateway", path: "mxgraph.bpmn.gateway", desc: "Gateway" },
    Entry { key: "bpmn.gateway_parallel", path: "mxgraph.bpmn.gateway_parallel", desc: "Parallel gateway" },
    Entry { key: "bpmn.gateway_exclusive", path: "mxgraph.bpmn.gateway_exclusive", desc: "Exclusive gateway" },
    Entry { key: "bpmn.event", path: "mxgraph.bpmn.event", desc: "Event" },
    Entry { key: "bpmn.event_start", path: "mxgraph.bpmn.event_start", desc: "Start event" },
    Entry { key: "bpmn.event_end", path: "mxgraph.bpmn.event_end", desc: "End event" },
    Entry { key: "bpmn.subprocess", path: "mxgraph.bpmn.subProcessMarker", desc: "Sub-process" },
    Entry { key: "bpmn.data_object", path: "mxgraph.bpmn.data_object", desc: "Data object" },
    Entry { key: "bpmn.data_store", path: "mxgraph.bpmn.data_store", desc: "Data store" },
    Entry { key: "bpmn.message", path: "mxgraph.bpmn.message", desc: "Message" },
    Entry { key: "bpmn.timer", path: "mxgraph.bpmn.timer", desc: "Timer event" },
];
