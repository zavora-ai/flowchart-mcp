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
    assert_eq!(tools.len(), 20, "expected 20 tools");

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
    assert!(xml.contains("parent=\"pool\""));
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
