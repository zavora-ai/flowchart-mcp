//! UI mockup / wireframe (mxgraph.mockup) stencil entries.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    Entry { key: "mockup.button", path: "mxgraph.mockup.forms.button", desc: "Button" },
    Entry { key: "mockup.textbox", path: "mxgraph.mockup.forms.textBox", desc: "Text box" },
    Entry { key: "mockup.textarea", path: "mxgraph.mockup.forms.textArea", desc: "Text area" },
    Entry { key: "mockup.checkbox", path: "mxgraph.mockup.forms.checkbox", desc: "Checkbox" },
    Entry { key: "mockup.radio", path: "mxgraph.mockup.forms.radioButton", desc: "Radio button" },
    Entry { key: "mockup.dropdown", path: "mxgraph.mockup.forms.comboBox", desc: "Dropdown / combo box" },
    Entry { key: "mockup.search", path: "mxgraph.mockup.forms.searchBox", desc: "Search box" },
    Entry { key: "mockup.slider", path: "mxgraph.mockup.forms.horizontalSlider", desc: "Slider" },
    Entry { key: "mockup.browser", path: "mxgraph.mockup.containers.browserWindow", desc: "Browser window" },
    Entry { key: "mockup.window", path: "mxgraph.mockup.containers.window", desc: "Window" },
    Entry { key: "mockup.image", path: "mxgraph.mockup.graphics.image", desc: "Image placeholder" },
    Entry { key: "mockup.video", path: "mxgraph.mockup.graphics.video", desc: "Video player" },
];
