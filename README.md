# flowchart-mcp-server

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)

32 MCP tools for authoring diagrams and exporting them to **draw.io** (diagrams.net mxGraph XML), **Mermaid**, **Graphviz DOT**, **SVG**, **PDF**, and **JSON** — plus **Mermaid import**. Multi-page documents, swimlanes & nested containers, rich node/edge styling, **named themes/palettes**, **gradients & sketch style**, **layers**, node images, **draw.io stencil libraries** (AWS/Azure/GCP/network/Kubernetes/UML/BPMN), **manual layout & routing overrides** (node position/size, edge waypoints & fixed ports, edge label position), **layered / tree / mind-map auto-layout**, a **true UML class shape** with compartments, self-loop edges, and ready-made chart types (flowchart, swimlane, org chart, mind map, UML class, ERD, BPMN, state machine). Plus a dedicated **UML sequence-diagram** subsystem (7 more tools: participants, lifelines, ordered messages → draw.io / Mermaid / SVG / JSON). 39 tools total. Pure Rust, local-first, no external services.

## Install

```bash
cargo install flowchart-mcp-server
```

## Configure

```json
{
  "mcpServers": {
    "flowchart": {
      "command": "flowchart-mcp-server"
    }
  }
}
```

## How it works

A document is held in memory and referenced by a `handle` returned from `create_flowchart` or `import_mermaid`. A document has one or more **pages**; editing tools target the currently selected page. You build the diagram with node/edge/container tools, then export to any format inline or to a file. Geometry for draw.io and SVG is computed by a built-in layered auto-layout — you never place coordinates by hand.

```jsonc
// 1. create
create_flowchart { "direction": "TB", "title": "Login flow" }      // → { handle }
// 2. build
add_node  { "handle": h, "id": "start", "label": "Start", "shape": "stadium" }
add_node  { "handle": h, "id": "auth",  "label": "Valid?", "shape": "diamond" }
add_node  { "handle": h, "id": "home",  "label": "Home",  "shape": "rectangle", "fill": "#D5E8D4", "bold": true }
add_edge  { "handle": h, "from": "start", "to": "auth" }
add_edge  { "handle": h, "from": "auth",  "to": "home", "label": "yes", "end_arrow": "block" }
// 3. export
export_flowchart { "handle": h, "format": "drawio", "output_path": "login.drawio" }
```

## Tools

| Tool | Purpose |
|------|---------|
| `create_flowchart` | New document (direction `TB`/`BT`/`LR`/`RL`, optional title and template). |
| `list_templates` | List starter templates. |
| `describe_flowchart` | Direction, title, page list, and full node/edge/container listing (with edge indexes). |
| `close_flowchart` | Free a document's memory. |
| `add_node` | Add a node with a shape, optional image, and rich style. |
| `update_node` | Change a node's label and/or shape. |
| `style_node` | Set fill/stroke/text color, stroke width, font, bold/italic, align, opacity, rounded, shadow, dashed. |
| `set_node_image` | Set or clear a node's image (rendered as an image shape). |
| `set_node_stencil` | Set or clear a node's draw.io stencil (AWS/Azure/GCP/network/k8s/UML/BPMN). |
| `list_stencils` | Browse/search the built-in draw.io stencil catalog. |
| `remove_node` | Delete a node and its incident edges. |
| `move_node` | Manually place/size a node (x/y/w/h), overriding auto-layout; clear to revert. |
| `add_edge` | Connect two nodes (line, label, arrowheads, routing, color, fixed ports, waypoints). |
| `style_edge` | Set start/end arrowheads, routing, and color on an existing edge. |
| `update_edge` | Change an existing edge's label and/or line style. |
| `route_edge` | Manually route an edge (waypoints + fixed exit/entry ports); clear to revert. |
| `remove_edge` | Delete an edge by index. |
| `set_direction` | Change the current page's flow direction. |
| `set_layout` | Switch auto-layout: `layered` (default), `tree`, or `mind_map`. |
| `apply_theme` | Recolor the page with a named palette (blue/green/gray/purple/orange/dark). |
| `add_layer` | Add a named draw.io layer (with visibility). |
| `set_node_layer` | Assign a node to a layer. |
| `label_edge` | Position/style an edge's label (along-edge pos, offset, background, border). |
| `set_step_numbering` | Toggle sequential step-number badges on the current page (numbered process map). |
| `add_subgraph` | Group nodes into a container (group/container/swimlane/pool, optionally nested). |
| `add_page` | Add a page to the document and select it. |
| `select_page` | Select the active page by index. |
| `export_flowchart` | Export to `drawio`, `mermaid`, `dot`, `svg`, `pdf`, or `json`. |
| `export_pages` | Export every page to its own file in a directory (configurable name pattern). |
| `import_mermaid` | Parse Mermaid flowchart text into a new document. |
| `import_json` | Load a full document from `json` export (inline or file) — round-trips every feature. |
| `build_document` | Build a whole multi-page document (nodes, edges, swimlanes) in **one** call. |
| `validate_flowchart` | Check the document against correctness properties (labeled decisions, reachability, no overlaps) and return a report. |

## Batch authoring

For large or multi-page diagrams, `build_document` constructs an entire document
in a single call — no per-node/edge round-trips. Geometry is auto-laid-out, and
swimlanes are declared per page as a `lanes` list with each node naming its `lane`.

```jsonc
build_document {
  "direction": "LR",
  "pages": [
    {
      "name": "Manifest",
      "title": "1. Manifest Capture",
      "lanes": ["Manifest Team", "System", "Customer"],
      "nodes": [
        { "id": "s",   "label": "Start",          "shape": "stadium",  "lane": "Manifest Team" },
        { "id": "dec", "label": "Consolidated?",  "shape": "diamond",  "lane": "Manifest Team" },
        { "id": "job", "label": "Job File No.",   "shape": "document", "lane": "System" }
      ],
      "edges": [
        { "from": "s", "to": "dec" },
        { "from": "dec", "to": "job", "label": "yes" }
      ]
    }
    // …more pages
  ]
}                                   // → { handle, page_count, node_count, edge_count }

export_pages { "handle": h, "format": "drawio", "output_dir": "out",
               "name_pattern": "{index}-{name}.{ext}" }   // one file per page
```

Pass `"number_steps": true` on `build_document` (or per page, or via the
`set_step_numbering` tool) to stamp sequential step-number badges on each step
in flow order — a numbered process map. Start/End terminators are not numbered.

`build_document` validates ids, edge endpoints, shapes, and lane membership up
front, so a bad spec fails cleanly without creating a half-built document.
`import_json` reloads a document previously saved with `export_flowchart format=json`,
making JSON a lossless save format for the whole pipeline.



`rectangle` · `round_rect` · `stadium` · `subroutine` · `cylinder` · `circle` · `double_circle` · `diamond` · `hexagon` · `parallelogram` · `parallelogram_alt` · `trapezoid` · `trapezoid_alt` · `note` · `card` · `document` · `uml_class`

The `uml_class` shape renders a class box with a bold title and a separated row per **compartment** (pass `compartments` as an array of line-arrays, e.g. attributes then methods). It renders as an HTML class table in draw.io and as a partitioned box in SVG.

Each shape maps to the closest native primitive in every export target (e.g. `diamond` → `rhombus` in draw.io, `diamond` in DOT, a polygon in SVG, `{...}` in Mermaid). Shapes without a native Mermaid form fall back to a rectangle there.

## Stencil libraries

Any node can use a built-in **draw.io stencil** instead of a primitive shape, giving access to the AWS, Azure, GCP, Cisco/network, Kubernetes, UML, BPMN, and mockup icon sets. Set one with `set_node_stencil` or the `stencil` field on `add_node`:

```jsonc
add_node { "handle": h, "id": "api", "label": "API", "stencil": "aws.api_gateway" }
add_node { "handle": h, "id": "db",  "label": "Users", "stencil": "aws.rds" }
```

`list_stencils { category?, query? }` browses the curated catalog of ~160 friendly keys (e.g. `aws.ec2`, `aws.lambda`, `azure.aks`, `gcp.bigquery`, `k8s.statefulset`, `net.firewall`, `uml.actor`, `bpmn.gateway`, `mockup.button`). Beyond the catalog, you can pass any raw `mxgraph.<lib>.<name>` token — e.g. `"mxgraph.azure.load_balancer"` — and it is emitted verbatim.

Stencils render with full fidelity in the **drawio** export (open in diagrams.net). The **svg**/**dot**/**mermaid** exports show a labeled placeholder box instead, since the stencil artwork lives inside draw.io.

## Styling

`style_node` (and the style fields on `add_node`) accept: `fill`, `stroke`, `text_color` (hex), `stroke_width`, `font_family`, `font_size`, `bold`, `italic`, `align` (`left`/`center`/`right`), `opacity` (0–100), `rounded`, `shadow`, `dashed`, `gradient` (end-color hex), `sketch` (hand-drawn), and `glass`. Only provided fields change.

## Themes

`apply_theme` (or the `theme` field on `create_flowchart`) recolors every node and edge with a coordinated palette: `blue` · `green` · `gray` · `purple` · `orange` · `dark`. Terminators/decisions get accent colors; other nodes a matching tint. Apply it last, after building, since it overwrites per-node fill/stroke/text colors.

## Layers

`add_layer` creates a named draw.io layer (with a `visible` flag); `set_node_layer` (or the `layer` field on `add_node`) assigns nodes to it. The **drawio** export emits real layer cells (hidden layers carry `visible="0"`), so you can toggle them in diagrams.net.

Set `html: true` on `add_node`/`update_node` to treat the label as **rich HTML** (`<b>`, `<i>`, `<br>`, `<font>`…): it renders formatted in the **drawio** export, and tags are stripped to plain text (with `<br>` as spaces) in mermaid/dot/svg/pdf.

## Edges & arrowheads

`add_edge`/`style_edge` accept `start_arrow` and `end_arrow` from:

`none` · `open` · `block` · `classic` · `diamond` · `oval` · `cross` · `er_one` · `er_many` · `er_zero_to_one` · `er_zero_to_many` · `er_one_to_many`

…plus `routing` (`orthogonal` / `straight` / `curved` / `entity_relation`), `line` (`solid` / `dotted` / `thick`), and `color`. The crow's-foot (`er_*`) arrowheads and `entity_relation` routing produce proper ERD connectors in draw.io.

## Containers, swimlanes & pools

`add_subgraph` groups nodes into a container whose `kind` is:

- `group` — dashed, title-less grouping (default)
- `container` — solid titled box
- `swimlane` — a single titled lane
- `pool` — an outer pool that holds swimlanes (pass `parent` on each lane and optional `orientation` `horizontal`/`vertical`)

Containers nest via `parent`, so a pool of lanes round-trips to draw.io as real swimlane cells with relative geometry.

## Pages

A document starts with one page. `add_page` appends a page (and selects it); `select_page` switches the active page. The **drawio** export emits every page as a separate `<diagram>`; **mermaid**/**dot**/**svg** render the current page.

## Auto-layout

Geometry is computed for you. `set_layout` (or the `layout` field on `create_flowchart`) selects the algorithm:

- `layered` (default) — ranked top-down/left-right flow; lane-aware when swimlanes are present.
- `tree` — a root fans out to children along the flow direction with non-overlapping subtrees (org charts, hierarchies).
- `mind_map` — a central root radiates branches both ways on the cross axis.

## Manual layout & routing

Layout is automatic by default, but you can pin specifics when you need exact control — the rest of the diagram still auto-lays-out around your overrides, and the canvas grows to fit.

- `move_node` sets a node's top-left `x`/`y` and/or `w`/`h` in canvas pixels; `clear: true` returns it to auto-layout.
- `route_edge` (or the `exit`/`entry`/`waypoints` fields on `add_edge`) fixes an edge's connection ports and the points it passes through. Ports are `[x, y]` in `0..1` on the source/target box (e.g. `[1.0, 0.5]` = right-middle); waypoints are `[x, y]` canvas pixels. `clear: true` resets to automatic routing.

```jsonc
move_node  { "handle": h, "id": "db", "x": 640, "y": 80, "w": 200, "h": 90 }
route_edge { "handle": h, "index": 0, "exit": [1.0, 0.5], "entry": [0.0, 0.5],
             "waypoints": [[480, 100], [480, 220]] }
```

These overrides round-trip through the **json** export and render in **drawio** and **svg**.

## Export formats

- **drawio** — diagrams.net mxGraph XML with computed geometry, per-shape styles, rich fonts/opacity/shadows, image shapes, arrowheads on both ends, edge routing, nested containers/swimlanes, and one page per document page. Open directly in [app.diagrams.net](https://app.diagrams.net).
- **mermaid** — `flowchart` source with shapes, edge labels, line styles, subgraphs, and `style` directives.
- **dot** — Graphviz `digraph` with `rankdir`, clusters, shape mapping, and edge attributes. Render with `dot -Tpng`.
- **svg** — self-contained SVG using the auto-layout, with arrowheads, dashed/thick edges, images, containers, and shape outlines.
- **pdf** — single-page **vector** PDF rendered from the same layout (zero-dependency; current page). Binary, so it requires `output_path`. For raster (PNG), open the SVG or drawio export in any viewer.
- **json** — the raw serialized document (all pages) for programmatic round-tripping.

With `output_path` the content is written to disk; otherwise it is returned inline under `data.content` (PDF always requires `output_path`).

## Templates

`create_flowchart` accepts a `template` id to pre-populate the document:

```
basic · decision · approval · etl · swimlane · org_chart · mind_map · uml_class · erd · bpmn · state_machine
```

## Sequence diagrams

UML sequence diagrams are a separate subsystem with their own handle space (a sequence handle is not a flowchart handle). Build a diagram from **participants** (lifelines) and ordered **messages**, then export it.

```jsonc
create_sequence   { "title": "Login" }                                  // → { handle }
add_participant   { "handle": h, "id": "u", "label": "User", "actor": true }
add_participant   { "handle": h, "id": "api", "label": "API" }
add_message       { "handle": h, "from": "u",  "to": "api", "label": "POST /login", "kind": "sync" }
add_message       { "handle": h, "from": "api", "to": "u",  "label": "200 OK",      "kind": "return" }
export_sequence   { "handle": h, "format": "drawio", "output_path": "login.drawio" }
```

| Tool | Purpose |
|------|---------|
| `create_sequence` | New sequence diagram (optional title). Returns a sequence handle. |
| `close_sequence` | Free a sequence's memory. |
| `add_participant` | Add a lifeline (`actor: true` for a stick figure). |
| `add_message` | Add an ordered message; missing endpoints are auto-created. |
| `remove_message` | Delete a message by index. |
| `describe_sequence` | Title, participants, and ordered messages (with indexes). |
| `export_sequence` | Export to `drawio` (UML lifelines), `mermaid` (sequenceDiagram), `svg`, or `json`. |

Message `kind` is `sync` (default, solid + filled arrow), `async` (open arrow), `return` (dashed), `create`, or `destroy` (ends a lifeline). The **drawio** export uses real `umlLifeline`/`umlActor` shapes; **mermaid** emits a `sequenceDiagram`.

## Responses

```jsonc
// success
{ "status": "success", "message": "Added node", "data": { "id": "a", "node_count": 1 } }
// error
{ "status": "error", "category": "not_found", "message": "...", "suggestion": "..." }
```

## Build from source

```bash
cargo build --release
cargo test
```

## License

Apache-2.0
