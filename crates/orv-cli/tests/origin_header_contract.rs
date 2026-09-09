use crate::support::{read_json, run_orv, temp_dir as temp_output_dir};

const ROUTE_ORIGIN_SERVER_ROUTE_GOLDEN: &str =
    include_str!("../../../docs/samples/route-origin-server-route-v1.golden.json");

#[test]
fn generated_smoke_freezes_origin_header_contract() {
    let out = temp_output_dir("origin-header-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r#"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
"#,
    )
    .expect("write fixture");
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    run_orv(&["verify-build", &out_arg]);

    let server = read_json(&out.join("server").join("app.orv-runtime.json"));
    let route = server["routes"]
        .as_array()
        .expect("routes array")
        .iter()
        .find(|route| route["method"] == "GET" && route["path"] == "/ping")
        .expect("GET /ping route");
    let route_golden: serde_json::Value =
        serde_json::from_str(ROUTE_ORIGIN_SERVER_ROUTE_GOLDEN).expect("route origin golden");
    assert_eq!(
        route, &route_golden,
        "route origin server route golden drift"
    );
    let route_origin = route["origin_id"].as_str().expect("route origin id");
    let response_origin = route["response_origin_ids"]
        .as_array()
        .expect("response origins array")[0]
        .as_str()
        .expect("response origin id");

    let smoke =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    assert!(smoke.contains("orv_smoke_origin_header()"));
    assert!(smoke.contains("orv_smoke_response_origin_header()"));
    assert!(smoke.contains(r"x-orv-origin-id:"));
    assert!(smoke.contains(r"x-orv-response-origin-id:"));
    assert!(smoke.contains("missing x-orv-origin-id"));
    assert!(smoke.contains("wrong x-orv-origin-id expected"));
    assert!(smoke.contains("missing x-orv-response-origin-id"));
    assert!(smoke.contains("wrong x-orv-response-origin-id expected"));
    assert!(smoke.contains(&format!(r#"ORV_SMOKE_ORIGIN_GET_PING="{route_origin}""#)));
    assert!(smoke.contains(&format!(
        r#"ORV_SMOKE_RESPONSE_ORIGIN_GET_PING="{response_origin}""#
    )));
    assert!(smoke.contains(
        r#"orv_smoke_curl_origin_response "GET /ping" "$ORV_SMOKE_ORIGIN_GET_PING" "$ORV_SMOKE_RESPONSE_ORIGIN_GET_PING" "$BASE_URL/ping""#
    ));

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn generated_smoke_does_not_force_ambiguous_response_origin_header() {
    let out = temp_output_dir("origin-header-ambiguous-response-contract");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("temp output dir");
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r#"@server {
  @listen 8080

  @route GET /mode {
    if @query.mode == "full" {
      @respond 200 { mode: "full" }
    }
    @respond 204 {}
  }
}
"#,
    )
    .expect("write fixture");
    let fixture_arg = fixture.display().to_string();
    let out_arg = out.display().to_string();

    run_orv(&["build", &fixture_arg, "--out", &out_arg, "--prod"]);
    run_orv(&["verify-build", &out_arg]);

    let server = read_json(&out.join("server").join("app.orv-runtime.json"));
    let route = server["routes"]
        .as_array()
        .expect("routes array")
        .iter()
        .find(|route| route["method"] == "GET" && route["path"] == "/mode")
        .expect("GET /mode route");
    assert_eq!(
        route["response_origin_ids"]
            .as_array()
            .expect("response origins array")
            .len(),
        2,
        "fixture must stay ambiguous"
    );

    let smoke =
        std::fs::read_to_string(out.join("deploy").join("smoke-test.sh")).expect("smoke test");
    assert!(smoke.contains(r#"ORV_SMOKE_ORIGIN_GET_MODE="ori_"#));
    assert!(!smoke.contains("ORV_SMOKE_RESPONSE_ORIGIN_GET_MODE="));
    assert!(smoke.contains(
        r#"orv_smoke_curl_origin "GET /mode" "$ORV_SMOKE_ORIGIN_GET_MODE" "$BASE_URL/mode""#
    ));
    assert!(!smoke
        .contains(r#"orv_smoke_curl_origin_response "GET /mode" "$ORV_SMOKE_ORIGIN_GET_MODE""#));

    let _ = std::fs::remove_dir_all(&out);
}
