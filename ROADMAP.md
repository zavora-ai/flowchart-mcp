# Roadmap — closing the draw.io gap

Top 20 features, scored by impact (closing the draw.io gap) vs effort.
Effort: S = <½ day · M = ~1 day · L = multi-day.

| # | Feature | Why it matters | Effort | Impact |
|----|---------|----------------|--------|--------|
| 1 | Manual position/size overrides (optional `x/y/w/h` per node) | Biggest flexibility gap; pin specifics over auto-layout | M | ★★★★★ — ✅ done (`move_node`) |
| 2 | Edge waypoints + fixed ports (`exitX/Y`, `entryX/Y`, `Array as="points"`) | Controls connector routing for clean complex diagrams | M | ★★★★★ — ✅ done (`route_edge`) |
| 3 | Expand stencil catalog (~37 → ~500 curated keys) | Real AWS/Azure/UML/network coverage | M | ★★★★★ — ✅ done (159 keys + raw passthrough) |
| 4 | PNG/PDF export (render SVG → raster/pdf) | Most-requested output; usable outside diagrams.net | M | ★★★★☆ — ✅ vector PDF done; PNG deferred (dep weight) |
| 5 | Layers (`parent="0"` layer cells, visibility) | Core draw.io structuring primitive | S | ★★★☆☆ — ✅ done (`add_layer`/`set_node_layer`) |
| 6 | HTML / rich-text labels (`<b>`,`<br>`,`<font>`) | Real formatting; needed for many shapes | S | ★★★★☆ — ✅ done (`html` flag) |
| 7 | True UML class shape (stackLayout compartments) | Replaces the `\n` fake | M | ★★★☆☆ — ✅ done (`uml_class` + compartments) |
| 8 | Gradients + glass/sketch styles | Raises aesthetic ceiling cheaply | S | ★★★☆☆ — ✅ done (`gradient`/`sketch`/`glass`) |
| 9 | Sequence diagrams (lifelines, activations, messages) | A whole diagram class we lack | L | ★★★★☆ — ✅ done (sequence subsystem, 7 tools) |
| 10 | State machine diagrams (start/end states, transitions) | Common UML type; builds on edges | M | ★★★☆☆ — ✅ done (`state_machine` + self-loops) |
| 11 | Mind-map / tree auto-layout (radial + tree) | Layered layout is wrong shape for these | M | ★★★☆☆ — ✅ done (`tree` / `mind_map`) |
| 12 | Tables / grids (`shape=table`, rows/cells) | UML, data, entity layouts | M | ★★★☆☆ |
| 13 | Edge label position + style (`x/y` offset, bg/border) | Labels currently auto-centered only | S | ★★★☆☆ — ✅ done (`label_edge`) |
| 14 | `update_edge` / `update_subgraph` / reorder | CRUD gaps — can't edit edges/containers | S | ★★★☆☆ — ✅ `update_edge` done |
| 15 | Connection-point-aware routing in layout | Fewer edge/box crossings | L | ★★★★☆ |
| 16 | Custom shape library import (stencil XML → SVG) | Stencils in our SVG/PNG, not just drawio | L | ★★★☆☆ |
| 17 | DOT / draw.io import (not just Mermaid) | Round-trip parity, two more formats | M | ★★☆☆☆ |
| 18 | Themes / palettes (named schemes, dark mode) | One-call professional styling | S | ★★★☆☆ — ✅ done (`apply_theme`, 6 palettes) |
| 19 | Metadata / links (`UserObject`, tooltips, hyperlinks) | Interactive/architecture diagrams | S | ★★☆☆☆ |
| 20 | VSDX (Visio) export | Enterprise interop | L | ★★☆☆☆ |

## Sequencing

- **Wave 1 (flexibility — the real gap):** #1, #2, #14 — manual overrides + waypoints/ports + edit-CRUD.
- **Wave 2 (breadth/usability):** #3, #4, #6 — full stencil catalog, PNG/PDF, HTML labels.
- **Wave 3 (new chart classes):** #7, #9, #10, #11 — UML class, sequence, state, mind-map.
- **Wave 4 (polish/interop):** #5, #8, #13, #18, #17, #16, #19, #20.
