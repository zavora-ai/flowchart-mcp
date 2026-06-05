//! End-to-end checkpoint: drive the server's tools over MCP stdio and assert
//! the round-trip create → edit → export → import behaves.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Server {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    id: i64,
}

impl Server {
    fn start() -> Self {
        let bin = env!("CARGO_BIN_EXE_flowchart-mcp-server");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn server");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, out, id: 0 };
        s.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2024-11-05","capabilities":{},
            "clientInfo":{"name":"t","version":"1"}}}));
        s.read_id(1);
        s.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        s.id = 1;
        s
    }

    fn send(&mut self, v: Value) {
        writeln!(self.stdin, "{v}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_id(&mut self, id: i64) -> Value {
        let mut line = String::new();
        loop {
            line.clear();
            self.out.read_line(&mut line).unwrap();
            let m: Value = serde_json::from_str(line.trim()).unwrap();
            if m.get("id") == Some(&json!(id)) {
                return m;
            }
        }
    }

    /// Call a tool and return the parsed JSON of its first text content.
    fn call(&mut self, name: &str, args: Value) -> Value {
        self.id += 1;
        let id = self.id;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":args}}));
        let resp = self.read_id(id);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("tool returned text content");
        serde_json::from_str(text).expect("tool text is JSON")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn full_lifecycle() {
    let mut s = Server::start();

    // tools/list exposes all 14 tools.
    s.id += 1;
    let id = s.id;
    s.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":{}}));
    let list = s.read_id(id);
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 39, "expected 39 tools");

    // Create a flowchart.
    let created = s.call("create_flowchart", json!({"direction":"TB","title":"Pipeline"}));
    assert_eq!(created["status"], "success");
    let handle = created["data"]["handle"].as_str().unwrap().to_string();

    // Build a small decision graph.
    s.call("add_node", json!({"handle":handle,"id":"a","label":"Start","shape":"stadium"}));
    s.call("add_node", json!({"handle":handle,"id":"b","label":"OK?","shape":"diamond"}));
    s.call("add_node", json!({"handle":handle,"id":"c","label":"Done","shape":"stadium","fill":"#D5E8D4"}));
    let edge = s.call("add_edge", json!({"handle":handle,"from":"a","to":"b"}));
    assert_eq!(edge["status"], "success");
    s.call("add_edge", json!({"handle":handle,"from":"b","to":"c","label":"yes","line":"dotted"}));

    // Describe and verify counts.
    let desc = s.call("describe_flowchart", json!({"handle":handle}));
    assert_eq!(desc["data"]["node_count"], 3);
    assert_eq!(desc["data"]["edge_count"], 2);

    // Export to draw.io inline and check it is mxGraph XML.
    let drawio = s.call("export_flowchart", json!({"handle":handle,"format":"drawio"}));
    let xml = drawio["data"]["content"].as_str().unwrap();
    assert!(xml.contains("<mxfile"));
    assert!(xml.contains("rhombus")); // the diamond node

    // Export to a temp file.
    let dir = std::env::temp_dir();
    let path = dir.join("flowchart_checkpoint.drawio");
    let p = path.to_string_lossy().to_string();
    let saved = s.call("export_flowchart", json!({"handle":handle,"format":"drawio","output_path":p}));
    assert_eq!(saved["status"], "success");
    assert!(path.exists());
    let _ = std::fs::remove_file(&path);

    // Export to Mermaid, then re-import it and confirm the structure survives.
    let mermaid = s.call("export_flowchart", json!({"handle":handle,"format":"mermaid"}));
    let mmd = mermaid["data"]["content"].as_str().unwrap().to_string();
    assert!(mmd.starts_with("flowchart TD"));

    let imported = s.call("import_mermaid", json!({"source":mmd}));
    assert_eq!(imported["status"], "success");
    assert_eq!(imported["data"]["node_count"], 3);
    assert_eq!(imported["data"]["edge_count"], 2);

    // JSON export round-trips the document (pages[0] holds the chart).
    let as_json = s.call("export_flowchart", json!({"handle":handle,"format":"json"}));
    let content = as_json["data"]["content"].as_str().unwrap();
    let model: Value = serde_json::from_str(content).unwrap();
    assert_eq!(model["pages"][0]["nodes"].as_array().unwrap().len(), 3);

    // Error path: unknown handle.
    let err = s.call("describe_flowchart", json!({"handle":"nope"}));
    assert_eq!(err["status"], "error");
    assert_eq!(err["category"], "not_found");

    // Close.
    let closed = s.call("close_flowchart", json!({"handle":handle}));
    assert_eq!(closed["status"], "success");
}

#[test]
fn containers_pages_arrows_images() {
    let mut s = Server::start();

    // Pool + two swimlanes with a node in each lane.
    let created = s.call("create_flowchart", json!({"direction":"TB"}));
    let handle = created["data"]["handle"].as_str().unwrap().to_string();
    s.call("add_node", json!({"handle":handle,"id":"x","label":"X","shape":"note"}));
    s.call("add_node", json!({"handle":handle,"id":"y","label":"Y","image":"https://example.com/i.png"}));
    s.call("add_edge", json!({"handle":handle,"from":"x","to":"y","end_arrow":"diamond","start_arrow":"oval","routing":"curved","color":"#FF0000"}));
    let pool = s.call("add_subgraph", json!({"handle":handle,"id":"pool","label":"Pool","members":[],"kind":"pool"}));
    assert_eq!(pool["status"], "success");
    let lane = s.call("add_subgraph", json!({"handle":handle,"id":"lane1","label":"Lane 1","members":["x","y"],"kind":"swimlane","parent":"pool"}));
    assert_eq!(lane["status"], "success");

    // Second page.
    let page = s.call("add_page", json!({"handle":handle,"name":"Details","direction":"LR"}));
    assert_eq!(page["data"]["page_index"], 1);
    s.call("add_node", json!({"handle":handle,"id":"p2","label":"P2","shape":"card"}));

    // draw.io export carries both pages, the swimlane, nested parent, arrowheads, image.
    let drawio = s.call("export_flowchart", json!({"handle":handle,"format":"drawio"}));
    let xml = drawio["data"]["content"].as_str().unwrap();
    assert_eq!(xml.matches("<diagram").count(), 2);
    assert!(xml.contains("swimlane"));
    // Swimlanes now render as full-length bands at the root layer (lane label
    // present), rather than nesting nodes inside the pool cell.
    assert!(xml.contains("value=\"Lane 1\""));
    assert!(xml.contains("endArrow=diamond"));
    assert!(xml.contains("startArrow=oval"));
    assert!(xml.contains("shape=image"));

    // describe reports pages + container kind.
    s.call("select_page", json!({"handle":handle,"index":0}));
    let desc = s.call("describe_flowchart", json!({"handle":handle}));
    assert_eq!(desc["data"]["pages"].as_array().unwrap().len(), 2);
    assert!(desc["data"]["subgraphs"].as_array().unwrap().iter().any(|s| s["kind"] == "pool"));

    // A new chart-type template builds and exports crow's-foot ERD arrows.
    let erd = s.call("create_flowchart", json!({"template":"erd"}));
    let eh = erd["data"]["handle"].as_str().unwrap().to_string();
    let ex = s.call("export_flowchart", json!({"handle":eh,"format":"drawio"}));
    assert!(ex["data"]["content"].as_str().unwrap().contains("ERmany"));

    // Stencils: catalog discovery + applying an AWS stencil emits the resIcon token.
    let st = s.call("list_stencils", json!({"category":"aws"}));
    assert!(st["data"]["count"].as_u64().unwrap() >= 5);
    let sten = s.call("create_flowchart", json!({}));
    let sh = sten["data"]["handle"].as_str().unwrap().to_string();
    s.call("add_node", json!({"handle":sh,"id":"db","label":"Users","stencil":"aws.rds"}));
    s.call("add_node", json!({"handle":sh,"id":"fn","label":"Handler"}));
    let img = s.call("set_node_stencil", json!({"handle":sh,"id":"fn","stencil":"aws.lambda"}));
    assert_eq!(img["status"], "success");
    let sx = s.call("export_flowchart", json!({"handle":sh,"format":"drawio"}));
    let sxml = sx["data"]["content"].as_str().unwrap();
    assert!(sxml.contains("resIcon=mxgraph.aws4.rds"));
    assert!(sxml.contains("resIcon=mxgraph.aws4.lambda"));

    s.call("close_flowchart", json!({"handle":handle}));
    s.call("close_flowchart", json!({"handle":eh}));
    s.call("close_flowchart", json!({"handle":sh}));
}

#[test]
fn build_document_and_export_pages() {
    let mut s = Server::start();

    // Build a 2-page document in a single call, with swimlanes on page 1.
    let spec = json!({
        "direction": "LR",
        "pages": [
            {
                "name": "Manifest",
                "title": "1. Manifest Capture",
                "lanes": ["Manifest Team", "System"],
                "nodes": [
                    { "id": "s",   "label": "Start", "shape": "stadium",  "lane": "Manifest Team" },
                    { "id": "dec", "label": "Consolidated?", "shape": "diamond", "lane": "Manifest Team" },
                    { "id": "job", "label": "Job File No.", "shape": "document", "lane": "System" },
                    { "id": "e",   "label": "End", "shape": "stadium", "lane": "Manifest Team" }
                ],
                "edges": [
                    { "from": "s", "to": "dec" },
                    { "from": "dec", "to": "job", "label": "Yes" },
                    { "from": "job", "to": "e" }
                ]
            },
            {
                "name": "Exit",
                "title": "2. Exit",
                "nodes": [
                    { "id": "a", "label": "A", "shape": "stadium" },
                    { "id": "b", "label": "B" }
                ],
                "edges": [ { "from": "a", "to": "b" } ]
            }
        ]
    });
    let built = s.call("build_document", spec);
    assert_eq!(built["status"], "success");
    assert_eq!(built["data"]["page_count"], 2);
    assert_eq!(built["data"]["node_count"], 6);
    let handle = built["data"]["handle"].as_str().unwrap().to_string();

    // Combined export carries both pages, the title banner, and a swimlane band.
    let combined = s.call("export_flowchart", json!({"handle":handle,"format":"drawio"}));
    let xml = combined["data"]["content"].as_str().unwrap();
    assert_eq!(xml.matches("<diagram").count(), 2);
    assert!(xml.contains("1. Manifest Capture"));
    assert!(xml.contains("swimlane"));
    assert!(xml.contains("value=\"Manifest Team\""));

    // Per-page export writes one file per page into a temp dir.
    let dir = std::env::temp_dir().join(format!("fmcp_pages_{}", std::process::id()));
    let dir_s = dir.to_string_lossy().to_string();
    let pages = s.call("export_pages", json!({
        "handle": handle, "format": "drawio", "output_dir": dir_s
    }));
    assert_eq!(pages["status"], "success");
    assert_eq!(pages["data"]["count"], 2);
    let files = pages["data"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    for f in files {
        let p = f.as_str().unwrap();
        assert!(std::path::Path::new(p).exists(), "page file missing: {p}");
    }
    // Default pattern is {index}-{name}.{ext}
    assert!(files[0].as_str().unwrap().contains("01-Manifest.drawio"));
    let _ = std::fs::remove_dir_all(&dir);

    s.call("close_flowchart", json!({"handle":handle}));
}

#[test]
fn build_document_rejects_bad_lane() {
    let mut s = Server::start();
    let spec = json!({
        "pages": [{
            "lanes": ["A"],
            "nodes": [ { "id": "x", "label": "X", "lane": "B" } ],
            "edges": []
        }]
    });
    let res = s.call("build_document", spec);
    assert_eq!(res["status"], "error");
    assert_eq!(res["category"], "invalid_input");
}

#[test]
fn json_round_trip() {
    let mut s = Server::start();

    // Build, export to JSON, re-import, and confirm structure survives.
    let built = s.call("build_document", json!({
        "direction": "LR",
        "pages": [{
            "name": "P1",
            "nodes": [
                { "id": "a", "label": "Start", "shape": "stadium" },
                { "id": "b", "label": "Work" }
            ],
            "edges": [ { "from": "a", "to": "b", "label": "go" } ]
        }]
    }));
    let h1 = built["data"]["handle"].as_str().unwrap().to_string();
    let j = s.call("export_flowchart", json!({"handle":h1,"format":"json"}));
    let doc_json = j["data"]["content"].as_str().unwrap().to_string();

    let imported = s.call("import_json", json!({"json": doc_json}));
    assert_eq!(imported["status"], "success");
    assert_eq!(imported["data"]["node_count"], 2);
    let h2 = imported["data"]["handle"].as_str().unwrap().to_string();

    // The re-imported doc exports the same node labels.
    let desc = s.call("describe_flowchart", json!({"handle":h2}));
    let nodes = desc["data"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    s.call("close_flowchart", json!({"handle":h1}));
    s.call("close_flowchart", json!({"handle":h2}));
}

#[test]
fn build_document_rejects_unlabeled_decision_branches() {
    let mut s = Server::start();
    // A diamond with two unlabeled branches must be rejected.
    let spec = json!({
        "pages": [{
            "nodes": [
                { "id": "d", "label": "Full / half / empty?", "shape": "diamond" },
                { "id": "a", "label": "A" },
                { "id": "b", "label": "B" }
            ],
            "edges": [
                { "from": "d", "to": "a" },
                { "from": "d", "to": "b" }
            ]
        }]
    });
    let res = s.call("build_document", spec);
    assert_eq!(res["status"], "error");
    assert_eq!(res["category"], "invalid_input");

    // The same decision with labels on every branch is accepted (3-way).
    let ok = json!({
        "pages": [{
            "nodes": [
                { "id": "d", "label": "Container state?", "shape": "diamond" },
                { "id": "f", "label": "Full" },
                { "id": "h", "label": "Half" },
                { "id": "e", "label": "Empty" }
            ],
            "edges": [
                { "from": "d", "to": "f", "label": "full" },
                { "from": "d", "to": "h", "label": "half" },
                { "from": "d", "to": "e", "label": "empty" }
            ]
        }]
    });
    let res2 = s.call("build_document", ok);
    assert_eq!(res2["status"], "success");
    s.call("close_flowchart", json!({"handle": res2["data"]["handle"].as_str().unwrap()}));
}

#[test]
fn validate_flowchart_reports_properties() {
    let mut s = Server::start();
    // A clean 3-way decision built via build_document validates cleanly.
    let built = s.call("build_document", json!({
        "direction": "LR",
        "pages": [{
            "name": "Branch",
            "lanes": ["Ops"],
            "nodes": [
                { "id": "s",  "label": "Start", "shape": "stadium", "lane": "Ops" },
                { "id": "d",  "label": "Route?", "shape": "diamond", "lane": "Ops" },
                { "id": "a",  "label": "A", "lane": "Ops" },
                { "id": "b",  "label": "B", "lane": "Ops" },
                { "id": "c",  "label": "C", "lane": "Ops" },
                { "id": "m",  "label": "Merge", "lane": "Ops" },
                { "id": "e",  "label": "End", "shape": "stadium", "lane": "Ops" }
            ],
            "edges": [
                { "from": "s", "to": "d" },
                { "from": "d", "to": "a", "label": "x" },
                { "from": "d", "to": "b", "label": "y" },
                { "from": "d", "to": "c", "label": "z" },
                { "from": "a", "to": "m" },
                { "from": "b", "to": "m" },
                { "from": "c", "to": "m" },
                { "from": "m", "to": "e" }
            ]
        }]
    }));
    let h = built["data"]["handle"].as_str().unwrap().to_string();
    let rep = s.call("validate_flowchart", json!({"handle": h}));
    assert_eq!(rep["status"], "success");
    assert_eq!(rep["data"]["valid"], true, "report: {rep}");
    assert_eq!(rep["data"]["violation_count"], 0);
    s.call("close_flowchart", json!({"handle": h}));
}

#[test]
fn manual_overrides_and_edit_crud() {
    let mut s = Server::start();
    let created = s.call("create_flowchart", json!({"direction":"TB"}));
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    s.call("add_node", json!({"handle":h,"id":"a","label":"A","shape":"stadium"}));
    s.call("add_node", json!({"handle":h,"id":"b","label":"B"}));
    let e = s.call("add_edge", json!({"handle":h,"from":"a","to":"b","label":"go",
        "exit":[1.0,0.5],"entry":[0.0,0.5],"waypoints":[[400.0,100.0],[400.0,200.0]]}));
    assert_eq!(e["status"], "success");

    // Manual node placement overrides auto-layout.
    let mv = s.call("move_node", json!({"handle":h,"id":"b","x":900.0,"y":600.0,"w":200.0,"h":80.0}));
    assert_eq!(mv["status"], "success");
    let desc = s.call("describe_flowchart", json!({"handle":h}));
    let bnode = desc["data"]["nodes"].as_array().unwrap().iter().find(|n| n["id"]=="b").unwrap();
    assert_eq!(bnode["pos"][0], 900.0);
    assert_eq!(bnode["size"][1], 80.0);

    // Edit-CRUD on the edge.
    let ue = s.call("update_edge", json!({"handle":h,"index":0,"label":"renamed","line":"thick"}));
    assert_eq!(ue["status"], "success");

    // draw.io export carries ports, waypoints, and the manual box.
    let xml = s.call("export_flowchart", json!({"handle":h,"format":"drawio"}));
    let x = xml["data"]["content"].as_str().unwrap();
    assert!(x.contains("exitX=1;exitY=0.5"));
    assert!(x.contains("<Array as=\"points\">"));
    assert!(x.contains("x=\"900\""));
    assert!(x.contains("value=\"renamed\""));

    // route_edge clear resets manual routing.
    let rc = s.call("route_edge", json!({"handle":h,"index":0,"clear":true}));
    assert_eq!(rc["status"], "success");
    let xml2 = s.call("export_flowchart", json!({"handle":h,"format":"drawio"}));
    assert!(!xml2["data"]["content"].as_str().unwrap().contains("<Array as=\"points\">"));

    s.call("close_flowchart", json!({"handle":h}));
}

#[test]
fn wave2_stencils_html_pdf() {
    let mut s = Server::start();

    // Expanded stencil catalog: aws category now has many entries.
    let aws = s.call("list_stencils", json!({"category":"aws"}));
    assert!(aws["data"]["count"].as_u64().unwrap() >= 20, "aws count: {}", aws["data"]["count"]);
    let k8s = s.call("list_stencils", json!({"query":"statefulset"}));
    assert!(k8s["data"]["count"].as_u64().unwrap() >= 1);

    let created = s.call("create_flowchart", json!({"direction":"TB"}));
    let h = created["data"]["handle"].as_str().unwrap().to_string();

    // HTML rich-text label: rendered in drawio, stripped in mermaid.
    s.call("add_node", json!({"handle":h,"id":"a","label":"<b>Build</b><br>stage","html":true}));
    s.call("add_node", json!({"handle":h,"id":"b","label":"Cache","stencil":"k8s.sts"}));
    s.call("add_edge", json!({"handle":h,"from":"a","to":"b"}));

    let xml = s.call("export_flowchart", json!({"handle":h,"format":"drawio"}));
    let x = xml["data"]["content"].as_str().unwrap();
    assert!(x.contains("&lt;b&gt;Build&lt;/b&gt;"));
    assert!(x.contains("mxgraph.kubernetes.sts"));

    let mmd = s.call("export_flowchart", json!({"handle":h,"format":"mermaid"}));
    assert!(mmd["data"]["content"].as_str().unwrap().contains("Build stage"));

    // PDF export requires output_path and produces a valid PDF file.
    let noerr = s.call("export_flowchart", json!({"handle":h,"format":"pdf"}));
    assert_eq!(noerr["status"], "error");
    let path = std::env::temp_dir().join("flowchart_wave2.pdf");
    let p = path.to_string_lossy().to_string();
    let pdf = s.call("export_flowchart", json!({"handle":h,"format":"pdf","output_path":p}));
    assert_eq!(pdf["status"], "success");
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"%PDF-1.4"));
    let _ = std::fs::remove_file(&path);

    s.call("close_flowchart", json!({"handle":h}));
}

#[test]
fn wave3_uml_tree_state() {
    let mut s = Server::start();

    // UML class node with compartments, on a tree layout.
    let created = s.call("create_flowchart", json!({"direction":"TB","layout":"tree"}));
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    s.call("add_node", json!({"handle":h,"id":"u","label":"User","shape":"uml_class",
        "compartments":[["+ id: int","+ name: String"],["+ login(): void"]]}));
    s.call("add_node", json!({"handle":h,"id":"a","label":"Admin","shape":"uml_class",
        "compartments":[["+ role: String"],["+ grant(): void"]]}));
    s.call("add_edge", json!({"handle":h,"from":"u","to":"a","end_arrow":"block","arrow":false}));
    // Self-loop transition.
    let loop_e = s.call("add_edge", json!({"handle":h,"from":"a","to":"a","label":"audit"}));
    assert_eq!(loop_e["status"], "success");

    let desc = s.call("describe_flowchart", json!({"handle":h}));
    assert_eq!(desc["data"]["layout"], "tree");
    let unode = desc["data"]["nodes"].as_array().unwrap().iter().find(|n| n["id"]=="u").unwrap();
    assert_eq!(unode["compartments"].as_array().unwrap().len(), 2);

    let xml = s.call("export_flowchart", json!({"handle":h,"format":"drawio"}));
    let x = xml["data"]["content"].as_str().unwrap();
    assert!(x.contains("&lt;b&gt;User&lt;/b&gt;"));
    assert!(x.contains("+ login(): void"));
    assert!(x.contains("loopEdgeStyle"));

    // set_layout switches the algorithm.
    let sl = s.call("set_layout", json!({"handle":h,"layout":"mind_map"}));
    assert_eq!(sl["data"]["layout"], "mind_map");

    // state_machine template builds with a self-loop edge.
    let sm = s.call("create_flowchart", json!({"template":"state_machine"}));
    let smh = sm["data"]["handle"].as_str().unwrap().to_string();
    let smx = s.call("export_flowchart", json!({"handle":smh,"format":"drawio"}));
    assert!(smx["data"]["content"].as_str().unwrap().contains("loopEdgeStyle"));

    s.call("close_flowchart", json!({"handle":h}));
    s.call("close_flowchart", json!({"handle":smh}));
}

#[test]
fn wave4_themes_layers_styles() {
    let mut s = Server::start();

    // Theme on create.
    let created = s.call("create_flowchart", json!({"direction":"TB","theme":"purple"}));
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    s.call("add_node", json!({"handle":h,"id":"a","label":"A","gradient":"#B5739D","sketch":true,"glass":true}));
    s.call("add_node", json!({"handle":h,"id":"b","label":"B"}));

    // Layer + assignment.
    let al = s.call("add_layer", json!({"handle":h,"id":"bg","label":"Background","visible":false}));
    assert_eq!(al["status"], "success");
    let sl = s.call("set_node_layer", json!({"handle":h,"id":"b","layer":"bg"}));
    assert_eq!(sl["status"], "success");

    // Edge label positioning.
    let e = s.call("add_edge", json!({"handle":h,"from":"a","to":"b","label":"go"}));
    assert_eq!(e["status"], "success");
    let le = s.call("label_edge", json!({"handle":h,"index":0,"pos":-0.5,"offset":10.0,"bg":"#FFFFCC","border":"#D6B656"}));
    assert_eq!(le["status"], "success");

    // apply_theme re-themes the whole page.
    let at = s.call("apply_theme", json!({"handle":h,"theme":"green"}));
    assert_eq!(at["status"], "success");
    assert!(at["data"]["nodes_restyled"].as_u64().unwrap() >= 2);

    // describe reports layers.
    let desc = s.call("describe_flowchart", json!({"handle":h}));
    assert_eq!(desc["data"]["layers"].as_array().unwrap().len(), 1);

    // drawio export carries gradient/sketch/glass, hidden layer, label bg.
    let xml = s.call("export_flowchart", json!({"handle":h,"format":"drawio"}));
    let x = xml["data"]["content"].as_str().unwrap();
    assert!(x.contains("gradientColor=#B5739D"));
    assert!(x.contains("sketch=1"));
    assert!(x.contains("visible=\"0\""));
    assert!(x.contains("parent=\"bg\""));
    assert!(x.contains("labelBackgroundColor=#FFFFCC"));
    assert!(x.contains("fillColor=#D5E8D4")); // green theme

    s.call("close_flowchart", json!({"handle":h}));
}

#[test]
fn sequence_diagram_lifecycle() {
    let mut s = Server::start();

    let created = s.call("create_sequence", json!({"title":"Login"}));
    assert_eq!(created["status"], "success");
    let h = created["data"]["handle"].as_str().unwrap().to_string();

    s.call("add_participant", json!({"handle":h,"id":"u","label":"User","actor":true}));
    s.call("add_participant", json!({"handle":h,"id":"api","label":"API"}));
    let m = s.call("add_message", json!({"handle":h,"from":"u","to":"api","label":"POST /login","kind":"sync"}));
    assert_eq!(m["data"]["index"], 0);
    s.call("add_message", json!({"handle":h,"from":"api","to":"db","label":"query","kind":"sync"})); // db auto-created
    s.call("add_message", json!({"handle":h,"from":"api","to":"u","label":"200 OK","kind":"return"}));
    s.call("add_message", json!({"handle":h,"from":"api","to":"api","label":"audit","kind":"async"}));

    let desc = s.call("describe_sequence", json!({"handle":h}));
    assert_eq!(desc["data"]["participant_count"], 3); // u, api, db
    assert_eq!(desc["data"]["message_count"], 4);

    // drawio export has UML lifelines + message edges.
    let dx = s.call("export_sequence", json!({"handle":h,"format":"drawio"}));
    let x = dx["data"]["content"].as_str().unwrap();
    assert!(x.contains("umlActor"));
    assert!(x.contains("umlLifeline"));
    assert!(x.contains("endArrow=block")); // sync
    assert!(x.contains("value=\"POST /login\""));

    // mermaid sequenceDiagram.
    let mm = s.call("export_sequence", json!({"handle":h,"format":"mermaid"}));
    let mmd = mm["data"]["content"].as_str().unwrap();
    assert!(mmd.starts_with("sequenceDiagram"));
    assert!(mmd.contains("u->>api: POST /login"));

    // json round-trip shape.
    let js = s.call("export_sequence", json!({"handle":h,"format":"json"}));
    let model: Value = serde_json::from_str(js["data"]["content"].as_str().unwrap()).unwrap();
    assert_eq!(model["messages"].as_array().unwrap().len(), 4);

    // remove a message.
    let rm = s.call("remove_message", json!({"handle":h,"index":3}));
    assert_eq!(rm["data"]["message_count"], 3);

    // Sequence handles are a separate space — describe_flowchart on it fails.
    let cross = s.call("describe_flowchart", json!({"handle":h}));
    assert_eq!(cross["status"], "error");

    s.call("close_sequence", json!({"handle":h}));
}
