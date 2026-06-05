//! UML stencil entries. Some are core draw.io shape names (no `mxgraph.`
//! prefix), others come from the `mxgraph.uml` library.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "uml.actor", path: "umlActor", desc: "Actor (stick figure)" },
    Entry { key: "uml.lifeline", path: "umlLifeline", desc: "Sequence lifeline" },
    Entry { key: "uml.boundary", path: "umlBoundary", desc: "Boundary" },
    Entry { key: "uml.control", path: "umlControl", desc: "Control" },
    Entry { key: "uml.entity", path: "umlEntity", desc: "Entity" },
    Entry { key: "uml.frame", path: "umlFrame", desc: "Frame" },
    Entry { key: "uml.component", path: "component", desc: "Component" },
    Entry { key: "uml.module", path: "module", desc: "Module" },
    Entry { key: "uml.provided_interface", path: "lollipop", desc: "Provided interface (lollipop)" },
    Entry { key: "uml.required_interface", path: "requiredInterface", desc: "Required interface (socket)" },
    Entry { key: "uml.note", path: "note", desc: "Note" },
    Entry { key: "uml.package", path: "folder", desc: "Package / folder" },
    Entry { key: "uml.state_start", path: "startState", desc: "Initial state" },
    Entry { key: "uml.state_end", path: "endState", desc: "Final state" },
    Entry { key: "uml.assembly", path: "providedRequiredInterface", desc: "Assembly connector" },
];
