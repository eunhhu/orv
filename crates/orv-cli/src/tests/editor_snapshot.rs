use super::*;

#[test]
fn editor_snapshot_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "editor", "snapshot", "src/main.orv"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_reveal_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "editor", "reveal", "dist", "ori_1"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_runtime_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "editor", "runtime", "src/main.orv"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn editor_reveal_focuses_route_origin_for_native_navigation() {
    let dir = temp_output_dir("editor-reveal");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r"@server {
  @listen 0
  @route GET /ping {
    @respond 200 { ok: true }
  }
}",
    )
    .expect("write source");
    let out = dir.join("dist");

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

    let reveal = editor_reveal_json(&out, &route.id).expect("editor reveal");

    assert_eq!(reveal["schema_version"], 1);
    assert_eq!(reveal["origin"]["id"], route.id);
    assert_eq!(reveal["focus"]["panel"], "routes");
    assert_eq!(reveal["focus"]["origin_id"], route.id);
    assert_eq!(reveal["source"]["location"]["range"]["start"]["line"], 2);
    assert!(reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@route GET /ping")));
    assert!(reveal["production"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/ping"));
    assert_eq!(reveal["production"]["summary"]["route_target_count"], 1);
    assert_eq!(
        reveal["production"]["summary"]["native_server_target_count"],
        1
    );
    assert_eq!(
        reveal["production"]["summary"]["native_server_route_count"],
        1
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_reveal_exposes_commerce_adapter_origin_match() {
    let dir = temp_output_dir("editor-reveal-commerce-adapter");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route POST /checkout {
    let captured = payments.capture({ orderId: "o_1", amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write source");
    let out = dir.join("dist");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let origin_map: orv_compiler::OriginMap = serde_json::from_str(
        &std::fs::read_to_string(out.join("origin-map.json")).expect("origin map"),
    )
    .expect("origin map json");
    let payment_connect = origin_map
        .entries
        .iter()
        .find(|entry| entry.kind == "call" && entry.name == "@payment.connect")
        .expect("payment connect origin");

    let reveal = editor_reveal_json(&out, &payment_connect.id).expect("editor reveal");
    let commerce = reveal["production"]["commerce_adapters"]
        .as_array()
        .expect("commerce adapters");
    let target = commerce
        .iter()
        .find(|target| target["path"] == "deploy/commerce-adapters.json")
        .expect("commerce adapter target");
    let matched = target["matched_adapters"]
        .as_array()
        .expect("matched commerce adapters");

    assert_eq!(reveal["origin"]["id"], payment_connect.id);
    assert_eq!(reveal["production"]["summary"]["graph_contract_count"], 3);
    assert_eq!(
        reveal["production"]["summary"]["preflight_smoke_summary_missing_count"],
        1
    );
    assert_eq!(reveal["production"]["summary"]["commerce_target_count"], 1);
    assert_eq!(reveal["focus"]["origin_id"], payment_connect.id);
    assert_eq!(reveal["focus"]["panel"], "source");
    assert_eq!(target["matched"], true);
    assert_eq!(target["selected_origin_id"], payment_connect.id);
    assert_eq!(target["matched_adapter_count"], 1);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0]["source_origin_id"], payment_connect.id);
    assert_eq!(matched[0]["matched_origin_id"], payment_connect.id);
    assert_eq!(matched[0]["match"], "direct");
    assert_eq!(matched[0]["kind"], "payment");
    assert_eq!(matched[0]["endpoint"], "http://payments.internal/capture");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_snapshot_outputs_graph_backed_panels() {
    let dir = temp_output_dir("editor-snapshot");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"struct User { id: int }
define Auth() -> { @out "auth" }
@server {
  @listen 8080
  @route GET /users/:id { @respond 200 { ok: true } }
}
"#,
    )
    .expect("write source");

    let snapshot = editor_snapshot_json(&path).expect("editor snapshot");

    assert_eq!(snapshot["schema_version"], 1);
    assert!(snapshot["panels"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .any(|file| file["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))));
    assert!(snapshot["panels"]["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .any(|route| route["method"] == "GET" && route["path"] == "/users/:id"));
    assert!(snapshot["panels"]["schema"]
        .as_array()
        .expect("schema")
        .iter()
        .any(|item| item["kind"] == "struct" && item["name"] == "User"));
    assert!(snapshot["panels"]["domains"]
        .as_array()
        .expect("domains")
        .iter()
        .any(|item| item["kind"] == "define" && item["name"] == "Auth"));
    assert_eq!(snapshot["live_refresh"]["strategy"], "source-hash");
    assert!(snapshot["live_refresh"]["watch"]["sources"]
        .as_array()
        .expect("watch sources")
        .iter()
        .any(|source| source["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("app.orv"))
            && source["content_hash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("fnv1a64:"))));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn editor_runtime_outputs_reference_runtime_inspection_panel() {
    let dir = temp_output_dir("editor-runtime");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("app.orv");
    std::fs::write(&path, "@out \"editor-runtime-ready\"\n").expect("write source");

    let runtime = editor_runtime_json(&path).expect("editor runtime");

    assert_eq!(runtime["schema_version"], 1);
    assert_eq!(runtime["runtime"]["status"], "ok");
    assert_eq!(runtime["runtime"]["stdout"], "editor-runtime-ready\n");
    assert_eq!(runtime["panels"]["runtime"]["status"], "ok");
    assert_eq!(
        runtime["panels"]["runtime"]["stdout"],
        "editor-runtime-ready\n"
    );
    assert!(!runtime["frames"].as_array().expect("frames").is_empty());
    let _ = std::fs::remove_dir_all(dir);
}
