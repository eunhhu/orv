use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-cli-{name}-{}-{nanos}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv_json(args: &[&str]) -> serde_json::Value {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("orv json")
}

fn run_orv(args: &[&str]) {
    let output = Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv");
    assert!(
        output.status.success(),
        "orv {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

fn origin_id(origin_map: &serde_json::Value, kind: &str, name: &str) -> String {
    origin_map["entries"]
        .as_array()
        .expect("origin entries")
        .iter()
        .find(|entry| entry["kind"] == kind && entry["name"] == name)
        .unwrap_or_else(|| panic!("missing origin {kind}:{name}"))
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("origin id")
        .to_string()
}

fn has_origin_edge(origin_map: &serde_json::Value, from: &str, to: &str, kind: &str) -> bool {
    origin_map["edges"]
        .as_array()
        .expect("origin edges")
        .iter()
        .any(|edge| edge["from"] == from && edge["to"] == to && edge["kind"] == kind)
}

struct RevealCoverageFixture {
    root: PathBuf,
    out_arg: String,
    route_id: String,
    checkout_route_id: String,
    html_id: String,
    db_id: String,
    db_operation_id: String,
    payment_id: String,
    response_id: String,
}

#[test]
fn cli_reveal_surfaces_share_route_html_db_commerce_and_trace_origins() {
    let fixture = build_reveal_coverage_fixture();

    assert_route_and_html_reveal_contract(&fixture);
    assert_adapter_reveal_contract(&fixture);
    assert_trace_reveal_contract(&fixture);

    let _ = std::fs::remove_dir_all(fixture.root);
}

fn build_reveal_coverage_fixture() -> RevealCoverageFixture {
    let root = temp_dir("reveal-coverage");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    std::fs::write(
        &source,
        r#"@server {
  @listen 8080
  let shopdb = @db.connect(@env.SHOP_DATABASE_URL ?? "postgres://db.internal/shop")
  let payments = @payment.connect(@env.PAYMENT_ADAPTER_URL ?? "http://payments.internal/capture")
  @route GET / {
    @serve @html {
      @body { @h1 "Home" }
    }
  }
  @route POST /checkout {
    let order = await shopdb.create("Order", { id: "o_1", total: 42 })
    let captured = payments.capture({ orderId: order.id, amount: 42, method: "card" })
    @respond 200 { payment: captured.status }
  }
}
"#,
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);

    let origin_map = read_json(&out.join("origin-map.json"));
    let route_id = origin_id(&origin_map, "route", "GET /");
    let checkout_route_id = origin_id(&origin_map, "route", "POST /checkout");
    let html_id = origin_id(&origin_map, "domain", "html");
    let db_id = origin_id(&origin_map, "call", "@db.connect");
    let db_operation_id = origin_id(&origin_map, "call", "shopdb.create");
    let payment_id = origin_id(&origin_map, "call", "@payment.connect");
    let response_id = origin_id(&origin_map, "domain", "respond");

    RevealCoverageFixture {
        root,
        out_arg,
        route_id,
        checkout_route_id,
        html_id,
        db_id,
        db_operation_id,
        payment_id,
        response_id,
    }
}

fn assert_route_and_html_reveal_contract(fixture: &RevealCoverageFixture) {
    let route_reveal = run_orv_json(&["reveal", &fixture.out_arg, &fixture.route_id]);
    assert_eq!(route_reveal["origin"]["id"], fixture.route_id);
    assert_eq!(
        route_reveal["production"]["summary"]["route_target_count"],
        serde_json::json!(1)
    );

    let html_reveal = run_orv_json(&["editor", "reveal", &fixture.out_arg, &fixture.html_id]);
    assert!(html_reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@html")));
    assert!(html_reveal["production"]["routes"]
        .as_array()
        .expect("html routes")
        .iter()
        .any(|route| route["method"] == "GET"
            && route["path"] == "/"
            && route["match"] == "contains"));
}

fn assert_adapter_reveal_contract(fixture: &RevealCoverageFixture) {
    let db_reveal = run_orv_json(&["lsp", "reveal", &fixture.out_arg, &fixture.db_id]);
    let db_target = db_reveal["production"]["db_adapters"]
        .as_array()
        .expect("db adapters")
        .iter()
        .find(|target| target["matched"] == true)
        .expect("matched db target")
        .clone();
    assert_eq!(
        db_target["matched_adapters"][0]["source_origin_id"],
        fixture.db_id
    );
    assert_eq!(
        db_target["matched_adapters"][0]["bridge"]["contract"],
        "http-json-v1"
    );

    let payment_reveal = run_orv_json(&["editor", "reveal", &fixture.out_arg, &fixture.payment_id]);
    let commerce_target = payment_reveal["production"]["commerce_adapters"]
        .as_array()
        .expect("commerce adapters")
        .iter()
        .find(|target| target["matched"] == true)
        .expect("matched commerce target")
        .clone();
    assert_eq!(
        commerce_target["matched_adapters"][0]["source_origin_id"],
        fixture.payment_id
    );
    assert_eq!(
        commerce_target["matched_adapters"][0]["endpoint"],
        "http://payments.internal/capture"
    );
}

fn assert_trace_reveal_contract(fixture: &RevealCoverageFixture) {
    let trace_path = fixture.root.join("trace.json");
    std::fs::write(
        &trace_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": 1,
            "kind": "orv.production.trace",
            "frame_count": 1,
            "frames": [{
                "method": "POST",
                "path": "/checkout",
                "status": 200,
                "route_origin_id": fixture.checkout_route_id,
                "response_origin_id": fixture.response_id,
                "db_operation_origin_id": fixture.db_operation_id,
                "commerce_adapter_origin_id": fixture.payment_id,
            }]
        }))
        .expect("trace json"),
    )
    .expect("write trace");
    let trace_arg = trace_path.display().to_string();
    let trace = run_orv_json(&["editor", "trace", &fixture.out_arg, "--trace", &trace_arg]);
    assert_eq!(trace["frames"][0]["origin_id"], fixture.checkout_route_id);
    assert_eq!(
        trace["frames"][0]["response_origin_id"],
        fixture.response_id
    );
    assert_eq!(
        trace["frames"][0]["db_operation_origin_id"],
        fixture.db_operation_id
    );
    assert_eq!(
        trace["frames"][0]["commerce_adapter_origin_id"],
        fixture.payment_id
    );
    assert_eq!(
        trace["frames"][0]["summary"]["db_operation_origin_id"],
        fixture.db_operation_id
    );
    assert_eq!(
        trace["frames"][0]["request"]["commerce_adapter_origin_id"],
        fixture.payment_id
    );
    assert!(
        trace["frames"][0]["response_navigation"]["source"]["snippet"]
            .as_str()
            .is_some_and(|snippet| snippet.contains("@respond 200"))
    );
    assert!(trace["frames"][0]["db_navigation"]["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("shopdb.create")));
    assert!(
        trace["frames"][0]["commerce_navigation"]["source"]["snippet"]
            .as_str()
            .is_some_and(|snippet| snippet.contains("@payment.connect"))
    );
}

#[test]
fn cli_reveal_follows_function_calls_to_route_and_domain_invocations() {
    let root = temp_dir("reveal-function-domain");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let out = root.join("dist");
    std::fs::write(
        &source,
        r#"function helper(name: string): string -> {
  @out "helper invoked"
  "hello {name}"
}

@server {
  @listen 8080
  @route GET /helper {
    let message = helper("Ada")
    @respond 200 { message: message }
  }
}
"#,
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &source_arg, "--prod", "--out", &out_arg]);

    let origin_map = read_json(&out.join("origin-map.json"));
    let route_id = origin_id(&origin_map, "route", "GET /helper");
    let function_id = origin_id(&origin_map, "function", "helper");
    let call_id = origin_id(&origin_map, "call", "helper");
    let out_id = origin_id(&origin_map, "domain", "out");
    assert!(has_origin_edge(
        &origin_map,
        &route_id,
        &call_id,
        "contains"
    ));
    assert!(has_origin_edge(
        &origin_map,
        &call_id,
        &function_id,
        "calls"
    ));
    assert!(has_origin_edge(
        &origin_map,
        &function_id,
        &out_id,
        "contains"
    ));

    let function_reveal = run_orv_json(&["reveal", &out_arg, &function_id]);
    assert!(function_reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("function helper")));
    assert!(function_reveal["production"]["routes"]
        .as_array()
        .expect("function routes")
        .iter()
        .any(|route| route["path"] == "/helper" && route["match"] == "calls"));

    let domain_reveal = run_orv_json(&["editor", "reveal", &out_arg, &out_id]);
    assert!(domain_reveal["source"]["snippet"]
        .as_str()
        .is_some_and(|snippet| snippet.contains("@out")));
    assert!(domain_reveal["production"]["routes"]
        .as_array()
        .expect("domain routes")
        .iter()
        .any(|route| route["path"] == "/helper" && route["match"] == "calls"));
    assert_eq!(domain_reveal["focus"]["panel"], "domains");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cli_graph_view_exposes_semantic_origin_spine() {
    let root = temp_dir("graph-view-origin-spine");
    std::fs::create_dir_all(&root).expect("create temp dir");
    let source = root.join("app.orv");
    let view = root.join("graph-view");
    std::fs::write(
        &source,
        r"@server {
  @listen 8080
  @route GET /ping {
    @respond 200 { ok: true }
  }
}
",
    )
    .expect("write source");

    let source_arg = source.display().to_string();
    let view_arg = view.display().to_string();
    run_orv(&["graph", &source_arg, "--view", "--out", &view_arg]);

    let graph = read_json(&view.join("graph.json"));
    let route_id = origin_id(&graph["semantic"]["origin_map"], "route", "GET /ping");
    let respond_id = origin_id(&graph["semantic"]["origin_map"], "domain", "respond");
    let origin_edges = graph["semantic"]["origin_edges"]
        .as_array()
        .expect("origin edges");
    assert!(origin_edges.iter().any(|edge| {
        edge["kind"] == "contains" && edge["from"] == route_id && edge["to"] == respond_id
    }));
    let origin_links = graph["semantic"]["origin_links"]
        .as_array()
        .expect("origin links");
    assert!(origin_links
        .iter()
        .any(|link| link["kind"] == "source_node" && link["origin_id"] == route_id));

    let html = std::fs::read_to_string(view.join("index.html")).expect("graph html");
    assert!(html.contains("ORV Project Graph"));
    assert!(html.contains("GET /ping"));
    assert!(html.contains("graph.json"));
    assert!(html.contains("data-node-kind=\"domain\""));
    assert!(html.contains("filterProjectGraphRows"));

    let _ = std::fs::remove_dir_all(root);
}
