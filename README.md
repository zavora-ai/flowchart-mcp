# flowchart-mcp-server

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)

20 MCP tools for authoring diagrams and exporting them to **draw.io** (diagrams.net mxGraph XML), **Mermaid**, **Graphviz DOT**, **SVG**, and **JSON** — plus **Mermaid import**. Multi-page documents, swimlanes & nested containers, rich node/edge styling, node images, **draw.io stencil libraries** (AWS/Azure/GCP/network/Kubernetes/UML/BPMN), and ready-made chart types (flowchart, swimlane, org chart, mind map, UML class, ERD, BPMN). Pure Rust, local-first, no external services.

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
| `add_edge` | Connect two nodes (line, label, arrowheads, routing, color). |
| `style_edge` | Set start/end arrowheads, routing, and color on an existing edge. |
| `remove_edge` | Delete an edge by index. |
| `set_direction` | Change the current page's flow direction. |
| `add_subgraph` | Group nodes into a container (group/container/swimlane/pool, optionally nested). |
| `add_page` | Add a page to the document and select it. |
| `select_page` | Select the active page by index. |
| `export_flowchart` | Export to `drawio`, `mermaid`, `dot`, `svg`, or `json`. |
| `import_mermaid` | Parse Mermaid flowchart text into a new document. |

## Shapes

`rectangle` · `round_rect` · `stadium` · `subroutine` · `cylinder` · `circle` · `double_circle` · `diamond` · `hexagon` · `parallelogram` · `parallelogram_alt` · `trapezoid` · `trapezoid_alt` · `note` · `card` · `document`

Each shape maps to the closest native primitive in every export target (e.g. `diamond` → `rhombus` in draw.io, `diamond` in DOT, a polygon in SVG, `{...}` in Mermaid). Shapes without a native Mermaid form fall back to a rectangle there.

## Stencil libraries

Any node can use a built-in **draw.io stencil** instead of a primitive shape, giving access to the AWS, Azure, GCP, Cisco/network, Kubernetes, UML, BPMN, and mockup icon sets. Set one with `set_node_stencil` or the `stencil` field on `add_node`:

```jsonc
add_node { "handle": h, "id": "api", "label": "API", "stencil": "aws.api_gateway" }
add_node { "handle": h, "id": "db",  "label": "Users", "stencil": "aws.rds" }
```

`list_stencils { category?, query? }` browses the curated catalog (friendly keys like `aws.ec2`, `k8s.pod`, `net.firewall`, `uml.actor`). Beyond the catalog, you can pass any raw `mxgraph.<lib>.<name>` token — e.g. `"mxgraph.azure.load_balancer"` — and it is emitted verbatim.

Stencils render with full fidelity in the **drawio** export (open in diagrams.net). The **svg**/**dot**/**mermaid** exports show a labeled placeholder box instead, since the stencil artwork lives inside draw.io.

## Styling

`style_node` (and the style fields on `add_node`) accept: `fill`, `stroke`, `text_color` (hex), `stroke_width`, `font_family`, `font_size`, `bold`, `italic`, `align` (`left`/`center`/`right`), `opacity` (0–100), `rounded`, `shadow`, `dashed`. Only provided fields change.

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

## Export formats

- **drawio** — diagrams.net mxGraph XML with computed geometry, per-shape styles, rich fonts/opacity/shadows, image shapes, arrowheads on both ends, edge routing, nested containers/swimlanes, and one page per document page. Open directly in [app.diagrams.net](https://app.diagrams.net).
- **mermaid** — `flowchart` source with shapes, edge labels, line styles, subgraphs, and `style` directives.
- **dot** — Graphviz `digraph` with `rankdir`, clusters, shape mapping, and edge attributes. Render with `dot -Tpng`.
- **svg** — self-contained SVG using the auto-layout, with arrowheads, dashed/thick edges, images, containers, and shape outlines.
- **json** — the raw serialized document (all pages) for programmatic round-tripping.

With `output_path` the content is written to disk; otherwise it is returned inline under `data.content`.

## Templates

`create_flowchart` accepts a `template` id to pre-populate the document:

```
basic · decision · approval · etl · swimlane · org_chart · mind_map · uml_class · erd · bpmn
```

## Responses

Every tool returns a structured JSON string:

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
