use super::*;

#[test]
fn graph_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "graph", "fixtures/e2e/hello.orv"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn graph_view_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "graph",
        "fixtures/e2e/hello.orv",
        "--view",
        "--out",
        "target/orv-graph-view",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn reveal_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "reveal",
        "target/orv-build-test",
        "route:GET_/ping:abc123",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn reveal_origin_links_static_html_to_page_output() {
    let out = temp_output_dir("reveal-static-html");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, r#"@out @html { @body { @h1 "Home" } }"#).expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let html = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "domain" && entry.name == "html")
        .expect("html origin");

    let reveal = reveal_origin_json(&build_out, &html.id).expect("reveal html origin");

    assert_eq!(reveal["origin"]["kind"], "domain");
    assert_eq!(reveal["origin"]["name"], "html");
    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@html")));
    let static_targets = reveal["production"]["static"]
        .as_array()
        .expect("static targets");
    assert!(static_targets.iter().any(|target| {
        target["kind"] == "static_page"
            && target["path"] == "pages/index.html"
            && target["exists"] == true
            && target["verified"] == true
            && target["runtime_features"]
                .as_array()
                .expect("runtime features")
                .is_empty()
    }));
    assert_eq!(reveal["production"]["summary"]["static_target_count"], 1);
    assert_eq!(reveal["production"]["summary"]["static_verified_count"], 1);
    let lsp_reveal = lsp_reveal_json(&build_out, &html.id).expect("lsp reveal html origin");
    assert_eq!(
        lsp_reveal["production"]["summary"]["static_target_count"],
        1
    );
    assert_eq!(
        lsp_reveal["production"]["summary"]["static_verified_count"],
        1
    );
    let editor_reveal =
        editor_reveal_json(&build_out, &html.id).expect("editor reveal html origin");
    assert_eq!(
        editor_reveal["production"]["summary"]["static_target_count"],
        1
    );
    assert_eq!(
        editor_reveal["production"]["summary"]["static_verified_count"],
        1
    );
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_links_route_html_to_containing_route_output() {
    let dir = temp_output_dir("reveal-route-html-source");
    std::fs::create_dir_all(&dir).expect("create route html source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route GET / {
    @serve @html {
      @body { @h1 "Home" }
    }
  }
}
"#,
    )
    .expect("write route html source");
    let out = temp_output_dir("reveal-route-html");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let html = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "domain" && entry.name == "html")
        .expect("html origin");

    let reveal = reveal_origin_json(&out, &html.id).expect("reveal html origin");

    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@html")));
    let routes = reveal["production"]["routes"]
        .as_array()
        .expect("production routes");
    assert!(routes.iter().any(|route| {
        route["method"] == "GET"
            && route["path"] == "/"
            && route["match"] == "contains"
            && route["matched_origin_id"] == html.id
    }));
    let native_server = reveal["production"]["native_server"]
        .as_array()
        .expect("native server targets");
    assert!(native_server.iter().any(|target| {
        target["routes"]
            .as_array()
            .expect("native routes")
            .iter()
            .any(|route| route["method"] == "GET" && route["path"] == "/")
    }));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_links_build_artifact_back_to_source_and_route() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("reveal-origin");

    cmd_build(&path, &out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "GET /ping")
        .expect("route origin");

    let reveal = reveal_origin_json(&out, &route.id).expect("reveal origin");

    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], route.id);
    assert_eq!(reveal["origin"]["kind"], "route");
    assert_eq!(reveal["origin"]["name"], "GET /ping");
    let canonical_path = std::fs::canonicalize(&path).expect("canonical entry path");
    assert_eq!(
        reveal["source"]["path"],
        canonical_path.display().to_string()
    );
    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert_eq!(reveal["project_graph"]["kind"], "domain");
    assert_eq!(reveal["project_graph"]["name"], "route");
    assert!(reveal["production"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    let native_server = reveal["production"]["native_server"]
        .as_array()
        .expect("native server targets");
    assert!(native_server.iter().any(|target| {
        target["kind"] == "native_server_plan"
            && target["path"] == "server/native-server.json"
            && target["status"] == "direct_http"
            && target["artifact"] == "server/app.orv-runtime.json"
            && target["target"]["path"] == "server/app"
            && target["routes_source"]["path"] == "server/native/routes.rs"
            && target["routes_source"]["exists"] == true
            && target["routes_source"]["route_count"] == 1
            && target["router_source"]["path"] == "server/native/router.rs"
            && target["router_source"]["exists"] == true
            && target["router_source"]["dispatch"] == true
            && target["router_source"]["handler_count_contract"] == true
            && target["router_source"]["response_origin_dispatch"] == true
            && target["handlers_source"]["path"] == "server/native/handlers.rs"
            && target["handlers_source"]["exists"] == true
            && target["handlers_source"]["handler_count_contract"] == true
            && target["handlers_source"]["body_lowering_placeholder"] == false
            && target["handlers_source"]["response_origin_dispatch"] == true
            && target["runtime_image"]["path"] == "server/runtime-image.json"
            && target["runtime_image"]["reference_image"] == "ghcr.io/orv-lang/orv-reference:latest"
            && target["runtime_image"]["target"]["image"] == "orv-native-server:latest"
            && target["commands"]["build"]
                == serde_json::json!([
                    "cargo",
                    "build",
                    "--manifest-path",
                    "server/native/Cargo.toml",
                    "--release"
                ])
            && target["commands"]["run"]["env"]["ORV_BUILD_DIR"] == "."
            && target["commands"]["run"]["command"]
                == serde_json::json!(["./server/native/target/release/orv-native-server"])
            && target["routes"]
                .as_array()
                .expect("native routes")
                .iter()
                .any(|route| route["method"] == "GET" && route["path"] == "/ping")
            && target["blocked_by"]
                .as_array()
                .expect("blocked_by")
                .iter()
                .all(|item| item != "native-codegen")
    }));
    assert_eq!(reveal["production"]["summary"]["route_target_count"], 1);
    assert_eq!(
        reveal["production"]["summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        reveal["production"]["summary"]["native_server_route_count"],
        1
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn reveal_origin_exposes_route_policy_contract() {
    let dir = temp_output_dir("reveal-route-policy-source");
    std::fs::create_dir_all(&dir).expect("create route policy reveal source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route POST /checkout {
    @csrf
    @respond 201 { ok: true }
  }
}
"#,
    )
    .expect("write route policy reveal source");
    let out = temp_output_dir("reveal-route-policy");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let route = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "route" && entry.name == "POST /checkout")
        .expect("checkout route origin");

    let reveal = reveal_origin_json(&out, &route.id).expect("reveal origin");
    let routes = reveal["production"]["routes"]
        .as_array()
        .expect("production routes");
    let route = routes
        .iter()
        .find(|route| route["method"] == "POST" && route["path"] == "/checkout")
        .expect("checkout production route");
    let policies = route["policies"].as_array().expect("route policies");

    assert!(policies.iter().any(|policy| policy["kind"] == "csrf"
        && policy["surface"] == "first_party_compiler_plugin"
        && policy["required"] == true
        && policy["origin_id"]
            .as_str()
            .is_some_and(|origin_id| origin_id.starts_with("ori_"))));
    assert!(policies.iter().any(|policy| policy["kind"] == "rate_limit"
        && policy["surface"] == "shop_template"
        && policy["limit"] == 10
        && policy["window_seconds"] == 60));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn reveal_origin_uses_build_source_bundle_when_original_client_source_is_missing() {
    let out = temp_output_dir("reveal-client-source-bundle");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");

    cmd_build(&entry, &build_out).expect("build artifacts");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(build_out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let signal = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "signal" && entry.name == "count")
        .expect("signal origin");
    std::fs::remove_file(&entry).expect("remove original source");

    let reveal = reveal_origin_json(&build_out, &signal.id).expect("reveal origin");

    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("let sig count")));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn check_build_reanalyzes_source_bundle_without_original_source() {
    let dir = temp_output_dir("check-build-source-bundle");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("page.orv");
    std::fs::write(
        &path,
        r#"let sig count: int = 0
@out @html { @body { @p count } }"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build(&path, &out).expect("build artifacts");
    std::fs::remove_file(&path).expect("remove source");

    cmd_check_build(&out).expect("check build");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn graph_json_for_path_outputs_schema_nodes_and_edges() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let value = project_graph_json_for_path(&path).expect("graph json");

    assert_eq!(value["schema_version"], 1);
    let nodes = value["nodes"].as_array().expect("nodes array");
    let edges = value["edges"].as_array().expect("edges array");
    assert!(nodes.iter().any(|node| node["kind"] == "file"));
    assert!(nodes.iter().any(|node| node["kind"] == "domain"));
    assert!(edges.iter().any(|edge| edge["kind"] == "contains"));
    assert_eq!(value["stats"]["node_count"], nodes.len());
    assert_eq!(value["stats"]["edge_count"], edges.len());
    assert_eq!(value["stats"]["file_count"], 1);
    assert!(
        value["stats"]["max_semantic_contains_depth"]
            .as_u64()
            .expect("semantic depth")
            >= 2
    );
}

#[test]
fn graph_view_writes_static_html_artifact() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let out = temp_output_dir("graph-view");
    std::fs::create_dir_all(&out).expect("create temp root");
    let value = project_graph_json_for_path(&path).expect("graph json");

    write_project_graph_view(&out, &value).expect("graph view");

    let graph = read_json_value(&out.join("graph.json")).expect("graph artifact");
    assert_eq!(graph["schema_version"], 1);
    let html = std::fs::read_to_string(out.join("index.html")).expect("graph html");
    assert!(html.contains("ORV Project Graph"));
    assert!(html.contains("data-node-count=\""));
    assert!(html.contains("<svg role=\"img\""));
    assert!(html.contains("graph.json"));
    assert!(html.contains("GET /ping"));
    assert!(html.contains("id=\"graph-search\""));
    assert!(html.contains("id=\"graph-kind-filter\""));
    assert!(html.contains("data-graph-node-row"));
    assert!(html.contains("data-node-kind=\"domain\""));
    assert!(html.contains("filterProjectGraphRows"));
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn graph_json_for_path_includes_semantic_origin_map() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let value = project_graph_json_for_path(&path).expect("graph json");
    let entries = value["semantic"]["origin_map"]["entries"]
        .as_array()
        .expect("origin entries array");

    assert!(entries
        .iter()
        .any(|entry| entry["kind"] == "route" && entry["name"] == "GET /ping"));
    assert!(entries
        .iter()
        .any(|entry| entry["kind"] == "domain" && entry["name"] == "respond"));
}

#[test]
fn graph_json_links_semantic_origins_to_ast_nodes() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let value = project_graph_json_for_path(&path).expect("graph json");
    let nodes = value["nodes"].as_array().expect("nodes array");
    let route_node = nodes
        .iter()
        .find(|node| node["kind"] == "domain" && node["name"] == "route")
        .expect("route AST node");
    let route_origin = value["semantic"]["origin_map"]["entries"]
        .as_array()
        .expect("origin entries array")
        .iter()
        .find(|entry| entry["kind"] == "route" && entry["name"] == "GET /ping")
        .expect("route origin");
    let links = value["semantic"]["origin_links"]
        .as_array()
        .expect("origin links array");

    assert!(links.iter().any(|link| {
        link["kind"] == "source_node"
            && link["origin_id"] == route_origin["id"]
            && link["node_id"] == route_node["id"]
    }));
}

#[test]
fn graph_json_includes_semantic_origin_edges() {
    let path = workspace_path(&["fixtures", "e2e", "hello.orv"]);
    let value = project_graph_json_for_path(&path).expect("graph json");
    let entries = value["semantic"]["origin_map"]["entries"]
        .as_array()
        .expect("origin entries array");
    let server = entries
        .iter()
        .find(|entry| entry["kind"] == "domain" && entry["name"] == "server")
        .expect("server origin");
    let route = entries
        .iter()
        .find(|entry| entry["kind"] == "route" && entry["name"] == "GET /ping")
        .expect("route origin");
    let respond = entries
        .iter()
        .find(|entry| entry["kind"] == "domain" && entry["name"] == "respond")
        .expect("respond origin");
    let edges = value["semantic"]["origin_edges"]
        .as_array()
        .expect("origin edges array");

    assert!(edges.iter().any(|edge| {
        edge["kind"] == "contains" && edge["from"] == server["id"] && edge["to"] == route["id"]
    }));
    assert!(edges.iter().any(|edge| {
        edge["kind"] == "contains" && edge["from"] == route["id"] && edge["to"] == respond["id"]
    }));
}

#[test]
fn graph_json_exposes_call_edges_from_origin_map() {
    let path = workspace_path(&["fixtures", "plan", "01-basics.orv"]);
    let value = project_graph_json_for_path(&path).expect("graph json");
    let edges = value["semantic"]["origin_edges"]
        .as_array()
        .expect("origin edges array");

    assert!(edges.iter().any(|edge| edge["kind"] == "calls"));
}
