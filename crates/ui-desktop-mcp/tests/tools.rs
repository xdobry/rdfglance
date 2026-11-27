use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, RwLock},
    time::Duration,
};

use egui::Context;
use rdf_glance::RdfGlanceApp;
use rdf_glance::domain::visual_query::FilterType;
use rdf_glance_desktop_mcp::{
    mcp::{MyMcpServer, UiCommand},
    tool_api::{Filter, dataset_profile_from_index},
    working_graph::{
        self, AddTypeInstancesParams, AlgorithmParams, DirectedPredicate, Direction, ExpandParams,
        GetStatisticsDataParams, NodeListParams, SetHiddenPredicatesParams, StatisticKind, StatisticsOrder,
    },
};
use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientConfig},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::default()
    }
}

#[tokio::test]
async fn get_dataset_profile_returns_loaded_app_profile() {
    let data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample-rdf-data/programming_languages.ttl");
    let data_path = data_path.to_str().expect("sample data path should be valid UTF-8");

    let mut app = RdfGlanceApp::new(None, vec![]);
    app.load_ttl(data_path, true);
    app.join_load(false);

    let expected_profile = {
        let rdf_data = app.rdf_data.read().expect("RDF data lock should be available");
        dataset_profile_from_index(&rdf_data, &app.type_index.type_instance_index)
    };
    assert!(expected_profile.nodes > 0);
    assert!(!expected_profile.types.is_empty());

    let dataset_profile = Arc::new(RwLock::new(expected_profile.clone()));
    let (ui_tx, _ui_rx) = mpsc::channel::<UiCommand>(1);
    let server = MyMcpServer::new(ui_tx, Context::default(), dataset_profile);
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server_handle = tokio::spawn(async move {
        server
            .serve(server_transport)
            .await
            .expect("MCP server should start")
            .waiting()
            .await
            .expect("MCP server should stop cleanly");
    });

    let client = TestClient
        .serve(client_transport)
        .await
        .expect("MCP client should connect");
    let result = client
        .call_tool(CallToolRequestParams::new("get_dataset_profile"))
        .await
        .expect("get_dataset_profile should succeed");

    assert_eq!(
        result.structured_content,
        Some(serde_json::to_value(expected_profile).expect("profile should serialize"))
    );

    client.cancel().await.expect("MCP client should cancel");
    server_handle.await.expect("MCP server task should finish");
}

fn loaded_app() -> RdfGlanceApp {
    let data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample-rdf-data/programming_languages.ttl");
    let mut app = RdfGlanceApp::new(None, vec![]);
    app.load_ttl(data_path.to_str().unwrap(), true);
    app.join_load(false);
    app.visible_nodes.lock_layout = true;
    app
}

#[test]
fn working_graph_adds_expands_runs_statistics_and_cleans() {
    let mut app = loaded_app();
    let initial = working_graph::add_nodes(
        &mut app,
        NodeListParams {
            nodes: vec!["http://dbpedia.org/resource/Niklaus_Wirth".into()],
        },
    )
    .unwrap();
    assert_eq!(initial.node_count, 1);
    assert_eq!(initial.edge_count, 0);
    assert!(
        initial
            .types
            .iter()
            .any(|entry| entry.iri == "dbo:Person" && entry.count == 1)
    );

    let expanded = working_graph::expand(
        &mut app,
        ExpandParams {
            nodes: vec!["dbr:Niklaus_Wirth".into()],
            types: vec!["dbo:ProgrammingLanguage".into()],
            predicates: vec![DirectedPredicate {
                predicate: "dbo:designer".into(),
                direction: Direction::Reverse,
            }],
        },
    )
    .unwrap();
    assert_eq!(expanded.node_count, 8);
    assert!(expanded.edge_count >= 7);
    assert!(
        expanded
            .predicates
            .iter()
            .any(|entry| entry.iri == "dbo:designer" && entry.count == 7)
    );

    let hidden = working_graph::set_hidden_predicates(
        &mut app,
        SetHiddenPredicatesParams {
            hidden_predicates: vec!["dbo:designer".into()],
        },
    )
    .unwrap();
    assert_eq!(hidden.hidden_edge_count, 7);
    assert_eq!(hidden.visible_edge_count + hidden.hidden_edge_count, hidden.edge_count);
    assert_eq!(hidden.hidden_predicates, vec!["dbo:designer"]);
    assert!(hidden.predicates.iter().any(|predicate| {
        predicate.iri == "dbo:designer"
            && predicate.count == 7
            && predicate.visible_count == 0
            && predicate.hidden_count == 7
            && predicate.hidden
    }));
    let shown = working_graph::set_hidden_predicates(
        &mut app,
        SetHiddenPredicatesParams {
            hidden_predicates: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(shown.hidden_edge_count, 0);
    assert_eq!(shown.visible_edge_count, shown.edge_count);
    assert!(shown.hidden_predicates.is_empty());

    let result = working_graph::run_algorithm(
        &mut app,
        AlgorithmParams {
            algorithm: StatisticKind::DegreeCentrality,
        },
    )
    .unwrap();
    assert_eq!(result.row_count, 8);
    assert_eq!(result.statistics, vec![StatisticKind::DegreeCentrality]);
    let highest = working_graph::get_statistics_data(
        &app,
        GetStatisticsDataParams {
            statistic: StatisticKind::DegreeCentrality,
            order: StatisticsOrder::Highest,
            limit: 3,
        },
    )
    .unwrap();
    assert_eq!(highest.total_rows, 8);
    assert_eq!(highest.returned_rows, 3);
    assert!(highest.values.iter().any(|entry| entry.iri == "dbr:Niklaus_Wirth"));
    assert!(highest.values.windows(2).all(|pair| pair[0].value >= pair[1].value));
    let lowest = working_graph::get_statistics_data(
        &app,
        GetStatisticsDataParams {
            statistic: StatisticKind::DegreeCentrality,
            order: StatisticsOrder::Lowest,
            limit: 8,
        },
    )
    .unwrap();
    assert!(lowest.values.windows(2).all(|pair| pair[0].value <= pair[1].value));
    assert_eq!(working_graph::statistics_info(&app).unwrap().row_count, 8);
    working_graph::reset_statistics(&mut app).unwrap();
    assert!(app.statistics_data.as_ref().unwrap().results.is_empty());
    assert!(!app.visualization_style.use_size_overwrite);
    assert!(!app.visualization_style.use_color_overwrite);
    let cleaned = working_graph::clean(&mut app).unwrap();
    assert_eq!((cleaned.node_count, cleaned.edge_count), (0, 0));
    assert!(!app.is_empty());
}

#[test]
fn working_graph_multi_add_and_expand_validate_inputs() {
    let mut app = loaded_app();
    let graph = working_graph::add_nodes(
        &mut app,
        NodeListParams {
            nodes: vec!["dbr:Niklaus_Wirth".into(), "dbr:Pascal_(programming_language)".into()],
        },
    )
    .unwrap();
    assert_eq!((graph.node_count, graph.edge_count), (2, 1));
    assert_eq!(app.visible_nodes.undo_stack.len(), 1);
    assert!(
        working_graph::add_nodes(
            &mut app,
            NodeListParams {
                nodes: vec!["dbr:Modula".into(), "urn:missing".into()],
            }
        )
        .is_err()
    );
    assert_eq!(working_graph::info(&app).unwrap().node_count, 2);
    let unchanged = working_graph::expand(
        &mut app,
        ExpandParams {
            nodes: vec!["dbr:Niklaus_Wirth".into()],
            types: vec![],
            predicates: vec![DirectedPredicate {
                predicate: "dbo:designer".into(),
                direction: Direction::Standard,
            }],
        },
    )
    .unwrap();
    assert_eq!(unchanged.node_count, 2);
}

#[test]
fn working_graph_adds_filtered_type_instances() {
    let mut app = loaded_app();
    let graph = working_graph::add_type_instances(
        &mut app,
        AddTypeInstancesParams {
            type_name: "dbo:ProgrammingLanguage".into(),
            filters: vec![Filter {
                attribute: "rdfs:label".into(),
                value: "Rust".into(),
                operator: FilterType::Equals,
            }],
        },
    )
    .unwrap();
    assert!(graph.node_count > 0);
    assert!(graph.types.iter().any(|entry| entry.iri == "dbo:ProgrammingLanguage"));
    let data = app.rdf_data.read().unwrap();
    let includes_rust = app.visible_nodes.nodes.read().unwrap().iter().any(|node| {
        data.node_data
            .get_node_by_index(node.node_index)
            .is_some_and(|(iri, _)| iri.contains("Rust"))
    });
    assert!(includes_rust, "Filtered type addition should include Rust");
}

struct DesktopClient {
    process: Child,
    messages: std::sync::mpsc::Receiver<serde_json::Value>,
    next_id: u64,
}

impl DesktopClient {
    fn new() -> Self {
        let mut process = Command::new(env!("CARGO_BIN_EXE_rdf-glance-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("desktop server should start");
        let stdout = process.stdout.take().unwrap();
        let (sender, messages) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("server output should be readable");
                let value = serde_json::from_str(&line).expect("server output should be JSON");
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        Self {
            process,
            messages,
            next_id: 1,
        }
    }

    fn send(&mut self, message: serde_json::Value) {
        let stdin = self.process.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn response(&mut self, id: u64) -> serde_json::Value {
        loop {
            let message = self
                .messages
                .recv_timeout(Duration::from_secs(30))
                .expect("server response timed out");
            if message["id"] == id {
                return message;
            }
        }
    }

    fn raw_request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        self.response(id)
    }

    fn request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let message = self.raw_request(method, params);
        assert!(message.get("error").is_none(), "MCP error: {message}");
        message["result"].clone()
    }

    fn call(&mut self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let result = self.request("tools/call", serde_json::json!({"name": name, "arguments": arguments}));
        assert_ne!(result["isError"], true, "tool failed: {result}");
        result["structuredContent"].clone()
    }
}

impl Drop for DesktopClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[test]
#[ignore = "requires a graphical desktop"]
fn desktop_stdio_load_reset_and_working_graph_tools() {
    let mut client = DesktopClient::new();
    client.request(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "integration-test", "version": "1.0"}
        }),
    );
    client.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let tools = client.request("tools/list", serde_json::json!({}));
    for name in [
        "load_data",
        "reset_data",
        "search",
        "wg_add_nodes",
        "wg_add_type_instances",
        "wg_set_hidden_predicates",
        "wg_expand",
        "wg_info",
        "wg_clean",
        "wg_run_algorithm",
        "wg_reset_statistics",
        "statistics_info",
        "get_statistics_data",
        "switch_display",
    ] {
        assert!(
            tools["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["name"] == name),
            "missing {name}"
        );
    }
    let empty = client.call("reset_data", serde_json::json!({}));
    assert_eq!(empty["nodes"], 0);
    assert_eq!(
        client.call("switch_display", serde_json::json!({"display_type": "statistics"})),
        "statistics"
    );
    let missing = client.raw_request(
        "tools/call",
        serde_json::json!({
            "name": "load_data", "arguments": {"file": "missing-rdfglance-data.ttl"}
        }),
    );
    assert!(missing.get("error").is_some());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample-rdf-data/programming_languages.ttl");
    let load_id = client.next_id;
    let graph_id = load_id + 1;
    client.next_id += 2;
    client.send(
        serde_json::json!({"jsonrpc": "2.0", "id": load_id, "method": "tools/call",
        "params": {"name": "load_data", "arguments": {"file": path.canonicalize().unwrap()}}}),
    );
    client.send(
        serde_json::json!({"jsonrpc": "2.0", "id": graph_id, "method": "tools/call",
        "params": {"name": "wg_info", "arguments": {}}}),
    );
    let mut loaded = None;
    let mut graph_info = None;
    while loaded.is_none() || graph_info.is_none() {
        let message = client
            .messages
            .recv_timeout(Duration::from_secs(30))
            .expect("queued response timed out");
        assert!(message.get("error").is_none(), "MCP error: {message}");
        if message["id"] == load_id {
            loaded = Some(message["result"]["structuredContent"].clone());
        }
        if message["id"] == graph_id {
            graph_info = Some(message["result"]["structuredContent"].clone());
        }
    }
    let loaded = loaded.unwrap();
    assert!(loaded["nodes"].as_u64().unwrap() > 0);
    assert_eq!(graph_info.unwrap()["node_count"], 0);
    let profile = client.call("get_dataset_profile", serde_json::json!({}));
    assert_eq!(profile["nodes"], loaded["nodes"]);
    let graph = client.call("wg_add_nodes", serde_json::json!({"nodes": ["dbr:Niklaus_Wirth"]}));
    assert_eq!(graph["node_count"], 1);
    let graph = client.call(
        "wg_expand",
        serde_json::json!({
            "types": ["dbo:ProgrammingLanguage"],
            "predicates": [{"predicate": "dbo:designer", "direction": "reverse"}]
        }),
    );
    assert_eq!(graph["node_count"], 8);
    let hidden = client.call(
        "wg_set_hidden_predicates",
        serde_json::json!({"hidden_predicates": ["dbo:designer"]}),
    );
    assert_eq!(hidden["hidden_edge_count"], 7);
    assert_eq!(hidden["visible_edge_count"].as_u64().unwrap() + 7, hidden["edge_count"]);
    assert_eq!(hidden["hidden_predicates"][0], "dbo:designer");
    let shown = client.call("wg_set_hidden_predicates", serde_json::json!({"hidden_predicates": []}));
    assert_eq!(shown["hidden_edge_count"], 0);
    assert_eq!(shown["visible_edge_count"], shown["edge_count"]);
    let result = client.call(
        "wg_run_algorithm",
        serde_json::json!({"algorithm": "degree_centrality"}),
    );
    assert_eq!(result["row_count"], 8);
    assert_eq!(result["statistics"][0], "degree_centrality");
    assert_eq!(client.call("statistics_info", serde_json::json!({})), result);
    let data = client.call(
        "get_statistics_data",
        serde_json::json!({"statistic": "degree_centrality", "order": "highest", "limit": 8}),
    );
    assert_eq!(data["total_rows"], 8);
    assert_eq!(data["values"].as_array().unwrap().len(), 8);
    client.call("wg_reset_statistics", serde_json::json!({}));
    assert_eq!(client.call("wg_info", serde_json::json!({}))["node_count"], 8);
    assert_eq!(client.call("wg_clean", serde_json::json!({}))["node_count"], 0);
    let typed = client.call(
        "wg_add_type_instances",
        serde_json::json!({
            "type_name": "dbo:ProgrammingLanguage",
            "filters": [{"attribute": "rdfs:label", "operator": "Equals", "value": "Rust"}]
        }),
    );
    assert!(typed["node_count"].as_u64().unwrap() > 0);
    assert!(
        typed["types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["iri"] == "dbo:ProgrammingLanguage")
    );
    assert_eq!(client.call("wg_clean", serde_json::json!({}))["node_count"], 0);
    assert_eq!(client.call("reset_data", serde_json::json!({}))["nodes"], 0);
    assert_eq!(client.call("get_dataset_profile", serde_json::json!({}))["nodes"], 0);
}
