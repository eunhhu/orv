#![allow(
    clippy::items_after_statements,
    clippy::needless_raw_string_hashes,
    clippy::too_many_lines,
    clippy::single_match_else,
    clippy::match_same_arms,
    clippy::manual_assert,
    clippy::future_not_send
)]

use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;

use super::{
    json_to_value, login_session_cookie, match_route, normalize_path, parse_query,
    request_trace_json, spawn_attached_server, value_to_json, write_request_trace_file,
    ServerRequestFrame, MAX_BODY_BYTES, ORV_RESPONSE_ORIGIN_ID_HEADER,
};
use crate::interp::{
    ResponseCtx, Value, ORV_CSRF_COOKIE_NAME, ORV_REFERENCE_CSRF_TOKEN,
    VALIDATION_ERROR_RESPONSE_KIND, VALIDATION_ERROR_RESPONSE_SCHEMA_VERSION,
    VALIDATION_FAILED_CODE,
};
use crate::server::runtime::{spawn_for_test, spawn_for_test_with_request_trace_file};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1 as client_http1;
use hyper::Request;
use hyper_util::rt::TokioIo;
use orv_analyzer::lower;
use orv_diagnostics::{FileId, Span};
use orv_hir::{HirExpr, HirExprKind, HirProgram, HirStmt, NameId};
use orv_resolve::resolve;
use orv_syntax::{lex, parse};
use sha2::Sha256;
use tokio::net::TcpStream;

const HTTP_SERVER_V1_GOLDEN: &str =
    include_str!("../../../../docs/samples/http-server-v1.golden.json");

// --- 단위: match_route / parse_query / value_to_json ---

#[test]
fn request_trace_json_uses_production_trace_schema() {
    let frame = ServerRequestFrame {
        method: "GET".to_string(),
        path: "/users/42".to_string(),
        route_method: Some("GET".to_string()),
        route_path: Some("/users/:id".to_string()),
        route_origin_id: Some("ori_route_user".to_string()),
        response_origin_id: Some("ori_response_user".to_string()),
        status: 200,
        params: HashMap::from([("id".to_string(), "42".to_string())]),
        query: HashMap::from([("tab".to_string(), "orders".to_string())]),
        body: "{\"active\":true}".to_string(),
    };

    let trace = request_trace_json(&[frame]);

    assert_eq!(trace["schema_version"], 1);
    assert_eq!(trace["kind"], "orv.production.trace");
    assert_eq!(trace["frame_count"], 1);
    assert_eq!(trace["frames"][0]["method"], "GET");
    assert_eq!(trace["frames"][0]["path"], "/users/42");
    assert_eq!(trace["frames"][0]["status"], 200);
    assert_eq!(trace["frames"][0]["route_method"], "GET");
    assert_eq!(trace["frames"][0]["route_path"], "/users/:id");
    assert_eq!(trace["frames"][0]["route_origin_id"], "ori_route_user");
    assert_eq!(
        trace["frames"][0]["response_origin_id"],
        "ori_response_user"
    );
    assert_eq!(trace["frames"][0]["params"]["id"], "42");
    assert_eq!(trace["frames"][0]["query"]["tab"], "orders");
    assert_eq!(trace["frames"][0]["body"], "{\"active\":true}");
}

#[test]
fn write_request_trace_file_creates_parent_dirs() {
    let dir = std::env::temp_dir().join(format!("orv-runtime-trace-file-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("trace").join("requests.json");
    let frame = ServerRequestFrame {
        method: "POST".to_string(),
        path: "/orders".to_string(),
        route_method: Some("POST".to_string()),
        route_path: Some("/orders".to_string()),
        route_origin_id: Some("ori_route_order".to_string()),
        response_origin_id: Some("ori_response_order".to_string()),
        status: 201,
        params: HashMap::new(),
        query: HashMap::new(),
        body: "{\"sku\":\"book\"}".to_string(),
    };

    write_request_trace_file(&path, &[frame]).expect("write trace");

    let trace: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read trace file"))
            .expect("trace json");
    assert_eq!(trace["kind"], "orv.production.trace");
    assert_eq!(trace["frames"][0]["method"], "POST");
    assert_eq!(trace["frames"][0]["status"], 201);
    assert_eq!(
        trace["frames"][0]["response_origin_id"],
        "ori_response_order"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn match_route_static_equal() {
    let m = match_route("/ping", "/ping").unwrap();
    assert!(m.is_empty());
}

#[test]
fn match_route_static_mismatch_returns_none() {
    assert!(match_route("/ping", "/pong").is_none());
}

#[test]
fn match_route_param_captures_value() {
    let m = match_route("/users/:id", "/users/42").unwrap();
    assert_eq!(m.get("id"), Some(&"42".to_string()));
}

#[test]
fn match_route_param_captures_value_with_static_suffix() {
    let m = match_route("/calendar/:userId.ics", "/calendar/42.ics").unwrap();
    assert_eq!(m.get("userId"), Some(&"42".to_string()));
    assert!(match_route("/calendar/:userId.ics", "/calendar/42.json").is_none());
}

#[test]
fn match_route_multiple_params() {
    let m = match_route("/users/:uid/posts/:pid", "/users/7/posts/hello").unwrap();
    assert_eq!(m.get("uid"), Some(&"7".to_string()));
    assert_eq!(m.get("pid"), Some(&"hello".to_string()));
}

#[test]
fn match_route_length_mismatch() {
    // segment 수가 다르면 단순 실패.
    assert!(match_route("/users/:id", "/users/42/extra").is_none());
    assert!(match_route("/users/:id", "/users").is_none());
}

#[test]
fn match_route_catchall_star_matches_any_path() {
    // SPEC §11.2: `@route GET *` 은 어느 경로든 잡는다. 매처 단에서 path
    // 가 "*" 면 params 없이 success.
    assert_eq!(match_route("*", "/").unwrap().len(), 0);
    assert_eq!(match_route("*", "/some/deep/path").unwrap().len(), 0);
    assert_eq!(match_route("*", "/users/42/things/99").unwrap().len(), 0);
}

#[test]
fn match_route_named_wildcard_captures_rest_path() {
    // A2b: `/assets/:rest*` 는 `/assets/` 이후의 모든 세그먼트를 `/` 로
    // 이어 붙여 `rest` 에 캡처.
    let p = match_route("/assets/:rest*", "/assets/foo/bar.png").unwrap();
    assert_eq!(p.get("rest"), Some(&"foo/bar.png".to_string()));

    // 단일 세그먼트도 잡힌다.
    let p = match_route("/assets/:rest*", "/assets/favicon.ico").unwrap();
    assert_eq!(p.get("rest"), Some(&"favicon.ico".to_string()));
}

#[test]
fn match_route_named_wildcard_requires_prefix_match() {
    // prefix(`/assets/`) 가 안 맞으면 실패.
    assert!(match_route("/assets/:rest*", "/other/foo").is_none());
}

#[test]
fn match_route_named_wildcard_needs_at_least_one_segment() {
    // `/assets/:rest*` 에서 rest 는 최소 1개 세그먼트 — `/assets` 만 오면
    // 매치 실패 (rest 가 필수 파라미터).
    assert!(match_route("/assets/:rest*", "/assets").is_none());
}

#[test]
fn match_route_named_wildcard_combined_with_leading_params() {
    // `/api/:ver/files/:rest*` 처럼 앞쪽 :param 과 조합.
    let p = match_route("/api/:ver/files/:rest*", "/api/v1/files/a/b/c.txt").unwrap();
    assert_eq!(p.get("ver"), Some(&"v1".to_string()));
    assert_eq!(p.get("rest"), Some(&"a/b/c.txt".to_string()));
}

#[test]
fn normalize_path_strips_trailing_slash() {
    assert_eq!(normalize_path("/users/42/"), "/users/42");
    assert_eq!(normalize_path("/users/42"), "/users/42");
}

#[test]
fn normalize_path_preserves_root() {
    // `/` 자체는 빈 문자열이 되면 의미가 무너지므로 예외.
    assert_eq!(normalize_path("/"), "/");
    assert_eq!(normalize_path("///"), "/");
}

#[test]
fn parse_query_basic() {
    let q = parse_query("a=1&b=hello");
    assert_eq!(q.get("a"), Some(&"1".to_string()));
    assert_eq!(q.get("b"), Some(&"hello".to_string()));
}

#[test]
fn parse_query_plus_becomes_space() {
    let q = parse_query("msg=hello+world");
    assert_eq!(q.get("msg"), Some(&"hello world".to_string()));
}

#[test]
fn parse_query_empty_returns_empty() {
    assert!(parse_query("").is_empty());
}

#[test]
fn parse_query_percent_decodes_value() {
    // RFC 3986 percent-encoding: %20 → space, %26 → &, %3D → =.
    let q = parse_query("q=hello%20world&amp=%26&eq=%3D");
    assert_eq!(q.get("q"), Some(&"hello world".to_string()));
    assert_eq!(q.get("amp"), Some(&"&".to_string()));
    assert_eq!(q.get("eq"), Some(&"=".to_string()));
}

#[test]
fn parse_query_percent_decodes_key() {
    // 드물지만 key 도 encoded 될 수 있다 (`foo bar=1` → `foo%20bar=1`).
    let q = parse_query("foo%20bar=1");
    assert_eq!(q.get("foo bar"), Some(&"1".to_string()));
}

#[test]
fn parse_query_percent_decodes_utf8() {
    // `안녕` UTF-8 = E0 95 88 EB 85 95 (3+3 바이트). percent-encoded 로 오면
    // 바이트 시퀀스를 재조립해 UTF-8 문자열로 복원해야 한다.
    let q = parse_query("name=%EC%95%88%EB%85%95");
    assert_eq!(q.get("name"), Some(&"안녕".to_string()));
}

#[test]
fn parse_query_plus_and_percent_mix() {
    // `+` 는 space, `%2B` 는 literal `+`. 둘이 한 value 에 섞여도 구분돼야 한다.
    let q = parse_query("x=a+b%2Bc");
    assert_eq!(q.get("x"), Some(&"a b+c".to_string()));
}

#[test]
fn parse_query_malformed_percent_kept_raw() {
    // `%ZZ` 같이 잘못된 encoding 은 raw 로 보존한다 (400 대신 best-effort).
    // SPEC §11.3 에 명시 규칙이 없어 MVP 는 관대한 파싱 채택.
    let q = parse_query("x=%ZZ&y=%2");
    assert_eq!(q.get("x"), Some(&"%ZZ".to_string()));
    assert_eq!(q.get("y"), Some(&"%2".to_string()));
}

#[test]
fn value_to_json_scalars() {
    assert_eq!(value_to_json(&Value::Int(42)), serde_json::json!(42));
    assert_eq!(value_to_json(&Value::Bool(true)), serde_json::json!(true));
    assert_eq!(
        value_to_json(&Value::Str("hi".into())),
        serde_json::json!("hi")
    );
    assert_eq!(value_to_json(&Value::Void), serde_json::Value::Null);
}

#[test]
fn value_to_json_object_roundtrip() {
    let v = Value::Object(vec![
        ("id".into(), Value::Int(1)),
        ("name".into(), Value::Str("alice".into())),
    ]);
    let j = value_to_json(&v);
    assert_eq!(j["id"], serde_json::json!(1));
    assert_eq!(j["name"], serde_json::json!("alice"));
}

#[test]
fn value_to_json_nested_array_of_objects() {
    let v = Value::Array(vec![
        Value::Object(vec![("n".into(), Value::Int(1))]),
        Value::Object(vec![("n".into(), Value::Int(2))]),
    ]);
    let j = value_to_json(&v);
    assert_eq!(j[0]["n"], serde_json::json!(1));
    assert_eq!(j[1]["n"], serde_json::json!(2));
}

#[test]
fn json_to_value_preserves_big_integers_as_string() {
    // 9_999_999_999_999_999_999 는 i64::MAX(9_223_372_036_854_775_807)를
    // 넘고, f64 로 몰면 표현이 어긋난다. 원문 그대로 Value::Str 로 보존.
    let j: serde_json::Value = serde_json::from_str("9999999999999999999").expect("parse");
    match json_to_value(j) {
        Value::Str(s) => assert_eq!(s, "9999999999999999999"),
        other => panic!("expected Str for big int, got {other:?}"),
    }
}

#[test]
fn json_to_value_int_within_i64_range() {
    let j: serde_json::Value = serde_json::from_str("42").expect("parse");
    match json_to_value(j) {
        Value::Int(n) => assert_eq!(n, 42),
        other => panic!("expected Int, got {other:?}"),
    }
}

#[test]
fn json_to_value_float_with_decimal() {
    // `1.5` 는 float — i64 가 아니므로 Float 로 떨어진다.
    let j: serde_json::Value = serde_json::from_str("1.5").expect("parse");
    match json_to_value(j) {
        Value::Float(f) => assert!((f - 1.5).abs() < f64::EPSILON),
        other => panic!("expected Float, got {other:?}"),
    }
}

// --- 통합: 실제 hyper 서버에 HTTP 요청을 쏴서 응답 검증 ---
//
// 모든 통합 테스트는 `#[tokio::test]` (멀티스레드 기본) 로 돌린다.
// `spawn_for_test` 가 accept 루프를 별도 task 로 띄우고, 테스트는 클라이언트
// TcpStream + hyper client::conn 으로 요청을 쏜다. 테스트 종료 시
// `handle.abort()` 로 루프 task 를 정리.

#[derive(Debug)]
struct ServerTestCase {
    listen: Option<Box<HirExpr>>,
    routes: Vec<HirExpr>,
    body_stmts: Vec<HirStmt>,
    captured_env: HashMap<NameId, Value>,
}

fn lower_src(src: &str) -> HirProgram {
    let lx = lex(src, FileId(0));
    assert!(lx.diagnostics.is_empty(), "lex: {:?}", lx.diagnostics);
    let pr = parse(lx.tokens, FileId(0));
    assert!(pr.diagnostics.is_empty(), "parse: {:?}", pr.diagnostics);
    let resolved = resolve(&pr.program);
    assert!(
        resolved.diagnostics.is_empty(),
        "resolve: {:?}",
        resolved.diagnostics
    );
    lower(&pr.program, &resolved)
}

/// orv 소스에서 첫 `@server` 표현식과 그 직전까지의 캡처 환경을 뽑아낸다.
///
/// top-level `let`/`const`/`function` 선언은 production 경로와 같은 방식으로
/// 먼저 실행해 `@server` 의 captured env 에 담는다.
fn extract_server_case(src: &str) -> ServerTestCase {
    let hir = lower_src(src);
    let server_idx = hir
        .items
        .iter()
        .position(|stmt| {
            matches!(
                stmt,
                HirStmt::Expr(HirExpr {
                    kind: HirExprKind::Server { .. },
                    ..
                })
            )
        })
        .expect("expected top-level @server expression");

    let captured_env = if server_idx == 0 {
        HashMap::new()
    } else {
        let prefix = HirProgram {
            items: hir.items[..server_idx].to_vec(),
            span: hir.items[0].span().join(hir.items[server_idx - 1].span()),
        };
        let mut sink = Vec::new();
        crate::interp::run_with_writer_in_env(&prefix, HashMap::new(), &mut sink)
            .expect("prefix program should execute")
    };

    let HirStmt::Expr(expr) = &hir.items[server_idx] else {
        panic!("expected server expr");
    };
    let HirExprKind::Server {
        listen,
        routes,
        body_stmts,
    } = &expr.kind
    else {
        panic!("expected Server variant");
    };
    ServerTestCase {
        listen: listen.clone(),
        routes: routes.clone(),
        body_stmts: body_stmts.clone(),
        captured_env,
    }
}

const TEST_ORIGIN_HEADER: &str = "x-orv-origin-id";

fn stripe_test_signature(secret: &str, timestamp: &str, payload: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(format!("{timestamp}.{payload}").as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("write hex");
    }
    format!("t={timestamp},v1={hex}")
}

fn expected_origin_id(kind: &str, name: &str, span: Span) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in kind
        .as_bytes()
        .iter()
        .chain(name.as_bytes())
        .copied()
        .chain(span.file.index().to_le_bytes())
        .chain(span.range.start.to_le_bytes())
        .chain(span.range.end.to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("ori_{hash:016x}")
}

fn assert_json_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

fn sse_event_data_values(body: &str, event_name: &str) -> Vec<serde_json::Value> {
    body.split("\n\n")
        .filter_map(|block| {
            let mut event = None;
            let mut data = String::new();
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("event: ") {
                    event = Some(value.trim());
                } else if let Some(value) = line.strip_prefix("data: ") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(value);
                }
            }
            (event == Some(event_name)).then(|| {
                serde_json::from_str(&data).unwrap_or_else(|err| {
                    panic!("failed to parse {event_name} event data as json: {err}: {data}")
                })
            })
        })
        .collect()
}

/// 요청을 쏘고 (status, content-type, origin id, body 바이트) 튜플로 돌려준다.
///
/// Request body 는 `body` 가 `Some` 이면 application/json 으로 보낸다.
async fn send_request_full(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<String>,
) -> (
    u16,
    Option<String>,
    Option<String>,
    HashMap<String, String>,
    Vec<u8>,
) {
    send_request_full_with_headers(addr, method, path, body, &[]).await
}

async fn send_request_full_with_headers(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<String>,
    headers: &[(&str, &str)],
) -> (
    u16,
    Option<String>,
    Option<String>,
    HashMap<String, String>,
    Vec<u8>,
) {
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = client_http1::handshake(io).await.expect("handshake");
    // 커넥션 드라이버는 백그라운드 task 로.
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let uri: hyper::Uri = path.parse().expect("uri");
    // body 가 없으면 빈 Full<Bytes> 로 통일 — 핸드셰이크 센더가 단일 body
    // 타입만 받으므로 if/else 분기에서 타입을 섞을 수 없다.
    let (bytes, has_body) = body.map_or_else(|| (Bytes::new(), false), |b| (Bytes::from(b), true));
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost");
    if has_body {
        builder = builder.header("content-type", "application/json");
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let req = builder.body(Full::new(bytes)).expect("build req");
    let resp = sender.send_request(req).await.expect("send");

    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);
    let origin = resp
        .headers()
        .get(TEST_ORIGIN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);
    let mut headers = HashMap::<String, String>::new();
    for (name, value) in resp.headers() {
        let Ok(value) = value.to_str() else {
            continue;
        };
        headers
            .entry(name.as_str().to_string())
            .and_modify(|existing| {
                existing.push('\n');
                existing.push_str(value);
            })
            .or_insert_with(|| value.to_string());
    }
    let bytes = resp.collect().await.expect("body").to_bytes().to_vec();
    (status, ct, origin, headers, bytes)
}

fn cookie_header_from_set_cookie(value: &str) -> String {
    value
        .lines()
        .filter_map(|cookie| cookie.split(';').next())
        .filter(|cookie| !cookie.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

async fn open_trace_event_stream(addr: SocketAddr) -> (u16, Option<String>, hyper::body::Incoming) {
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = client_http1::handshake(io).await.expect("handshake");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = Request::builder()
        .method("GET")
        .uri("/__orv/trace/events")
        .header("host", "localhost")
        .body(Full::new(Bytes::new()))
        .expect("build req");
    let resp = sender.send_request(req).await.expect("send");
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);
    (status, ct, resp.into_body())
}

async fn send_request_with_content_type(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: String,
    content_type: &str,
) -> (u16, Option<String>, Vec<u8>) {
    send_request_with_content_type_and_headers(addr, method, path, body, content_type, &[]).await
}

async fn send_request_with_content_type_and_headers(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: String,
    content_type: &str,
    headers: &[(&str, &str)],
) -> (u16, Option<String>, Vec<u8>) {
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = client_http1::handshake(io).await.expect("handshake");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let uri: hyper::Uri = path.parse().expect("uri");
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost")
        .header("content-type", content_type);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let req = builder
        .body(Full::new(Bytes::from(body)))
        .expect("build req");
    let resp = sender.send_request(req).await.expect("send");
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);
    let bytes = resp.collect().await.expect("body").to_bytes().to_vec();
    (status, ct, bytes)
}

async fn send_request_with_headers(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<String>,
    headers: &[(&str, &str)],
) -> (u16, Option<String>, Vec<u8>) {
    let stream = TcpStream::connect(addr).await.expect("connect");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = client_http1::handshake(io).await.expect("handshake");
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let uri: hyper::Uri = path.parse().expect("uri");
    let (bytes, has_body) = body.map_or_else(|| (Bytes::new(), false), |b| (Bytes::from(b), true));
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost");
    if has_body {
        builder = builder.header("content-type", "application/json");
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let req = builder.body(Full::new(bytes)).expect("build req");
    let resp = sender.send_request(req).await.expect("send");
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);
    let bytes = resp.collect().await.expect("body").to_bytes().to_vec();
    (status, ct, bytes)
}

/// 요청을 쏘고 (status, content-type, body 바이트) 튜플로 돌려준다.
///
/// Request body 는 `body` 가 `Some` 이면 application/json 으로 보낸다.
async fn send_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<String>,
) -> (u16, Option<String>, Vec<u8>) {
    let (status, ct, _origin, _headers, bytes) = send_request_full(addr, method, path, body).await;
    (status, ct, bytes)
}

async fn run_on_localset<F: std::future::Future>(future: F) -> F::Output {
    tokio::task::LocalSet::new().run_until(future).await
}

#[tokio::test]
async fn serves_simple_get_route_with_object_payload() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true, msg: "pong" } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, ct, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ok"], serde_json::json!(true));
        assert_eq!(json["msg"], serde_json::json!("pong"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn http_server_v1_contract_covers_json_route_and_default_404() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true, msg: "pong" } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, content_type, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        assert_eq!(content_type.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            json,
            serde_json::json!({
                "ok": true,
                "msg": "pong"
            })
        );

        let (missing_status, missing_content_type, missing_body) =
            send_request(addr, "GET", "/missing", None).await;
        assert_eq!(missing_status, 404);
        assert_eq!(
            missing_content_type.as_deref(),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(String::from_utf8_lossy(&missing_body), "Not Found");
        let actual = serde_json::json!({
            "json_route": {
                "status": status,
                "content_type": content_type,
                "body": json,
            },
            "default_404": {
                "status": missing_status,
                "content_type": missing_content_type,
                "body": String::from_utf8_lossy(&missing_body),
            }
        });
        let expected: serde_json::Value =
            serde_json::from_str(HTTP_SERVER_V1_GOLDEN).expect("http server golden");
        assert_eq!(actual, expected, "HTTP Server v1 golden drift");

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn writes_request_trace_file_on_graceful_shutdown() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            "@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }",
        );
        let dir =
            std::env::temp_dir().join(format!("orv-runtime-serve-trace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let trace_path = dir.join("trace").join("requests.json");
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (addr, handle, _boot) = spawn_for_test_with_request_trace_file(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            trace_path.clone(),
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("spawn");

        let (status, _ct, _body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        shutdown_tx.send(()).expect("shutdown send");
        handle.await.expect("server task join");

        let trace: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&trace_path).expect("read trace file"))
                .expect("trace json");
        assert_eq!(trace["kind"], "orv.production.trace");
        assert_eq!(trace["frame_count"], 1);
        assert_eq!(trace["frames"][0]["method"], "GET");
        assert_eq!(trace["frames"][0]["path"], "/ping");
        assert_eq!(trace["frames"][0]["status"], 200);
        assert!(trace["frames"][0]["response_origin_id"]
            .as_str()
            .is_some_and(|origin| origin.starts_with("ori_")));
        let _ = std::fs::remove_dir_all(&dir);
    })
    .await;
}

#[tokio::test]
async fn request_trace_events_endpoint_streams_captured_frames() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            "@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }",
        );
        let dir =
            std::env::temp_dir().join(format!("orv-runtime-trace-events-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let trace_path = dir.join("trace").join("requests.json");
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (addr, handle, _boot) = spawn_for_test_with_request_trace_file(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            trace_path,
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("spawn");

        let (status, _, _) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let (stream_status, content_type, mut body) = open_trace_event_stream(addr).await;

        assert_eq!(stream_status, 200);
        assert_eq!(content_type.as_deref(), Some("text/event-stream"));
        let body = tokio::time::timeout(std::time::Duration::from_secs(1), body.frame())
            .await
            .expect("trace event timeout")
            .expect("trace event")
            .expect("trace event frame")
            .into_data()
            .expect("trace event data");
        let body = String::from_utf8(body.to_vec()).expect("event stream utf8");
        assert!(body.contains("event: orv:trace"));
        assert!(body.contains("\"kind\":\"orv.production.trace\""));
        assert!(body.contains("\"frame_count\":1"));
        assert!(body.contains("\"path\":\"/ping\""));
        shutdown_tx.send(()).expect("shutdown send");
        handle.await.expect("server task join");
        let _ = std::fs::remove_dir_all(dir);
    })
    .await;
}

#[tokio::test]
async fn request_trace_events_endpoint_emits_per_frame_events() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            "@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }",
        );
        let dir = std::env::temp_dir().join(format!(
            "orv-runtime-trace-frame-events-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let trace_path = dir.join("trace").join("requests.json");
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (addr, handle, _boot) = spawn_for_test_with_request_trace_file(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            trace_path,
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("spawn");

        assert_eq!(send_request(addr, "GET", "/ping", None).await.0, 200);
        assert_eq!(send_request(addr, "GET", "/ping", None).await.0, 200);
        let (_, _, mut body) = open_trace_event_stream(addr).await;

        let body = tokio::time::timeout(std::time::Duration::from_secs(1), body.frame())
            .await
            .expect("trace event timeout")
            .expect("trace event")
            .expect("trace event frame")
            .into_data()
            .expect("trace event data");
        let body = String::from_utf8(body.to_vec()).expect("event stream utf8");
        assert_eq!(body.matches("event: orv:trace.frame").count(), 2);
        assert!(body.contains("\"kind\":\"orv.production.trace.frame\""));
        assert!(body.contains("\"index\":0"));
        assert!(body.contains("\"index\":1"));
        let frame_events = sse_event_data_values(&body, "orv:trace.frame");
        assert_eq!(frame_events.len(), 2);
        for (index, event) in frame_events.iter().enumerate() {
            assert_json_keys(
                event,
                &["schema_version", "kind", "index", "frame"],
                "trace frame event",
            );
            assert_eq!(event["schema_version"], serde_json::json!(1));
            assert_eq!(
                event["kind"],
                serde_json::json!("orv.production.trace.frame")
            );
            assert_eq!(event["index"], serde_json::json!(index));
            assert_json_keys(
                &event["frame"],
                &[
                    "method",
                    "path",
                    "status",
                    "route_method",
                    "route_path",
                    "route_origin_id",
                    "response_origin_id",
                    "params",
                    "query",
                    "body",
                ],
                "trace frame event frame",
            );
            assert_eq!(event["frame"]["method"], serde_json::json!("GET"));
            assert_eq!(event["frame"]["path"], serde_json::json!("/ping"));
            assert_eq!(event["frame"]["status"], serde_json::json!(200));
            assert_eq!(event["frame"]["route_method"], serde_json::json!("GET"));
            assert_eq!(event["frame"]["route_path"], serde_json::json!("/ping"));
            assert!(event["frame"]["route_origin_id"]
                .as_str()
                .is_some_and(|origin| origin.starts_with("ori_")));
            assert!(event["frame"]["response_origin_id"]
                .as_str()
                .is_some_and(|origin| origin.starts_with("ori_")));
            assert!(event["frame"]["params"].is_object());
            assert!(event["frame"]["query"].is_object());
            assert_eq!(event["frame"]["body"], serde_json::json!(""));
        }
        shutdown_tx.send(()).expect("shutdown send");
        handle.await.expect("server task join");
        let _ = std::fs::remove_dir_all(dir);
    })
    .await;
}

#[tokio::test]
async fn request_trace_events_endpoint_stays_open_for_new_frames() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            "@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }",
        );
        let dir = std::env::temp_dir().join(format!(
            "orv-runtime-trace-open-stream-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let trace_path = dir.join("trace").join("requests.json");
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (addr, handle, _boot) = spawn_for_test_with_request_trace_file(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            trace_path,
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("spawn");

        let (status, content_type, mut body) = open_trace_event_stream(addr).await;
        assert_eq!(status, 200);
        assert_eq!(content_type.as_deref(), Some("text/event-stream"));
        let first = tokio::time::timeout(std::time::Duration::from_secs(1), body.frame())
            .await
            .expect("initial event timeout")
            .expect("initial event")
            .expect("initial event frame")
            .into_data()
            .expect("initial data");
        let first = String::from_utf8(first.to_vec()).expect("initial utf8");
        assert!(first.contains("event: orv:trace"));
        assert!(first.contains("\"frame_count\":0"));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), body.frame())
                .await
                .is_err(),
            "trace stream ended before new request frames"
        );

        let ping = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            send_request(addr, "GET", "/ping", None),
        )
        .await
        .expect("ping while trace stream is open");
        assert_eq!(ping.0, 200);
        let next = tokio::time::timeout(std::time::Duration::from_secs(1), body.frame())
            .await
            .expect("frame event timeout")
            .expect("frame event")
            .expect("frame event frame")
            .into_data()
            .expect("frame data");
        let next = String::from_utf8(next.to_vec()).expect("frame utf8");
        assert!(next.contains("event: orv:trace.frame"));
        assert!(next.contains("\"path\":\"/ping\""));
        shutdown_tx.send(()).expect("shutdown send");
        handle.await.expect("server task join");
        let _ = std::fs::remove_dir_all(dir);
    })
    .await;
}

#[tokio::test]
async fn route_response_includes_origin_headers() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }"#,
        );
        let route = routes
            .iter()
            .find(|expr| matches!(expr.kind, HirExprKind::Route { .. }))
            .expect("route");
        let expected_origin = expected_origin_id("route", "GET /ping", route.span);
        let HirExprKind::Route { handler, .. } = &route.kind else {
            unreachable!("route expression");
        };
        let HirStmt::Expr(respond) = &handler.stmts[0] else {
            panic!("expected respond statement");
        };
        let expected_response_origin = expected_origin_id("domain", "respond", respond.span);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, ct, origin, headers, body) =
            send_request_full(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        assert_eq!(origin.as_deref(), Some(expected_origin.as_str()));
        assert_eq!(
            headers
                .get(ORV_RESPONSE_ORIGIN_ID_HEADER)
                .map(String::as_str),
            Some(expected_response_origin.as_str())
        );
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ok"], serde_json::json!(true));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn route_default_response_omits_response_origin_header() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /plain { { ok: true } }
                }"#,
        );
        let route = routes
            .iter()
            .find(|expr| matches!(expr.kind, HirExprKind::Route { .. }))
            .expect("route");
        let expected_origin = expected_origin_id("route", "GET /plain", route.span);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, ct, origin, headers, body) =
            send_request_full(addr, "GET", "/plain", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        assert_eq!(origin.as_deref(), Some(expected_origin.as_str()));
        assert!(!headers.contains_key(ORV_RESPONSE_ORIGIN_ID_HEADER));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ok"], serde_json::json!(true));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn route_response_origin_header_tracks_executed_branch() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /mode {
                        if @query.mode == "full" {
                            @respond 200 { mode: "full" }
                        }
                        @respond 204 {}
                    }
                }"#,
        );
        let route = routes
            .iter()
            .find(|expr| matches!(expr.kind, HirExprKind::Route { .. }))
            .expect("route");
        let expected_origin = expected_origin_id("route", "GET /mode", route.span);
        let HirExprKind::Route { handler, .. } = &route.kind else {
            unreachable!("route expression");
        };
        let HirStmt::Expr(branch) = &handler.stmts[0] else {
            panic!("expected branch expression");
        };
        let HirExprKind::If { then, .. } = &branch.kind else {
            panic!("expected if branch");
        };
        let HirStmt::Expr(full_respond) = &then.stmts[0] else {
            panic!("expected full respond");
        };
        let expected_full_response_origin =
            expected_origin_id("domain", "respond", full_respond.span);
        let HirStmt::Expr(default_respond) = &handler.stmts[1] else {
            panic!("expected default respond");
        };
        let expected_default_response_origin =
            expected_origin_id("domain", "respond", default_respond.span);

        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _ct, origin, headers, body) =
            send_request_full(addr, "GET", "/mode?mode=full", None).await;
        assert_eq!(status, 200);
        assert_eq!(origin.as_deref(), Some(expected_origin.as_str()));
        assert_eq!(
            headers
                .get(ORV_RESPONSE_ORIGIN_ID_HEADER)
                .map(String::as_str),
            Some(expected_full_response_origin.as_str())
        );
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["mode"], serde_json::json!("full"));

        let (status, _ct, origin, headers, body) =
            send_request_full(addr, "GET", "/mode?mode=compact", None).await;
        assert_eq!(status, 204);
        assert_eq!(origin.as_deref(), Some(expected_origin.as_str()));
        assert_eq!(
            headers
                .get(ORV_RESPONSE_ORIGIN_ID_HEADER)
                .map(String::as_str),
            Some(expected_default_response_origin.as_str())
        );
        assert!(body.is_empty());

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn serves_route_with_path_param() {
    run_on_localset(async {
        // `@param` 은 전체 params object, 개별 값은 `.field` 로 접근 (C3 규약).
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /users/:id { @respond 200 { id: @param.id } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/users/42", None).await;
        assert_eq!(status, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        // @param.id 는 문자열로 수집되므로 "42" (string).
        assert_eq!(json["id"], serde_json::json!("42"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn serves_post_route_with_json_body_echo() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /echo { @respond 201 { received: @body } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let payload = r#"{"name":"alice","age":30}"#.to_string();
        let (status, _, body) = send_request(addr, "POST", "/echo", Some(payload)).await;
        assert_eq!(status, 201);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["received"]["name"], serde_json::json!("alice"));
        assert_eq!(json["received"]["age"], serde_json::json!(30));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn checkout_route_has_reference_rate_limit() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /checkout { @respond 200 { ok: true } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for _ in 0..10 {
            let (status, _, _) = send_request(addr, "POST", "/checkout", Some("{}".into())).await;
            assert_eq!(status, 200);
        }
        let (status, content_type, body) =
            send_request(addr, "POST", "/checkout", Some("{}".into())).await;
        assert_eq!(status, 429);
        assert_eq!(content_type.as_deref(), Some("text/plain; charset=utf-8"));
        let body = String::from_utf8(body).expect("rate-limit body utf8");
        assert!(body.contains("rate limit exceeded"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn explicit_route_rate_limit_overrides_default_policy() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /checkout {
                        @rateLimit limit=2 window=60
                        @respond 200 { ok: true }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for _ in 0..2 {
            let (status, _, _) = send_request(addr, "POST", "/checkout", Some("{}".into())).await;
            assert_eq!(status, 200);
        }
        let (status, _, body) = send_request(addr, "POST", "/checkout", Some("{}".into())).await;
        assert_eq!(status, 429);
        assert!(String::from_utf8(body)
            .expect("rate-limit body utf8")
            .contains("rate limit exceeded"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn checkout_route_rate_limit_can_be_exempted() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /checkout {
                        @rateLimit exempt
                        @respond 200 { ok: true }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for _ in 0..12 {
            let (status, _, _) = send_request(addr, "POST", "/checkout", Some("{}".into())).await;
            assert_eq!(status, 200);
        }

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn explicit_route_rate_limit_can_use_body_key() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /limited {
                        @rateLimit key=@body.memberId limit=1 window=60
                        @respond 200 { ok: true }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, _) = send_request(
            addr,
            "POST",
            "/limited",
            Some(r#"{"memberId":"alice"}"#.into()),
        )
        .await;
        assert_eq!(status, 200);
        let (status, _, _) = send_request(
            addr,
            "POST",
            "/limited",
            Some(r#"{"memberId":"bob"}"#.into()),
        )
        .await;
        assert_eq!(status, 200);
        let (status, _, body) = send_request(
            addr,
            "POST",
            "/limited",
            Some(r#"{"memberId":"alice"}"#.into()),
        )
        .await;
        assert_eq!(status, 429);
        assert!(String::from_utf8(body)
            .expect("rate-limit body utf8")
            .contains("rate limit exceeded"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn member_login_sets_reference_session_cookie_defaults() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /members/login {
                        @respond 201 {
                          session: { id: 42, handle: "ada", status: "active", role: "admin" }
                        }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, _, headers, _) =
            send_request_full(addr, "POST", "/members/login", Some("{}".into())).await;
        assert_eq!(status, 201);
        let cookie = headers.get("set-cookie").expect("set-cookie header");
        assert!(cookie.contains("orv_session=42"));
        assert!(cookie.contains("orv_session_role=admin"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=86400"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Secure"));

        handle.abort();
    })
    .await;
}

#[test]
fn member_login_cookie_rejects_unsafe_session_id() {
    let resp = ResponseCtx {
        origin_id: None,
        status: 201,
        payload: Value::Object(vec![(
            "session".to_string(),
            Value::Object(vec![(
                "id".to_string(),
                Value::Str("bad value; Path=/".to_string()),
            )]),
        )]),
        raw_body: None,
        location: None,
    };

    assert_eq!(login_session_cookie("POST", "/members/login", &resp), None);
}

#[tokio::test]
async fn session_required_route_checks_reference_session_cookie() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /account {
                        @session required
                        @respond 200 { sessionId: @session.id }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (missing_status, _, missing_body) = send_request(addr, "GET", "/account", None).await;
        assert_eq!(missing_status, 401);
        let missing: serde_json::Value =
            serde_json::from_slice(&missing_body).expect("missing session json");
        assert_eq!(missing["err"], serde_json::json!("session_required"));

        let (status, _, body) = send_request_with_headers(
            addr,
            "GET",
            "/account",
            None,
            &[("cookie", "orv_session=abc_123")],
        )
        .await;
        assert_eq!(status, 200);
        let ok: serde_json::Value = serde_json::from_slice(&body).expect("session json");
        assert_eq!(ok["sessionId"], serde_json::json!("abc_123"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn auth_role_route_checks_reference_session_role_cookie() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /admin {
                        @Auth required role="admin"
                        @respond 200 { ok: true, role: @session.role }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (missing_status, _, missing_body) = send_request(addr, "GET", "/admin", None).await;
        assert_eq!(missing_status, 401);
        let missing: serde_json::Value =
            serde_json::from_slice(&missing_body).expect("missing auth json");
        assert_eq!(missing["err"], serde_json::json!("auth_required"));

        let (member_status, _, member_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin",
            None,
            &[("cookie", "orv_session=abc_123; orv_session_role=member")],
        )
        .await;
        assert_eq!(member_status, 403);
        let member: serde_json::Value =
            serde_json::from_slice(&member_body).expect("member auth json");
        assert_eq!(member["err"], serde_json::json!("role_required"));
        assert_eq!(member["requiredRole"], serde_json::json!("admin"));
        assert_eq!(member["role"], serde_json::json!("member"));

        let (status, _, body) = send_request_with_headers(
            addr,
            "GET",
            "/admin",
            None,
            &[("cookie", "orv_session=admin_1; orv_session_role=admin")],
        )
        .await;
        assert_eq!(status, 200);
        let ok: serde_json::Value = serde_json::from_slice(&body).expect("auth json");
        assert_eq!(ok["ok"], serde_json::json!(true));
        assert_eq!(ok["role"], serde_json::json!("admin"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn csrf_route_checks_reference_cookie_and_token() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /checkout {
                        @csrf
                        @respond 200 { ok: true }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (missing_status, _, missing_body) =
            send_request(addr, "POST", "/checkout", Some("{}".into())).await;
        assert_eq!(missing_status, 403);
        let missing: serde_json::Value =
            serde_json::from_slice(&missing_body).expect("missing csrf json");
        assert_eq!(missing["err"], serde_json::json!("csrf_token_required"));

        let csrf_cookie = format!("{ORV_CSRF_COOKIE_NAME}={ORV_REFERENCE_CSRF_TOKEN}");
        let (status, _, body) = send_request_with_headers(
            addr,
            "POST",
            "/checkout",
            Some("{}".into()),
            &[
                ("cookie", csrf_cookie.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(status, 200);
        let ok: serde_json::Value = serde_json::from_slice(&body).expect("csrf json");
        assert_eq!(ok["ok"], serde_json::json!(true));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn serves_post_route_with_form_urlencoded_body() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r"@server {
                    @listen 0
                    @route POST /members {
                        @respond 201 {
                            handle: @body.handle,
                            email: @body.email,
                            name: @body.name
                        }
                    }
                }",
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request_with_content_type(
            addr,
            "POST",
            "/members",
            "handle=ada&email=ada%40example.test&name=Ada+Lovelace".to_string(),
            "application/x-www-form-urlencoded; charset=utf-8",
        )
        .await;
        assert_eq!(status, 201);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["handle"], serde_json::json!("ada"));
        assert_eq!(json["email"], serde_json::json!("ada@example.test"));
        assert_eq!(json["name"], serde_json::json!("Ada Lovelace"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn unknown_route_returns_404() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 {} }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, _) = send_request(addr, "GET", "/missing", None).await;
        assert_eq!(status, 404);

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn respond_204_emits_empty_body() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route DELETE /item/:id { @respond 204 {} }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, ct, body) = send_request(addr, "DELETE", "/item/abc", None).await;
        assert_eq!(status, 204);
        assert!(body.is_empty(), "204 should have empty body, got {body:?}");
        assert!(ct.is_none(), "204 should not set a body content-type");

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn trailing_slash_is_normalized_and_matched() {
    run_on_localset(async {
        // 회귀: `/users/42/` 가 `/users/:id` 매처에 잡혀야 한다.
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /users/:id { @respond 200 { id: @param.id } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/users/42/", None).await;
        assert_eq!(status, 200, "trailing-slash path should match");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["id"], serde_json::json!("42"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn catchall_star_route_matches_unknown_paths() {
    run_on_localset(async {
        // SPEC §11.2: `@route GET *` 은 어느 경로도 잡는다. 앞선 구체 route 가
        // 먼저 매치되면 그쪽이 이긴다 — 선언 순서 규칙.
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 { hit: "ping" } }
                    @route GET * { @respond 404 { err: "not found" } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["hit"], serde_json::json!("ping"));

        let (status2, _, body2) = send_request(addr, "GET", "/whatever", None).await;
        assert_eq!(status2, 404, "catchall route should respond 404");
        let json2: serde_json::Value = serde_json::from_slice(&body2).expect("json");
        assert_eq!(json2["err"], serde_json::json!("not found"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn content_type_is_case_insensitive() {
    run_on_localset(async {
        // `APPLICATION/JSON` 도 JSON 경로로 파싱되어 `@body.x` 가 동작해야 한다.
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /m { @respond 200 { x: @body.x } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 일반 send_request 는 소문자 content-type 을 붙이므로 저수준 커스텀
        // 헤더로 보낸다.
        use hyper::client::conn::http1 as client_http1;
        let stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
        let io = TokioIo::new(stream);
        let (mut sender, conn) = client_http1::handshake(io).await.expect("handshake");
        tokio::spawn(async move {
            let _ = conn.await;
        });

        let req = Request::builder()
            .method("POST")
            .uri("/m")
            .header("host", "localhost")
            .header("content-type", "APPLICATION/JSON")
            .body(Full::new(Bytes::from(r#"{"x":7}"#)))
            .expect("build req");
        let resp = sender.send_request(req).await.expect("send");
        let status = resp.status().as_u16();
        let bytes = resp.collect().await.expect("body").to_bytes().to_vec();
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json");

        assert_eq!(status, 200);
        assert_eq!(json["x"], serde_json::json!(7));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn oversized_body_returns_413() {
    run_on_localset(async {
        // MAX_BODY_BYTES = 1 MiB. 이를 살짝 넘기는 바디로 413 을 확인한다.
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /upload { @respond 200 {} }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let big = "a".repeat(MAX_BODY_BYTES + 1024);
        let (status, _, _) = send_request(addr, "POST", "/upload", Some(big)).await;
        assert_eq!(status, 413, "expected 413 Payload Too Large");

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn boot_stmts_run_before_accept() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @out "boot"
                    @listen 0
                    @route GET /p { @respond 200 {} }
                }"#,
        );
        let (addr, handle, boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let boot_str = String::from_utf8(boot).expect("utf-8");
        assert_eq!(boot_str, "boot\n");
        let (status, _, body) = send_request(addr, "GET", "/p", None).await;
        assert_eq!(status, 200);
        assert_eq!(body, b"{}".to_vec());

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_design_tokens_are_visible_to_html_handlers() {
    run_on_localset(async {
            let ServerTestCase {
                listen,
                routes,
                body_stmts,
                captured_env,
            } = extract_server_case(
                r##"@server {
                    @listen 0
                    @design {
                        @colors { surface: "#f8fafc", text: "#15201e" }
                        @spacing { lg: "24px" }
                    }
                    @route GET / {
                        @serve @html {
                            @body {
                                style="background-color: {@design.colors.surface}; color: {@design.colors.text}; padding: {@design.spacing.lg}"
                                @h1 "Miol Shop"
                            }
                        }
                    }
                }"##,
            );
            let (addr, handle, _boot) = spawn_for_test(
                listen.as_deref(),
                &routes,
                &body_stmts,
                captured_env,
                std::future::pending::<()>(),
            )
            .await
            .expect("spawn");

            let (status, _, body) = send_request(addr, "GET", "/", None).await;
            assert_eq!(status, 200);
            let html = String::from_utf8(body).expect("html utf-8");
            assert!(html.contains(
                r##"style="background-color: #f8fafc; color: #15201e; padding: 24px""##
            ));

            handle.abort();
        })
        .await;
}

// --- C6 E2E: fixtures/e2e/*.orv 파일을 실제로 lower 하고 서버를 띄워 ---
// --- 실제 HTTP 요청으로 응답을 검증한다. ---

/// `fixtures/e2e/<name>` 를 읽어 production 과 같은 server prep 입력으로
/// 바꾼다. fixture 는 대개 `@server` 단일 표현식이지만, helper 함수 같은
/// top-level 바인딩이 추가되어도 captured env 로 흘러간다.
fn read_e2e_fixture(name: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/e2e")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn extract_server_from_fixture(name: &str) -> ServerTestCase {
    extract_server_case(&read_e2e_fixture(name))
}

fn spawn_checkout_shipping_failure_server() -> (
    std::net::SocketAddr,
    std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    std::thread::JoinHandle<()>,
) {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("bind shipping failure server");
    listener
        .set_nonblocking(true)
        .expect("set shipping failure listener nonblocking");
    let address = listener.local_addr().expect("shipping failure address");
    let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let server_requests = requests.clone();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while server_requests.lock().expect("requests").len() < 3
            && std::time::Instant::now() < deadline
        {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_blocking_http_request(&mut stream);
                    server_requests.lock().expect("requests").push(request);
                    let body = "transient carrier failure";
                    let response = format!(
                        "HTTP/1.1 500 Internal Server Error\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    std::io::Write::write_all(&mut stream, response.as_bytes())
                        .expect("write shipping failure response");
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(err) => panic!("accept shipping failure request: {err}"),
            }
        }
    });
    (address, requests, server)
}

fn read_blocking_http_request(stream: &mut std::net::TcpStream) -> String {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .expect("set shipping failure stream timeout");
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 512];
    let header_end = loop {
        let read = std::io::Read::read(stream, &mut buf).expect("read shipping failure request");
        assert!(read > 0, "shipping failure request closed before headers");
        bytes.extend_from_slice(&buf[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while bytes.len() < header_end + content_length {
        let read =
            std::io::Read::read(stream, &mut buf).expect("read shipping failure request body");
        assert!(read > 0, "shipping failure request closed before body");
        bytes.extend_from_slice(&buf[..read]);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn snapshot_table_rows<'a>(
    snapshot: &'a serde_json::Value,
    table: &str,
) -> &'a [serde_json::Value] {
    snapshot["tables"][table]["rows"]
        .as_array()
        .map_or(&[], Vec::as_slice)
}

#[tokio::test]
async fn fixture_hello_serves_ping() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_from_fixture("hello.orv");
        assert!(body_stmts.is_empty(), "hello.orv has no boot stmts");
        let (addr, handle, boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");
        assert!(boot.is_empty(), "hello.orv should produce no boot output");

        let (status, ct, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ok"], serde_json::json!(true));
        assert_eq!(json["msg"], serde_json::json!("pong"));

        handle.abort();
    })
    .await;
}

fn assert_validation_error_payload(
    value: &serde_json::Value,
    expected_path: &str,
    expected_code: &str,
    expected_actual: &serde_json::Value,
) {
    assert_eq!(
        value["schema_version"],
        serde_json::json!(VALIDATION_ERROR_RESPONSE_SCHEMA_VERSION)
    );
    assert_eq!(
        value["kind"],
        serde_json::json!(VALIDATION_ERROR_RESPONSE_KIND)
    );
    assert_eq!(value["error"], serde_json::json!(VALIDATION_FAILED_CODE));
    let fields = value["fields"].as_array().expect("validation fields");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0]["path"], serde_json::json!(expected_path));
    assert_eq!(
        fields[0]["code"],
        serde_json::json!(expected_code),
        "validation code drift at {expected_path}: {value}"
    );
    assert!(fields[0]["message"]
        .as_str()
        .is_some_and(|message| message.contains("constraint mismatch")));
    assert!(fields[0]["expected"].is_string());
    assert_eq!(&fields[0]["actual"], expected_actual);
}

#[tokio::test]
async fn declarative_request_bindings_validate_body_query_and_form() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    struct SearchQuery {
                      page: int(min=1)
                      q: string(trim, lower, min=1)
                    }
                    struct SignupForm {
                      email: string(trim, lower)
                      age: int(min=13)
                    }
                    @route GET /search {
                      @query: SearchQuery
                      @respond 200 { page: @query.page, q: @query.q }
                    }
                    @route POST /signup-json {
                      @body: SignupForm
                      @respond 200 { email: @body.email, age: @body.age }
                    }
                    @route POST /signup {
                      @form: SignupForm
                      @respond 200 { email: @form.email, age: @form.age }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (search_status, _, search_body) =
            send_request(addr, "GET", "/search?page=2&q=%20HELLO%20", None).await;
        assert_eq!(search_status, 200);
        let search: serde_json::Value = serde_json::from_slice(&search_body).expect("search json");
        assert_eq!(search["page"], serde_json::json!(2));
        assert_eq!(search["q"], serde_json::json!("hello"));

        let (bad_search_status, bad_search_ct, bad_search_body) =
            send_request(addr, "GET", "/search?page=0&q=hello", None).await;
        assert_eq!(bad_search_status, 400);
        assert_eq!(bad_search_ct.as_deref(), Some("application/json"));
        let bad_search: serde_json::Value =
            serde_json::from_slice(&bad_search_body).expect("bad search json");
        assert_validation_error_payload(
            &bad_search,
            "$.page",
            "type_mismatch",
            &serde_json::json!("0"),
        );

        let (json_signup_status, json_signup_ct, json_signup_body) = send_request(
            addr,
            "POST",
            "/signup-json",
            Some(r#"{"email":" USER@ORV.DEV ","age":"15"}"#.to_string()),
        )
        .await;
        assert_eq!(json_signup_status, 200);
        assert_eq!(json_signup_ct.as_deref(), Some("application/json"));
        let json_signup: serde_json::Value =
            serde_json::from_slice(&json_signup_body).expect("json signup");
        assert_eq!(json_signup["email"], serde_json::json!("user@orv.dev"));
        assert_eq!(json_signup["age"], serde_json::json!(15));

        let (bad_json_signup_status, bad_json_signup_ct, bad_json_signup_body) = send_request(
            addr,
            "POST",
            "/signup-json",
            Some(r#"{"email":"ok@orv.dev","age":12}"#.to_string()),
        )
        .await;
        assert_eq!(bad_json_signup_status, 400);
        assert_eq!(bad_json_signup_ct.as_deref(), Some("application/json"));
        let bad_json_signup: serde_json::Value =
            serde_json::from_slice(&bad_json_signup_body).expect("bad json signup");
        assert_validation_error_payload(
            &bad_json_signup,
            "$.age",
            "constraint_mismatch",
            &serde_json::json!(12),
        );

        let (signup_status, _, signup_body) = send_request_with_content_type(
            addr,
            "POST",
            "/signup",
            "email=%20USER%40ORV.DEV%20&age=15".to_string(),
            "application/x-www-form-urlencoded",
        )
        .await;
        assert_eq!(signup_status, 200);
        let signup: serde_json::Value = serde_json::from_slice(&signup_body).expect("signup json");
        assert_eq!(signup["email"], serde_json::json!("user@orv.dev"));
        assert_eq!(signup["age"], serde_json::json!(15));

        let (bad_signup_status, bad_signup_ct, bad_signup_body) = send_request_with_content_type(
            addr,
            "POST",
            "/signup",
            "email=ok%40orv.dev&age=12".to_string(),
            "application/x-www-form-urlencoded",
        )
        .await;
        assert_eq!(bad_signup_status, 400);
        assert_eq!(bad_signup_ct.as_deref(), Some("application/json"));
        let bad_signup: serde_json::Value =
            serde_json::from_slice(&bad_signup_body).expect("bad signup json");
        assert_validation_error_payload(
            &bad_signup,
            "$.age",
            "type_mismatch",
            &serde_json::json!("12"),
        );

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn fixture_path_param_covers_param_query_and_json_body() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_from_fixture("path_param.orv");
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 1) :id 경로 파라미터
        let (s1, _, b1) = send_request(addr, "GET", "/users/42", None).await;
        assert_eq!(s1, 200);
        let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
        assert_eq!(j1["id"], serde_json::json!("42"));

        // 2) @query.q — URI 에 쿼리스트링 직접 포함
        let (s2, _, b2) = send_request(addr, "GET", "/search?q=orv", None).await;
        assert_eq!(s2, 200);
        let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
        assert_eq!(j2["q"], serde_json::json!("orv"));

        // 2b) percent-encoded + `+` 혼합 쿼리 — `hello world` 와 UTF-8 `안녕`
        //     모두 핸들러까지 디코딩된 채로 도달해야 한다 (A1).
        let (s2b, _, b2b) = send_request(
            addr,
            "GET",
            "/search?q=hello+world%20%EC%95%88%EB%85%95",
            None,
        )
        .await;
        assert_eq!(s2b, 200);
        let j2b: serde_json::Value = serde_json::from_slice(&b2b).expect("json");
        assert_eq!(j2b["q"], serde_json::json!("hello world 안녕"));

        // 3) POST /echo 에 JSON body 보내면 그대로 되돌려받아야 한다
        let payload = r#"{"name":"alice","age":30}"#.to_string();
        let (s3, _, b3) = send_request(addr, "POST", "/echo", Some(payload)).await;
        assert_eq!(s3, 201);
        let j3: serde_json::Value = serde_json::from_slice(&b3).expect("json");
        assert_eq!(j3["received"]["name"], serde_json::json!("alice"));
        assert_eq!(j3["received"]["age"], serde_json::json!(30));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn request_state_v1_contract_covers_param_query_header_body_and_raw_body() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /users/:id {
                      @respond 201 {
                        id: @param.id,
                        q: @query.q,
                        auth: @header["x-client-auth"],
                        name: @body.name,
                        age: @body.age,
                        raw: @request.rawBody
                      }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let payload = r#"{"name":"Ada","age":37}"#.to_string();
        let (status, content_type, _, _, body) = send_request_full_with_headers(
            addr,
            "POST",
            "/users/u-42?q=hello+world%20%EC%95%88%EB%85%95",
            Some(payload.clone()),
            &[("x-client-auth", "token-123")],
        )
        .await;

        assert_eq!(status, 201);
        assert_eq!(content_type.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["id"], serde_json::json!("u-42"));
        assert_eq!(json["q"], serde_json::json!("hello world 안녕"));
        assert_eq!(json["auth"], serde_json::json!("token-123"));
        assert_eq!(json["name"], serde_json::json!("Ada"));
        assert_eq!(json["age"], serde_json::json!(37));
        assert_eq!(json["raw"], serde_json::json!(payload));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn fixture_shopping_mall_covers_home_catalog_and_order_flow() {
    run_on_localset(async {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let sqlite_path = std::env::temp_dir().join(format!(
            "orv-shopping-fixture-{}-{unique}.sqlite",
            std::process::id()
        ));
        let payment_path = std::env::temp_dir().join(format!(
            "orv-shopping-fixture-payments-{}-{unique}.jsonl",
            std::process::id()
        ));
        let shipping_path = std::env::temp_dir().join(format!(
            "orv-shopping-fixture-shipments-{}-{unique}.jsonl",
            std::process::id()
        ));
        let src = read_e2e_fixture("shopping_mall.orv")
            .replace(
                "sqlite://data/shop.sqlite",
                &format!("sqlite://{}", sqlite_path.display()),
            )
            .replace(
                "file://data/payments.jsonl",
                &format!("file://{}", payment_path.display()),
            )
            .replace(
                "file://data/shipments.jsonl",
                &format!("file://{}", shipping_path.display()),
            );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (home_status, home_ct, _home_origin, home_headers, home_body) =
            send_request_full(addr, "GET", "/", None).await;
        assert_eq!(home_status, 200);
        assert_eq!(home_ct.as_deref(), Some("text/html; charset=utf-8"));
        let csrf_cookie = home_headers
            .get("set-cookie")
            .expect("home csrf set-cookie");
        assert!(csrf_cookie.contains(&format!(
            "{ORV_CSRF_COOKIE_NAME}={ORV_REFERENCE_CSRF_TOKEN}"
        )));
        assert!(csrf_cookie.contains("SameSite=Lax"));
        let csrf_cookie_pair = csrf_cookie
            .split(';')
            .next()
            .expect("csrf cookie pair")
            .to_string();
        let home_html = String::from_utf8(home_body).expect("home html");
        assert!(home_html.contains("<h1>Miol Shop</h1>"));
        assert!(home_html.contains("<form action=\"/products\" method=\"post\">"));
        assert!(home_html
            .contains("<input type=\"text\" name=\"badge\" value=\"New arrival\" required>"));
        assert!(home_html.contains("<input type=\"number\" name=\"stock\" required>"));
        assert!(home_html.contains("<form action=\"/orders\" method=\"post\">"));
        assert!(home_html.contains("<form action=\"/members/login\" method=\"post\">"));
        assert!(home_html.contains("<form action=\"/checkout\" method=\"post\">"));
        assert!(home_html
            .contains("<input type=\"hidden\" name=\"_csrf\" value=\"orv-reference-csrf\">"));
        assert_eq!(
            home_html
                .matches("<input type=\"hidden\" name=\"_csrf\" value=\"orv-reference-csrf\">")
                .count(),
            8
        );
        assert!(home_html.contains("<form action=\"/cart/items\" method=\"post\">"));
        assert!(home_html.contains("<a href=\"/admin\">Admin dashboard</a>"));
        assert!(home_html.contains("<a href=\"/catalog\">Shop catalog</a>"));
        assert!(home_html.contains("<a href=\"/cart\">Cart</a>"));
        assert!(home_html.contains("<a href=\"/account/sessions\">My sessions</a>"));
        assert!(home_html.contains("POST /payments"));
        assert!(home_html.contains("POST /webhooks/stripe"));
        assert!(home_html.contains("POST /shipments"));

        let csrf_required_routes = [
            (
                "/products",
                serde_json::json!({
                    "sku": "csrf-product",
                    "name": "CSRF Product",
                    "badge": "CSRF",
                    "price": 1,
                    "stock": 1
                })
                .to_string(),
            ),
            (
                "/members",
                serde_json::json!({
                    "handle": "csrf-member",
                    "name": "CSRF Member",
                    "email": "csrf-member@example.test"
                })
                .to_string(),
            ),
            (
                "/members/login",
                serde_json::json!({
                    "handle": "csrf-member",
                    "email": "csrf-member@example.test"
                })
                .to_string(),
            ),
            (
                "/cart/items",
                serde_json::json!({
                    "handle": "csrf-member",
                    "sku": "csrf-product",
                    "quantity": 1
                })
                .to_string(),
            ),
            (
                "/orders",
                serde_json::json!({
                    "customer": "csrf-member",
                    "sku": "csrf-product",
                    "quantity": 1,
                    "total": 1
                })
                .to_string(),
            ),
            (
                "/checkout",
                serde_json::json!({
                    "handle": "csrf-member",
                    "sku": "csrf-product",
                    "quantity": 1
                })
                .to_string(),
            ),
            (
                "/payments",
                serde_json::json!({
                    "orderId": 1,
                    "amount": 1,
                    "method": "card"
                })
                .to_string(),
            ),
            (
                "/shipments",
                serde_json::json!({
                    "orderId": 1,
                    "carrier": "local",
                    "address": "Seoul"
                })
                .to_string(),
            ),
        ];
        for (path, body) in csrf_required_routes {
            let (status, _, response_body) = send_request(addr, "POST", path, Some(body)).await;
            assert_eq!(status, 403, "{path} should require csrf");
            let rejection: serde_json::Value =
                serde_json::from_slice(&response_body).expect("csrf rejection json");
            assert_eq!(
                rejection["err"],
                serde_json::json!("csrf_token_required"),
                "{path} should return csrf rejection"
            );
        }

        let (missing_admin_status, _, missing_admin_body) =
            send_request(addr, "GET", "/admin", None).await;
        assert_eq!(missing_admin_status, 401);
        let missing_admin: serde_json::Value =
            serde_json::from_slice(&missing_admin_body).expect("missing admin auth json");
        assert_eq!(missing_admin["err"], serde_json::json!("auth_required"));

        let admin_login_payload = serde_json::json!({
            "handle": "admin",
            "email": "admin@example.test",
            "password": "admin-reference-password"
        })
        .to_string();
        let (admin_login_status, _, _, admin_login_headers, admin_login_body) =
            send_request_full_with_headers(
                addr,
                "POST",
                "/members/login",
                Some(admin_login_payload),
                &[
                    ("cookie", csrf_cookie_pair.as_str()),
                    ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
                ],
            )
            .await;
        assert_eq!(admin_login_status, 201);
        let admin_login: serde_json::Value =
            serde_json::from_slice(&admin_login_body).expect("admin login json");
        assert_eq!(admin_login["session"]["role"], serde_json::json!("admin"));
        let admin_cookie = admin_login_headers
            .get("set-cookie")
            .expect("admin login set-cookie");
        assert!(admin_cookie.contains("orv_session_role=admin"));
        let admin_cookie_header = cookie_header_from_set_cookie(admin_cookie);

        let (admin_status, admin_ct, admin_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_status, 200);
        assert_eq!(admin_ct.as_deref(), Some("text/html; charset=utf-8"));
        let admin_html = String::from_utf8(admin_body).expect("admin html");
        assert!(admin_html.contains("<h1>Miol Shop Admin</h1>"));
        assert!(admin_html.contains("Operations dashboard"));
        assert!(admin_html.contains("<a href=\"/admin/catalog\">Catalog read model</a>"));
        assert!(admin_html.contains("<a href=\"/admin/summary\">Operations summary</a>"));
        assert!(admin_html.contains("<a href=\"/admin/orders\">Order read model</a>"));
        assert!(admin_html.contains("<a href=\"/admin/payments\">Payment read model</a>"));
        assert!(admin_html.contains("<a href=\"/admin/shipments\">Shipment read model</a>"));
        assert!(admin_html.contains("<a href=\"/admin/webhooks\">Webhook read model</a>"));
        assert!(admin_html.contains("<a href=\"/admin/audit\">Audit read model</a>"));
        assert!(admin_html.contains("Stripe webhook events: POST /webhooks/stripe"));
        assert!(admin_html.contains("data/shop.sqlite"));

        let (health_status, _, health_body) = send_request(addr, "GET", "/health", None).await;
        assert_eq!(health_status, 200);
        let health: serde_json::Value = serde_json::from_slice(&health_body).expect("health json");
        assert_eq!(health["ok"], serde_json::json!(true));

        let product_payload = serde_json::json!({
            "sku": "kettle",
            "name": "Kettle",
            "badge": "Featured",
            "price": 25000,
            "stock": 2
        })
        .to_string();
        let (create_product_status, _, create_product_body) = send_request_with_headers(
            addr,
            "POST",
            "/products",
            Some(product_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(create_product_status, 201);
        let created_product: serde_json::Value =
            serde_json::from_slice(&create_product_body).expect("product json");
        assert_eq!(created_product["product"]["id"], serde_json::json!(1));
        assert_eq!(
            created_product["product"]["sku"],
            serde_json::json!("kettle")
        );

        let (form_product_status, _, form_product_body) =
            send_request_with_content_type_and_headers(
                addr,
                "POST",
                "/products",
                "sku=mug&name=Mug&badge=Counter&price=1200&stock=3&_csrf=orv-reference-csrf"
                    .to_string(),
                "application/x-www-form-urlencoded",
                &[("cookie", csrf_cookie_pair.as_str())],
            )
            .await;
        assert_eq!(form_product_status, 201);
        let form_product: serde_json::Value =
            serde_json::from_slice(&form_product_body).expect("form product json");
        assert_eq!(form_product["product"]["sku"], serde_json::json!("mug"));
        assert_eq!(form_product["product"]["stock"], serde_json::json!(3));

        let (list_status, _, list_body) = send_request(addr, "GET", "/products", None).await;
        assert_eq!(list_status, 200);
        let list: serde_json::Value = serde_json::from_slice(&list_body).expect("list json");
        assert_eq!(list["products"].as_array().map(Vec::len), Some(2));
        assert_eq!(list["products"][0]["name"], serde_json::json!("Kettle"));

        let (catalog_status, catalog_ct, catalog_body) =
            send_request(addr, "GET", "/catalog", None).await;
        assert_eq!(catalog_status, 200);
        assert_eq!(catalog_ct.as_deref(), Some("text/html; charset=utf-8"));
        let catalog_html = String::from_utf8(catalog_body).expect("catalog html");
        assert!(catalog_html.contains("<h1>Shop Catalog</h1>"));
        assert!(catalog_html.contains("Kettle"));
        assert!(catalog_html.contains("Mug"));
        assert!(catalog_html.contains("stock 3"));

        let (admin_catalog_status, admin_catalog_ct, admin_catalog_body) =
            send_request_with_headers(
                addr,
                "GET",
                "/admin/catalog",
                None,
                &[("cookie", admin_cookie_header.as_str())],
            )
            .await;
        assert_eq!(admin_catalog_status, 200);
        assert_eq!(
            admin_catalog_ct.as_deref(),
            Some("text/html; charset=utf-8")
        );
        let admin_catalog_html = String::from_utf8(admin_catalog_body).expect("admin catalog html");
        assert!(admin_catalog_html.contains("<h1>Catalog</h1>"));
        assert!(admin_catalog_html.contains("kettle: Kettle / Featured / stock 2"));
        assert!(admin_catalog_html.contains("mug: Mug / Counter / stock 3"));

        let (form_order_status, _, form_order_body) = send_request_with_content_type_and_headers(
            addr,
            "POST",
            "/orders",
            "customer=bea&sku=mug&quantity=2&total=2400&_csrf=orv-reference-csrf".to_string(),
            "application/x-www-form-urlencoded",
            &[("cookie", csrf_cookie_pair.as_str())],
        )
        .await;
        assert_eq!(form_order_status, 201);
        let form_order: serde_json::Value =
            serde_json::from_slice(&form_order_body).expect("form order json");
        assert_eq!(form_order["order"]["customer"], serde_json::json!("bea"));
        assert_eq!(form_order["order"]["quantity"], serde_json::json!(2));
        assert_eq!(form_order["remainingStock"], serde_json::json!(1));

        let order_payload = serde_json::json!({
            "customer": "ada",
            "sku": "kettle",
            "quantity": 1,
            "total": 25000
        })
        .to_string();
        let (create_order_status, _, create_order_body) = send_request_with_headers(
            addr,
            "POST",
            "/orders",
            Some(order_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(create_order_status, 201);
        let created_order: serde_json::Value =
            serde_json::from_slice(&create_order_body).expect("order json");
        assert_eq!(
            created_order["order"]["status"],
            serde_json::json!("reserved")
        );
        assert_eq!(created_order["order"]["total"], serde_json::json!(25000));
        assert_eq!(created_order["remainingStock"], serde_json::json!(1));
        let order_id = created_order["order"]["id"].as_i64().expect("order id");

        let (find_order_status, _, find_order_body) =
            send_request(addr, "GET", "/orders/ada", None).await;
        assert_eq!(find_order_status, 200);
        let found_order: serde_json::Value =
            serde_json::from_slice(&find_order_body).expect("found order json");
        assert_eq!(found_order["order"]["customer"], serde_json::json!("ada"));
        assert_eq!(found_order["order"]["sku"], serde_json::json!("kettle"));

        let (find_product_status, _, find_product_body) =
            send_request(addr, "GET", "/products/kettle", None).await;
        assert_eq!(find_product_status, 200);
        let found_product: serde_json::Value =
            serde_json::from_slice(&find_product_body).expect("found product json");
        assert_eq!(found_product["product"]["stock"], serde_json::json!(1));

        let oversell_payload = serde_json::json!({
            "customer": "grace",
            "sku": "kettle",
            "quantity": 2,
            "total": 50000
        })
        .to_string();
        let (oversell_status, _, oversell_body) = send_request_with_headers(
            addr,
            "POST",
            "/orders",
            Some(oversell_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(oversell_status, 409);
        let oversell: serde_json::Value =
            serde_json::from_slice(&oversell_body).expect("oversell json");
        assert_eq!(oversell["err"], serde_json::json!("out_of_stock"));
        assert_eq!(oversell["stock"], serde_json::json!(1));

        let member_payload = serde_json::json!({
            "handle": "ada",
            "name": "Ada Lovelace",
            "email": "ada@example.test",
            "password": "correct horse battery staple"
        })
        .to_string();
        let (create_member_status, _, create_member_body) = send_request_with_headers(
            addr,
            "POST",
            "/members",
            Some(member_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(create_member_status, 201);
        let created_member: serde_json::Value =
            serde_json::from_slice(&create_member_body).expect("member json");
        assert_eq!(created_member["member"]["handle"], serde_json::json!("ada"));

        let (find_member_status, _, find_member_body) =
            send_request(addr, "GET", "/members/ada", None).await;
        assert_eq!(find_member_status, 200);
        let found_member: serde_json::Value =
            serde_json::from_slice(&find_member_body).expect("found member json");
        assert_eq!(
            found_member["member"]["email"],
            serde_json::json!("ada@example.test")
        );
        assert_ne!(
            found_member["member"]["passwordHash"],
            serde_json::json!("correct horse battery staple")
        );
        assert!(found_member["member"]["passwordHash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("$argon2")));

        let wrong_login_payload = serde_json::json!({
            "handle": "ada",
            "email": "ada@example.test",
            "password": "wrong password"
        })
        .to_string();
        let (wrong_login_status, _, wrong_login_body) = send_request_with_headers(
            addr,
            "POST",
            "/members/login",
            Some(wrong_login_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(wrong_login_status, 401);
        let wrong_login: serde_json::Value =
            serde_json::from_slice(&wrong_login_body).expect("wrong login json");
        assert_eq!(
            wrong_login["err"],
            serde_json::json!("invalid_member_login")
        );

        let login_payload = serde_json::json!({
            "handle": "ada",
            "email": "ada@example.test",
            "password": "correct horse battery staple"
        })
        .to_string();
        let (login_status, _, _, login_headers, login_body) = send_request_full_with_headers(
            addr,
            "POST",
            "/members/login",
            Some(login_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(login_status, 201);
        let login_cookie = login_headers.get("set-cookie").expect("login set-cookie");
        assert!(login_cookie.contains("orv_session=2"));
        assert!(login_cookie.contains("orv_session_role=member"));
        assert!(login_cookie.contains("Path=/"));
        assert!(login_cookie.contains("Max-Age=86400"));
        assert!(login_cookie.contains("HttpOnly"));
        assert!(login_cookie.contains("SameSite=Lax"));
        assert!(login_cookie.contains("Secure"));
        let login_cookie_header = cookie_header_from_set_cookie(login_cookie);
        let login: serde_json::Value = serde_json::from_slice(&login_body).expect("login json");
        assert_eq!(login["session"]["handle"], serde_json::json!("ada"));
        assert_eq!(login["session"]["status"], serde_json::json!("active"));
        assert_eq!(login["session"]["role"], serde_json::json!("member"));

        let (member_admin_status, _, member_admin_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin",
            None,
            &[("cookie", login_cookie_header.as_str())],
        )
        .await;
        assert_eq!(member_admin_status, 403);
        let member_admin: serde_json::Value =
            serde_json::from_slice(&member_admin_body).expect("member admin auth json");
        assert_eq!(member_admin["err"], serde_json::json!("role_required"));
        assert_eq!(member_admin["requiredRole"], serde_json::json!("admin"));

        let cart_payload = serde_json::json!({
            "handle": "ada",
            "sku": "mug",
            "quantity": 1
        })
        .to_string();
        let (cart_item_status, _, cart_item_body) = send_request_with_headers(
            addr,
            "POST",
            "/cart/items",
            Some(cart_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(cart_item_status, 201);
        let cart_item: serde_json::Value =
            serde_json::from_slice(&cart_item_body).expect("cart item json");
        assert_eq!(cart_item["cartItem"]["handle"], serde_json::json!("ada"));
        assert_eq!(cart_item["cartItem"]["sku"], serde_json::json!("mug"));
        assert_eq!(cart_item["cartItem"]["quantity"], serde_json::json!(1));

        let (cart_status, cart_ct, cart_body) = send_request(addr, "GET", "/cart", None).await;
        assert_eq!(cart_status, 200);
        assert_eq!(cart_ct.as_deref(), Some("text/html; charset=utf-8"));
        let cart_html = String::from_utf8(cart_body).expect("cart html");
        assert!(cart_html.contains("<h1>Cart</h1>"));
        assert!(cart_html.contains("ada"));
        assert!(cart_html.contains("mug"));
        assert!(cart_html.contains("quantity 1"));

        let (missing_sessions_status, _, missing_sessions_body) =
            send_request(addr, "GET", "/account/sessions", None).await;
        assert_eq!(missing_sessions_status, 401);
        let missing_sessions: serde_json::Value =
            serde_json::from_slice(&missing_sessions_body).expect("missing sessions json");
        assert_eq!(
            missing_sessions["err"],
            serde_json::json!("session_required")
        );

        let (sessions_status, sessions_ct, sessions_body) = send_request_with_headers(
            addr,
            "GET",
            "/account/sessions",
            None,
            &[("cookie", login_cookie_header.as_str())],
        )
        .await;
        assert_eq!(sessions_status, 200);
        assert_eq!(sessions_ct.as_deref(), Some("text/html; charset=utf-8"));
        let sessions_html = String::from_utf8(sessions_body).expect("sessions html");
        assert!(sessions_html.contains("<h1>Account Sessions</h1>"));
        assert!(sessions_html.contains("ada"));
        assert!(sessions_html.contains("active"));

        let payment_payload = serde_json::json!({
            "orderId": order_id,
            "amount": 25000,
            "method": "card"
        })
        .to_string();
        let (payment_status, _, payment_body) = send_request_with_headers(
            addr,
            "POST",
            "/payments",
            Some(payment_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(payment_status, 201);
        let payment: serde_json::Value =
            serde_json::from_slice(&payment_body).expect("payment json");
        assert_eq!(payment["payment"]["status"], serde_json::json!("captured"));
        assert_eq!(payment["payment"]["provider"], serde_json::json!("file"));
        assert_eq!(payment["order"]["status"], serde_json::json!("paid"));

        let shipment_payload = serde_json::json!({
            "orderId": order_id,
            "carrier": "post",
            "address": "Seoul"
        })
        .to_string();
        let (shipment_status, _, shipment_body) = send_request_with_headers(
            addr,
            "POST",
            "/shipments",
            Some(shipment_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(shipment_status, 201);
        let shipment: serde_json::Value =
            serde_json::from_slice(&shipment_body).expect("shipment json");
        assert_eq!(shipment["shipment"]["status"], serde_json::json!("ready"));
        assert_eq!(shipment["shipment"]["provider"], serde_json::json!("file"));
        assert_eq!(shipment["order"]["status"], serde_json::json!("shipped"));

        let shipment_path = format!("/shipments/{order_id}");
        let (find_shipment_status, _, find_shipment_body) =
            send_request(addr, "GET", &shipment_path, None).await;
        assert_eq!(find_shipment_status, 200);
        let found_shipment: serde_json::Value =
            serde_json::from_slice(&find_shipment_body).expect("found shipment json");
        assert_eq!(
            found_shipment["shipment"]["tracking"],
            serde_json::json!("TRK-LOCAL")
        );

        crate::interp::test_env::set("STRIPE_WEBHOOK_SECRET", "whsec_test");
        crate::interp::test_env::set("STRIPE_WEBHOOK_TOLERANCE_SECONDS", "999999999");
        let webhook_payload = r#"{"id":"evt_1"}"#.to_string();
        let webhook_signature =
            "t=1700000000,v1=c89214b5b5da833daed6f0b8c5bb6bd58cea9022bd80ccc78230f3942d632925";
        let (webhook_status, _, webhook_body) = send_request_with_headers(
            addr,
            "POST",
            "/webhooks/stripe",
            Some(webhook_payload.clone()),
            &[("stripe-signature", webhook_signature)],
        )
        .await;
        assert_eq!(webhook_status, 202);
        let webhook: serde_json::Value =
            serde_json::from_slice(&webhook_body).expect("webhook json");
        assert_eq!(webhook["duplicate"], serde_json::json!(false));
        assert_eq!(
            webhook["verification"]["status"],
            serde_json::json!("verified")
        );
        assert_eq!(webhook["webhook"]["provider"], serde_json::json!("stripe"));
        assert_eq!(webhook["webhook"]["status"], serde_json::json!("verified"));
        assert_eq!(webhook["webhook"]["eventId"], serde_json::json!("evt_1"));

        let (duplicate_webhook_status, _, duplicate_webhook_body) = send_request_with_headers(
            addr,
            "POST",
            "/webhooks/stripe",
            Some(webhook_payload),
            &[("stripe-signature", webhook_signature)],
        )
        .await;
        crate::interp::test_env::clear("STRIPE_WEBHOOK_SECRET");
        assert_eq!(duplicate_webhook_status, 200);
        let duplicate_webhook: serde_json::Value =
            serde_json::from_slice(&duplicate_webhook_body).expect("duplicate webhook json");
        assert_eq!(duplicate_webhook["duplicate"], serde_json::json!(true));
        assert_eq!(
            duplicate_webhook["webhook"]["eventId"],
            serde_json::json!("evt_1")
        );

        let checkout_payload = serde_json::json!({
            "handle": "ada",
            "sku": "mug",
            "quantity": 1,
            "total": 1200,
            "method": "card",
            "carrier": "post",
            "address": "Seoul"
        })
        .to_string();
        let (missing_checkout_status, _, missing_checkout_body) =
            send_request(addr, "POST", "/checkout", Some(checkout_payload.clone())).await;
        assert_eq!(missing_checkout_status, 403);
        let missing_checkout: serde_json::Value =
            serde_json::from_slice(&missing_checkout_body).expect("missing checkout json");
        assert_eq!(
            missing_checkout["err"],
            serde_json::json!("csrf_token_required")
        );

        let (checkout_status, _, checkout_body) = send_request_with_headers(
            addr,
            "POST",
            "/checkout",
            Some(checkout_payload),
            &[
                ("cookie", csrf_cookie_pair.as_str()),
                ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
            ],
        )
        .await;
        assert_eq!(checkout_status, 201);
        let checkout: serde_json::Value =
            serde_json::from_slice(&checkout_body).expect("checkout json");
        assert_eq!(checkout["order"]["customer"], serde_json::json!("ada"));
        assert_eq!(checkout["order"]["status"], serde_json::json!("shipped"));
        assert_eq!(checkout["payment"]["status"], serde_json::json!("captured"));
        assert_eq!(
            checkout["shipment"]["tracking"],
            serde_json::json!("TRK-LOCAL")
        );
        let checkout_order_id = checkout["order"]["id"].as_i64().expect("checkout order id");

        crate::interp::test_env::set("STRIPE_WEBHOOK_SECRET", "whsec_test");
        let reconciliation_payload = serde_json::json!({
            "id": "evt_checkout_paid",
            "orderId": checkout_order_id,
            "paymentStatus": "provider_paid",
            "orderStatus": "provider_reconciled"
        })
        .to_string();
        let reconciliation_signature =
            stripe_test_signature("whsec_test", "1700000001", &reconciliation_payload);
        let (reconciliation_status, _, reconciliation_body) = send_request_with_headers(
            addr,
            "POST",
            "/webhooks/stripe",
            Some(reconciliation_payload),
            &[("stripe-signature", &reconciliation_signature)],
        )
        .await;
        crate::interp::test_env::clear("STRIPE_WEBHOOK_SECRET");
        crate::interp::test_env::clear("STRIPE_WEBHOOK_TOLERANCE_SECONDS");
        assert_eq!(reconciliation_status, 202);
        let reconciliation: serde_json::Value =
            serde_json::from_slice(&reconciliation_body).expect("reconciliation json");
        assert_eq!(
            reconciliation["reconciledPayment"]["status"],
            serde_json::json!("provider_paid")
        );
        assert_eq!(
            reconciliation["reconciledOrder"]["status"],
            serde_json::json!("provider_reconciled")
        );

        let (admin_orders_status, _, admin_orders_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/orders",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_orders_status, 200);
        let admin_orders_html = String::from_utf8(admin_orders_body).expect("orders html utf8");
        assert!(admin_orders_html.contains("ada"));
        assert!(admin_orders_html.contains("provider_reconciled"));

        let (admin_payments_status, _, admin_payments_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/payments",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_payments_status, 200);
        let admin_payments_html =
            String::from_utf8(admin_payments_body).expect("payments html utf8");
        assert!(admin_payments_html.contains("captured"));
        assert!(admin_payments_html.contains("provider_paid"));
        assert!(admin_payments_html.contains("file"));

        let (admin_shipments_status, _, admin_shipments_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/shipments",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_shipments_status, 200);
        let admin_shipments_html =
            String::from_utf8(admin_shipments_body).expect("shipments html utf8");
        assert!(admin_shipments_html.contains("TRK-LOCAL"));

        let (admin_webhooks_status, _, admin_webhooks_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/webhooks",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_webhooks_status, 200);
        let admin_webhooks_html =
            String::from_utf8(admin_webhooks_body).expect("webhooks html utf8");
        assert!(admin_webhooks_html.contains("evt_1"));
        assert!(admin_webhooks_html.contains("evt_checkout_paid"));
        assert!(admin_webhooks_html.contains("verified"));

        let (admin_audit_status, _, admin_audit_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/audit",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(admin_audit_status, 200);
        let admin_audit_html = String::from_utf8(admin_audit_body).expect("audit html utf8");
        assert!(admin_audit_html.contains("checkout.complete"));
        assert!(admin_audit_html.contains("payment.capture"));
        assert!(admin_audit_html.contains("shipment.book"));
        assert!(admin_audit_html.contains("webhook.received"));

        let (summary_status, _, summary_body) = send_request_with_headers(
            addr,
            "GET",
            "/admin/summary",
            None,
            &[("cookie", admin_cookie_header.as_str())],
        )
        .await;
        assert_eq!(summary_status, 200);
        let summary: serde_json::Value =
            serde_json::from_slice(&summary_body).expect("admin summary json");
        assert_eq!(summary["products"], serde_json::json!(2));
        assert_eq!(summary["members"], serde_json::json!(2));
        assert_eq!(summary["orders"], serde_json::json!(3));
        assert_eq!(summary["payments"], serde_json::json!(2));
        assert_eq!(summary["shipments"], serde_json::json!(2));
        assert_eq!(summary["webhookEvents"], serde_json::json!(2));
        assert_eq!(summary["auditEvents"], serde_json::json!(16));

        handle.abort();

        let restored = crate::db::InMemoryDb::load_sqlite(&sqlite_path)
            .expect("reload shopping fixture sqlite");
        let snapshot = restored.snapshot_json();
        let member_rows = snapshot["tables"]["Member"]["rows"]
            .as_array()
            .expect("member rows");
        assert_eq!(member_rows.len(), 2);
        assert!(member_rows
            .iter()
            .any(|member| member["handle"] == "admin" && member["role"] == "admin"));
        assert!(member_rows
            .iter()
            .any(|member| member["handle"] == "ada" && member["role"] == "member"));
        assert_eq!(
            snapshot["tables"]["Order"]["rows"].as_array().map(Vec::len),
            Some(3)
        );
        assert_eq!(
            snapshot["tables"]["Payment"]["rows"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            snapshot["tables"]["Shipment"]["rows"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            snapshot["tables"]["Shipment"]["rows"][0]["tracking"],
            serde_json::json!("TRK-LOCAL")
        );
        assert_eq!(
            snapshot["tables"]["WebhookEvent"]["rows"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            snapshot["tables"]["WebhookEvent"]["rows"][0]["eventId"],
            serde_json::json!("evt_1")
        );
        assert_eq!(
            snapshot["tables"]["AuditEvent"]["rows"]
                .as_array()
                .map(Vec::len),
            Some(16)
        );
        let audit_rows = snapshot["tables"]["AuditEvent"]["rows"]
            .as_array()
            .expect("audit rows");
        assert!(audit_rows
            .iter()
            .any(|event| event["kind"] == "product.create"));
        assert!(audit_rows
            .iter()
            .any(|event| event["kind"] == "session.login" && event["actor"] == "admin"));
        let payment_records = std::fs::read_to_string(&payment_path).expect("payment record log");
        let shipping_records =
            std::fs::read_to_string(&shipping_path).expect("shipping record log");
        assert!(payment_records.contains(r#""kind":"payment.capture""#));
        assert!(payment_records.contains(&format!(r#""orderId":{order_id}"#)));
        assert!(shipping_records.contains(r#""kind":"shipping.booking""#));
        assert!(shipping_records.contains(r#""tracking":"TRK-LOCAL""#));
        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(payment_path);
        let _ = std::fs::remove_file(shipping_path);
    })
    .await;
}

#[tokio::test]
async fn fixture_shopping_mall_records_checkout_compensation_when_shipping_fails() {
    run_on_localset(async {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let sqlite_path = std::env::temp_dir().join(format!(
            "orv-shopping-compensation-{}-{unique}.sqlite",
            std::process::id()
        ));
        let payment_path = std::env::temp_dir().join(format!(
            "orv-shopping-compensation-payments-{}-{unique}.jsonl",
            std::process::id()
        ));
        let shipping_path = std::env::temp_dir().join(format!(
            "orv-shopping-compensation-shipments-{}-{unique}.jsonl",
            std::process::id()
        ));
        let src = read_e2e_fixture("shopping_mall.orv")
            .replace(
                "sqlite://data/shop.sqlite",
                &format!("sqlite://{}", sqlite_path.display()),
            )
            .replace(
                "file://data/payments.jsonl",
                &format!("file://{}", payment_path.display()),
            )
            .replace(
                "file://data/shipments.jsonl",
                &format!("file://{}", shipping_path.display()),
            );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (home_status, _, _, home_headers, _) = send_request_full(addr, "GET", "/", None).await;
        assert_eq!(home_status, 200);
        let csrf_cookie = home_headers
            .get("set-cookie")
            .expect("home csrf set-cookie");
        let csrf_cookie_pair = csrf_cookie
            .split(';')
            .next()
            .expect("csrf cookie pair")
            .to_string();
        let csrf_headers = [
            ("cookie", csrf_cookie_pair.as_str()),
            ("x-csrf-token", ORV_REFERENCE_CSRF_TOKEN),
        ];

        let product_payload = serde_json::json!({
            "sku": "mug",
            "name": "Mug",
            "badge": "Compensation",
            "price": 1200,
            "stock": 1
        })
        .to_string();
        let (product_status, _, _) = send_request_with_headers(
            addr,
            "POST",
            "/products",
            Some(product_payload),
            &csrf_headers,
        )
        .await;
        assert_eq!(product_status, 201);

        let member_payload = serde_json::json!({
            "handle": "ada",
            "name": "Ada Lovelace",
            "email": "ada@example.test",
            "password": "correct horse battery staple"
        })
        .to_string();
        let (member_status, _, _) = send_request_with_headers(
            addr,
            "POST",
            "/members",
            Some(member_payload),
            &csrf_headers,
        )
        .await;
        assert_eq!(member_status, 201);

        let (provider_addr, provider_requests, provider_server) =
            spawn_checkout_shipping_failure_server();
        crate::interp::test_env::set("SHIPPING_ADAPTER_URL", "carrier://local");
        crate::interp::test_env::set(
            "CARRIER_API_ENDPOINT",
            &format!("http://{provider_addr}/carrier/shipments"),
        );
        crate::interp::test_env::set("CARRIER_API_KEY", "carrier_compensation_secret");

        let checkout_payload = serde_json::json!({
            "handle": "ada",
            "sku": "mug",
            "quantity": 1,
            "total": 1200,
            "method": "card",
            "carrier": "post",
            "address": "Seoul"
        })
        .to_string();
        let (checkout_status, _, checkout_body) = send_request_with_headers(
            addr,
            "POST",
            "/checkout",
            Some(checkout_payload),
            &csrf_headers,
        )
        .await;
        crate::interp::test_env::clear("SHIPPING_ADAPTER_URL");
        crate::interp::test_env::clear("CARRIER_API_ENDPOINT");
        crate::interp::test_env::clear("CARRIER_API_KEY");
        provider_server
            .join()
            .expect("shipping failure server finished");

        assert_eq!(checkout_status, 202);
        let checkout: serde_json::Value =
            serde_json::from_slice(&checkout_body).expect("checkout compensation json");
        assert_eq!(
            checkout["order"]["status"],
            serde_json::json!("payment_captured_pending_shipment")
        );
        assert_eq!(checkout["payment"]["status"], serde_json::json!("captured"));
        assert_eq!(checkout["shipment"], serde_json::Value::Null);
        assert_eq!(
            checkout["compensation"]["required"],
            serde_json::json!(true)
        );

        let requests = provider_requests.lock().expect("provider requests").clone();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| request.contains("idempotency-key: carrier.shipment.create:1")));
        assert!(requests
            .iter()
            .all(|request| request.contains("authorization: Bearer carrier_compensation_secret")));
        assert!(requests
            .iter()
            .all(|request| request.contains(r#""kind":"carrier.shipment.create""#)));

        let (product_status, _, product_body) =
            send_request(addr, "GET", "/products/mug", None).await;
        assert_eq!(product_status, 200);
        let product: serde_json::Value =
            serde_json::from_slice(&product_body).expect("product json");
        assert_eq!(product["product"]["stock"], serde_json::json!(0));

        handle.abort();

        let restored =
            crate::db::InMemoryDb::load_sqlite(&sqlite_path).expect("reload compensation sqlite");
        let snapshot = restored.snapshot_json();
        let orders = snapshot_table_rows(&snapshot, "Order");
        assert_eq!(orders.len(), 1);
        assert_eq!(
            orders[0]["status"],
            serde_json::json!("payment_captured_pending_shipment")
        );
        assert_eq!(snapshot_table_rows(&snapshot, "Payment").len(), 1);
        assert_eq!(snapshot_table_rows(&snapshot, "Shipment").len(), 0);
        let audit_rows = snapshot_table_rows(&snapshot, "AuditEvent");
        assert!(audit_rows
            .iter()
            .any(|event| event["kind"] == "checkout.compensation_required"
                && event["status"] == "payment_captured_pending_shipment"));
        assert!(!audit_rows
            .iter()
            .any(|event| event["kind"] == "checkout.complete"));

        let payment_records = std::fs::read_to_string(&payment_path).expect("payment record log");
        assert!(payment_records.contains(r#""kind":"payment.capture""#));
        assert!(payment_records.contains(r#""status":"captured""#));
        assert!(!payment_records.contains("carrier_compensation_secret"));

        let _ = std::fs::remove_file(sqlite_path);
        let _ = std::fs::remove_file(payment_path);
        let _ = std::fs::remove_file(shipping_path);
    })
    .await;
}

#[tokio::test]
async fn server_body_wal_persists_route_db_mutations() {
    run_on_localset(async {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "orv-server-db-wal-{}-{unique}.jsonl",
            std::process::id()
        ));
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&format!(
            r#"@server {{
                    @listen 0
                    @db.wal "{}"
                    @route POST /members {{
                        let member = await @db.create("Member", @body)
                        @respond 201 {{ member: member }}
                    }}
                }}"#,
            path.display()
        ));
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let payload = serde_json::json!({
            "handle": "ada",
            "email": "ada@example.test"
        })
        .to_string();
        let (status, _, body) = send_request(addr, "POST", "/members", Some(payload)).await;
        assert_eq!(status, 201);
        let created: serde_json::Value = serde_json::from_slice(&body).expect("member json");
        assert_eq!(created["member"]["handle"], serde_json::json!("ada"));
        handle.abort();

        let restored = crate::db::InMemoryDb::load_wal(&path).expect("replay server wal");
        let snapshot = restored.snapshot_json();
        assert_eq!(
            snapshot["tables"]["Member"]["rows"][0]["handle"],
            serde_json::json!("ada")
        );
        let _ = std::fs::remove_file(path);
    })
    .await;
}

#[tokio::test]
async fn fixture_catchall_boots_specific_route_and_wildcard_fallback() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_from_fixture("catchall.orv");
        assert_eq!(body_stmts.len(), 1, "catchall.orv has one boot @out");
        let (addr, handle, boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 부트 출력 — C5c 의 body_stmts 패치가 실제로 런타임에 도달하는지
        // 검증. `@out` 은 줄바꿈을 붙여 기록한다.
        let boot_str = String::from_utf8(boot).expect("utf-8");
        assert_eq!(boot_str, "boot ok\n");

        // 1) 구체 라우트가 catchall 보다 먼저 매치
        let (s1, _, b1) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(s1, 200);
        let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
        assert_eq!(j1["hit"], serde_json::json!("ping"));

        // 2) 그 외 경로는 `@route GET *` 이 잡아 404
        let (s2, _, b2) = send_request(addr, "GET", "/unknown/path", None).await;
        assert_eq!(s2, 404);
        let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
        assert_eq!(j2["err"], serde_json::json!("not found"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn fixture_middleware_accumulates_context_and_runs_after() {
    // C_middleware: `@Inject` (@before) 가 @next 로 context 에 값을 쌓고
    // `@Audit` (@after) 가 handler 뒤에 실행된다. `@respond` payload 는
    // `@context.role`/`@context.uid` 를 읽어 검증. `@after` 의 stdout 출력은
    // hyper 경로에서 sink 로 버려지므로(보수적 MVP) 응답 바디만 본다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_from_fixture("middleware.orv");
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, ct, body) = send_request(addr, "GET", "/me", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["role"], serde_json::json!("admin"));
        assert_eq!(json["uid"], serde_json::json!(42));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn fixture_domains_exercises_reference_runtime_stubs() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_from_fixture("domains.orv");
        assert!(body_stmts.is_empty(), "domains.orv has no boot stmts");
        let (addr, handle, boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");
        assert!(boot.is_empty(), "domains.orv should produce no boot output");

        let (status, ct, body) = send_request(addr, "GET", "/domains", None).await;
        assert_eq!(status, 200);
        assert_eq!(ct.as_deref(), Some("application/json"));
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["chunkSize"], serde_json::json!(5));
        assert_eq!(json["path"], serde_json::json!("files/upload-1.txt"));
        assert_eq!(
            json["url"],
            serde_json::json!("/orv-storage/files/upload-1.txt?signed=1")
        );
        assert_eq!(json["job"], serde_json::json!("queued"));
        assert_eq!(json["videoId"], serde_json::json!("upload-1"));
        assert_eq!(json["doc"], serde_json::json!("42"));
        assert_eq!(json["mail"], serde_json::json!(true));
        assert_eq!(json["media"], serde_json::json!("camera"));
        assert_eq!(json["upload"], serde_json::json!("upload-1"));
        assert_eq!(json["push"], serde_json::json!(true));
        assert_eq!(
            json["subscription"],
            serde_json::json!("push://subscription")
        );
        assert_eq!(json["sent"], serde_json::json!("sent"));
        assert_eq!(json["cache"], serde_json::json!("assets-v1"));
        assert_eq!(json["cached"], serde_json::json!("stored"));
        assert_eq!(json["loaded"], serde_json::json!("code"));
        assert_eq!(json["local"], serde_json::json!("logo"));
        assert_eq!(json["tun"], serde_json::json!("orv0"));
        assert_eq!(json["packetBytes"], serde_json::json!(6));
        assert_eq!(
            json["plugin"],
            serde_json::json!("ext/markdown-preview.wasm")
        );
        assert_eq!(json["activation"], serde_json::json!("activated"));
        assert_eq!(json["compute"], serde_json::json!("compute"));
        assert_eq!(json["observability"], serde_json::json!("superapp"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn handlers_can_use_top_level_function_bindings() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"function helper() -> "pong"

                @server {
                    @listen 0
                    @route GET /ping { @respond 200 { msg: helper() } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["msg"], serde_json::json!("pong"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn handlers_can_use_server_level_function_bindings() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    function helper() -> "pong"
                    @route GET /ping { @respond 200 { msg: helper() } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["msg"], serde_json::json!("pong"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn shutdown_signal_stops_accept_loop_gracefully() {
    // A4: graceful shutdown.
    //
    // 시나리오:
    //   1) 서버 기동 → 첫 요청 200 확인
    //   2) shutdown 채널에 `()` 전송
    //   3) `handle.await` 가 정상 종료 (Ok, not aborted)
    //   4) 같은 주소로 재연결 시도 → listener 닫혀 연결 실패
    //
    // `handle.abort()` 가 아니라 자연 종료 경로라는 점이 핵심. in-flight
    // 연결이 있어도 serve_loop 는 select 에서 빠져나오기만 하고, 이미
    // accept 된 커넥션은 `serve_connection.await` 안에서 자연 완료된다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /ping { @respond 200 { ok: true } }
                }"#,
        );
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .expect("spawn");

        // 1) 첫 요청 — 서버 정상 동작 확인
        let (s1, _, _) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(s1, 200);

        // 2) shutdown 신호 → 3) 루프가 자연 종료해야 handle.await 가 완료됨
        let _ = shutdown_tx.send(());
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("serve_loop did not exit within timeout")
            .expect("join handle err");

        // 4) 리스너 닫혔으니 재연결 실패. 일부 OS 는 TIME_WAIT 상태로
        //    잠깐 연결을 받아줄 수 있으므로 에러 자체를 강제하기보다
        //    "핸들이 끝났다" 까지가 primary assertion. 연결 시도는
        //    정상 경로 smoke check.
        let probe = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            TcpStream::connect(addr),
        )
        .await;
        match probe {
            Ok(Ok(_)) => {
                // 연결은 맺혔지만 accept 가 닫혀 요청 처리 불가.
                // 여기까지는 OS TCP 스택 거동이라 허용.
            }
            Ok(Err(_)) | Err(_) => {
                // ConnectionRefused 또는 timeout — 기대 경로.
            }
        }
    })
    .await;
}

#[tokio::test]
async fn attached_server_handle_serves_until_drop() {
    let hir = lower_src(
        r"@server {
                @listen 0
                @route GET /ping { @respond 200 { ok: true } }
            }",
    );
    let server = spawn_attached_server(hir).expect("spawn attached server");
    let addr = server.addr();

    let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(status, 200);
    assert_eq!(json["ok"], serde_json::json!(true));

    drop(server);
    let probe = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        TcpStream::connect(addr),
    )
    .await;
    if let Ok(Ok(_)) = probe {
        panic!("attached server still accepted connections after drop");
    }
}

#[tokio::test]
async fn attached_server_prefix_wal_persists_route_db_mutations() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "orv-attached-db-wal-{}-{unique}.jsonl",
        std::process::id()
    ));
    let hir = lower_src(&format!(
        r#"@db.wal "{}"
            @server {{
                @listen 0
                @route POST /members {{
                    let member = await @db.create("Member", @body)
                    @respond 201 {{ member: member }}
                }}
            }}"#,
        path.display()
    ));
    let server = spawn_attached_server(hir).expect("spawn attached server");
    let addr = server.addr();

    let payload = serde_json::json!({
        "handle": "ada",
        "email": "ada@example.test"
    })
    .to_string();
    let (status, _, body) = send_request(addr, "POST", "/members", Some(payload)).await;
    assert_eq!(status, 201);
    let created: serde_json::Value = serde_json::from_slice(&body).expect("member json");
    assert_eq!(created["member"]["handle"], serde_json::json!("ada"));
    drop(server);

    let restored = crate::db::InMemoryDb::load_wal(&path).expect("replay attached wal");
    let snapshot = restored.snapshot_json();
    assert_eq!(
        snapshot["tables"]["Member"]["rows"][0]["handle"],
        serde_json::json!("ada")
    );
    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn serve_single_file_returns_bytes_and_mime() {
    // A5a: `@serve "path"` — 단일 파일 서빙. 파일 바이트 그대로 + 확장자
    // 기반 Content-Type 헤더. 이 테스트는 세 가지를 한 번에 검증한다:
    //
    //   1. HTML 확장자는 text/html charset=utf-8
    //   2. body bytes 가 파일 내용 그대로 (JSON 직렬화 안 됨)
    //   3. 바이너리 파일 (ICO) 은 image/x-icon
    run_on_localset(async {
        let tmp = std::env::temp_dir().join(format!("orv_serve_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("mktemp");
        let html_path = tmp.join("index.html");
        let ico_path = tmp.join("favicon.ico");
        std::fs::write(&html_path, b"<!doctype html><h1>hi</h1>").expect("write html");
        // ICO magic bytes — 단순 바이너리 검증용
        std::fs::write(&ico_path, [0u8, 0, 1, 0, 1, 0]).expect("write ico");

        let src = format!(
            r#"@server {{
                    @listen 0
                    @route GET /index.html {{ @serve "{}" }}
                    @route GET /favicon.ico {{ @serve "{}" }}
                }}"#,
            html_path.display(),
            ico_path.display()
        );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 1+2) HTML
        let (s_html, ct_html, b_html) = send_request(addr, "GET", "/index.html", None).await;
        assert_eq!(s_html, 200);
        assert_eq!(ct_html.as_deref(), Some("text/html; charset=utf-8"));
        assert_eq!(b_html, b"<!doctype html><h1>hi</h1>");

        // 3) ICO
        let (s_ico, ct_ico, b_ico) = send_request(addr, "GET", "/favicon.ico", None).await;
        assert_eq!(s_ico, 200);
        assert_eq!(ct_ico.as_deref(), Some("image/x-icon"));
        assert_eq!(b_ico, vec![0u8, 0, 1, 0, 1, 0]);

        handle.abort();
        std::fs::remove_dir_all(&tmp).ok();
    })
    .await;
}

#[tokio::test]
async fn nested_route_group_prefixes_match_flat() {
    // A2a E2E: `@route /admin { @route GET /users {...} }` 가 실제
    // HTTP 요청 `/admin/users` 에 매치되어야 한다. analyzer 의 unfold 가
    // runtime 매처까지 이어지는지 검증.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route /admin {
                        @route GET /users { @respond 200 { hit: "users" } }
                        @route GET /posts { @respond 200 { hit: "posts" } }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (s1, _, b1) = send_request(addr, "GET", "/admin/users", None).await;
        assert_eq!(s1, 200);
        let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
        assert_eq!(j1["hit"], serde_json::json!("users"));

        let (s2, _, b2) = send_request(addr, "GET", "/admin/posts", None).await;
        assert_eq!(s2, 200);
        let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
        assert_eq!(j2["hit"], serde_json::json!("posts"));

        // unjoin 경로는 매치 안 돼 404
        let (s3, _, _) = send_request(addr, "GET", "/users", None).await;
        assert_eq!(s3, 404);

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn group_middleware_applies_to_all_inner_routes() {
    // C_middleware 확장: `@route /admin { @Auth; @route ... }` 에서 `@Auth`
    // (@before) 가 내부 모든 route 의 handler 앞에 prepend 되어야 한다.
    // analyzer 의 `inherited_stmts` 경로가 middleware stmt 도 누적한다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    define Auth() -> @before { @next {user: "admin"} }
                    @route /admin {
                        @Auth
                        @route GET /users { @respond 200 { u: @context.user, kind: "users" } }
                        @route GET /posts { @respond 200 { u: @context.user, kind: "posts" } }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for (path, kind) in [("/admin/users", "users"), ("/admin/posts", "posts")] {
            let (status, _, body) = send_request(addr, "GET", path, None).await;
            assert_eq!(status, 200, "path {path}");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
            assert_eq!(json["u"], serde_json::json!("admin"), "path {path}");
            assert_eq!(json["kind"], serde_json::json!(kind), "path {path}");
        }

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn nested_group_middleware_stacks_outer_first() {
    // 중첩 그룹: outer 그룹의 middleware 가 inner 그룹 middleware 보다 먼저
    // 실행되어 context 누적 순서가 outer → inner 이어야 한다. `@next` 가
    // 같은 key 를 덮어쓰는 규칙(마지막 push 우세)과 결합해, inner 가 outer
    // 의 값을 override 할 수 있는지도 본다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    define Outer() -> @before { @next {scope: "outer", depth: 1} }
                    define Inner() -> @before { @next {scope: "inner"} }
                    @route /api {
                        @Outer
                        @route /v1 {
                            @Inner
                            @route GET /ping {
                                @respond 200 { scope: @context.scope, depth: @context.depth }
                            }
                        }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/api/v1/ping", None).await;
        assert_eq!(status, 200);
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        // inner 가 scope 을 override — 마지막 push 우세.
        assert_eq!(json["scope"], serde_json::json!("inner"));
        // depth 는 outer 에서만 push 되어 그대로 유지.
        assert_eq!(json["depth"], serde_json::json!(1));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_redirect_default_302() {
    // SPEC §11.9: `@redirect "/path"` → 302 + Location 헤더.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /old {
                        @redirect "/new"
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/old", None).await;
        assert_eq!(status, 302);
        assert_eq!(body.len(), 0);

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_redirect_explicit_status() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route GET /old {
                        @redirect 301 "/new-home"
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, _) = send_request(addr, "GET", "/old", None).await;
        assert_eq!(status, 301);

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_db_create_find_roundtrip() {
    // C_db E2E: POST /users 로 row 생성, GET /users/:id 로 조회, GET /users
    // 로 전체 목록 조회. 요청 간 db 가 공유되는지 검증.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route POST /users {
                        let u = await @db.create("User", @body)
                        @respond 201 u
                    }
                    @route GET /users/:id {
                        let raw: string = @param.id
                        let found = await @db.find("User", { name: raw })
                        @respond 200 found
                    }
                    @route GET /users {
                        let all = await @db.findAll("User", {})
                        @respond 200 all
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 1) 생성.
        let (s1, _, b1) = send_request(
            addr,
            "POST",
            "/users",
            Some(r#"{"name":"alice","age":30}"#.into()),
        )
        .await;
        assert_eq!(s1, 201);
        let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
        assert_eq!(j1["id"], serde_json::json!(1));
        assert_eq!(j1["name"], serde_json::json!("alice"));

        // 2) name 으로 조회 (MVP: int.from 미구현이라 string filter 사용).
        let (s2, _, b2) = send_request(addr, "GET", "/users/alice", None).await;
        assert_eq!(s2, 200);
        let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
        assert_eq!(j2["name"], serde_json::json!("alice"));

        // 3) 또 하나 생성 후 전체 조회.
        let (_, _, _) = send_request(
            addr,
            "POST",
            "/users",
            Some(r#"{"name":"bob","age":25}"#.into()),
        )
        .await;
        let (s3, _, b3) = send_request(addr, "GET", "/users", None).await;
        assert_eq!(s3, 200);
        let j3: serde_json::Value = serde_json::from_slice(&b3).expect("json");
        assert_eq!(j3.as_array().map(Vec::len), Some(2));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_level_middleware_applies_to_all_routes() {
    // SPEC §11.7: `@server { @AccessLog; @route ... }` — server block
    // 최상단의 middleware 는 이후 모든 route 에 prepend.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    define Inject() -> @before { @next {v: "top"} }
                    @Inject
                    @route GET /a { @respond 200 { v: @context.v, kind: "a" } }
                    @route GET /b { @respond 200 { v: @context.v, kind: "b" } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for path in ["/a", "/b"] {
            let (status, _, body) = send_request(addr, "GET", path, None).await;
            assert_eq!(status, 200, "path {path}");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
            assert_eq!(json["v"], serde_json::json!("top"), "path {path}");
        }

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_level_middleware_only_applies_to_routes_declared_after() {
    // 선언 순서 규칙: `@Cors` 이전 route 는 middleware 미적용, 이후는 적용.
    // group-flatten 과 동일 의미론.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    define First() -> @before { @next {hdr: "first"} }
                    @route GET /before { @respond 200 { hdr: @context.hdr, tag: "pre" } }
                    @First
                    @route GET /after { @respond 200 { hdr: @context.hdr, tag: "post" } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // /before: middleware 선언 전 → context.hdr 없음 → @context.hdr 접근 에러
        // 로 500 이 나야 한다 (handler 에 no field hdr).
        let (s_before, _, _) = send_request(addr, "GET", "/before", None).await;
        assert_eq!(s_before, 500, "/before must not have middleware applied");

        // /after: middleware 선언 뒤 → context.hdr == "first"
        let (s_after, _, b) = send_request(addr, "GET", "/after", None).await;
        assert_eq!(s_after, 200);
        let json: serde_json::Value = serde_json::from_slice(&b).expect("json");
        assert_eq!(json["hdr"], serde_json::json!("first"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn group_and_leaf_middleware_compose_in_declared_order() {
    // 그룹 middleware → leaf route 내부 middleware 순서로 쌓여야 한다.
    // 그룹이 `role: "user"` 를 넣고, leaf 가 `role: "admin"` 으로 덮어쓴다.
    // 마지막 push 우세 규칙이 선언 순서와 일치해야 한다.
    run_on_localset(async {
            let ServerTestCase {
                listen,
                routes,
                body_stmts,
                captured_env,
            } = extract_server_case(
                r#"@server {
                    @listen 0
                    define Base() -> @before { @next {role: "user", gid: 1} }
                    define Elevate() -> @before { @next {role: "admin"} }
                    @route /api {
                        @Base
                        @route GET /public { @respond 200 { role: @context.role, gid: @context.gid } }
                        @route GET /secret {
                            @Elevate
                            @respond 200 { role: @context.role, gid: @context.gid }
                        }
                    }
                }"#,
            );
            let (addr, handle, _boot) = spawn_for_test(
                listen.as_deref(),
                &routes,
                &body_stmts,
                captured_env,
                std::future::pending::<()>(),
            )
            .await
            .expect("spawn");

            let (s1, _, b1) = send_request(addr, "GET", "/api/public", None).await;
            assert_eq!(s1, 200);
            let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
            assert_eq!(j1["role"], serde_json::json!("user"));
            assert_eq!(j1["gid"], serde_json::json!(1));

            let (s2, _, b2) = send_request(addr, "GET", "/api/secret", None).await;
            assert_eq!(s2, 200);
            let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
            // leaf 내부 @Elevate 가 role 덮어씀, gid 는 Base 값 유지.
            assert_eq!(j2["role"], serde_json::json!("admin"));
            assert_eq!(j2["gid"], serde_json::json!(1));

            handle.abort();
        })
        .await;
}

#[tokio::test]
async fn group_middleware_before_can_short_circuit_all_inner_routes() {
    // 그룹 middleware 의 `@respond` 로 인증 실패 단락. `/admin/*` 내 모든
    // route 가 handler 본문 실행 없이 401 을 돌려줘야 한다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    define Deny() -> @before { @respond 401 { err: "unauth" } }
                    @route /admin {
                        @Deny
                        @route GET /users { @respond 200 { hit: "users" } }
                        @route DELETE /users/:id { @respond 200 { hit: "deleted" } }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        for (method, path) in [("GET", "/admin/users"), ("DELETE", "/admin/users/42")] {
            let (status, _, body) = send_request(addr, method, path, None).await;
            assert_eq!(status, 401, "{method} {path}");
            let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
            assert_eq!(json["err"], serde_json::json!("unauth"), "{method} {path}");
        }

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_level_let_is_visible_to_handlers() {
    // A3: `@server { let x = ...; @route ... }` 에서 선언된 바인딩이
    // 라우트 핸들러 스코프 안에서 읽힌다. @out 같은 부트 문장과 나란히
    // 섞여 있어도 동작해야 한다.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @out "boot"
                    let version = "1.0.0"
                    let greeting = "hello"
                    @route GET /v { @respond 200 { v: version, g: greeting } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/v", None).await;
        assert_eq!(status, 200);
        let j: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(j["v"], serde_json::json!("1.0.0"));
        assert_eq!(j["g"], serde_json::json!("hello"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn nested_group_let_is_visible_to_handlers() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    @route /admin {
                        let version = "1.0.0"
                        @route GET /v { @respond 200 { v: version } }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/admin/v", None).await;
        assert_eq!(status, 200);
        let j: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(j["v"], serde_json::json!("1.0.0"));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn listen_can_use_top_level_binding() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"let port = 0

                @server {
                    @listen port
                    @route GET /ping { @respond 200 { ok: true } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let j: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(j["ok"], serde_json::json!(true));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn listen_can_use_server_level_binding() {
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    let port = 0
                    @listen port
                    @route GET /ping { @respond 200 { ok: true } }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, body) = send_request(addr, "GET", "/ping", None).await;
        assert_eq!(status, 200);
        let j: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(j["ok"], serde_json::json!(true));

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn server_level_let_reassignment_is_per_request() {
    // A3 하이브리드: 핸들러가 server-level `let` 을 재할당해도 per-request
    // clone 이라 다른 요청에 안 샌다. 두 번 호출 시 둘 다 counter == 1.
    run_on_localset(async {
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(
            r#"@server {
                    @listen 0
                    let mut counter = 0
                    @route GET /inc {
                        counter = counter + 1
                        @respond 200 { counter: counter }
                    }
                }"#,
        );
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 두 번 연속 호출 — 공유 상태면 1, 2 가 나오고, per-request clone
        // 이면 둘 다 1 이 나온다. 후자가 A3 가 약속한 동작.
        let (s1, _, b1) = send_request(addr, "GET", "/inc", None).await;
        assert_eq!(s1, 200);
        let j1: serde_json::Value = serde_json::from_slice(&b1).expect("json");
        assert_eq!(j1["counter"], serde_json::json!(1));

        let (s2, _, b2) = send_request(addr, "GET", "/inc", None).await;
        assert_eq!(s2, 200);
        let j2: serde_json::Value = serde_json::from_slice(&b2).expect("json");
        assert_eq!(
            j2["counter"],
            serde_json::json!(1),
            "second request saw leaked mutation from first"
        );

        handle.abort();
    })
    .await;
}

#[tokio::test]
async fn serve_directory_resolves_rest_param() {
    // A5b: `@serve "./dir"` + `@route GET /prefix/:rest* { ... }` 조합.
    // 디렉토리 대상이면 `@param.rest` 와 join 해 파일을 찾는다.
    run_on_localset(async {
        let tmp = std::env::temp_dir().join(format!("orv_serve_dir_{}", std::process::id()));
        let sub = tmp.join("sub");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::write(tmp.join("index.html"), b"<h1>root</h1>").expect("w1");
        std::fs::write(sub.join("deep.txt"), b"deep file").expect("w2");

        let src = format!(
            r#"@server {{
                    @listen 0
                    @route GET /assets/:rest* {{ @serve "{}" }}
                }}"#,
            tmp.display()
        );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // 1) 루트 파일
        let (s1, ct1, b1) = send_request(addr, "GET", "/assets/index.html", None).await;
        assert_eq!(s1, 200);
        assert_eq!(ct1.as_deref(), Some("text/html; charset=utf-8"));
        assert_eq!(b1, b"<h1>root</h1>");

        // 2) 하위 디렉토리 파일
        let (s2, _, b2) = send_request(addr, "GET", "/assets/sub/deep.txt", None).await;
        assert_eq!(s2, 200);
        assert_eq!(b2, b"deep file");

        // 3) 없는 파일 → 404
        let (s3, _, _) = send_request(addr, "GET", "/assets/missing.txt", None).await;
        assert_eq!(s3, 404);

        handle.abort();
        std::fs::remove_dir_all(&tmp).ok();
    })
    .await;
}

#[tokio::test]
async fn serve_directory_rejects_traversal_attempts() {
    // A5b 보안: `..` 세그먼트가 포함된 rest 는 403. canonicalize 후 root
    // prefix 검사가 통과하더라도 문법적 signal 로 먼저 차단.
    run_on_localset(async {
        let tmp = std::env::temp_dir().join(format!("orv_serve_traverse_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("mkdir");
        std::fs::write(tmp.join("ok.txt"), b"ok").expect("w");
        // 바깥 파일
        let outside = tmp
            .parent()
            .unwrap()
            .join(format!("orv_serve_outside_{}.txt", std::process::id()));
        std::fs::write(&outside, b"secret").expect("w outside");

        let src = format!(
            r#"@server {{
                    @listen 0
                    @route GET /a/:rest* {{ @serve "{}" }}
                }}"#,
            tmp.display()
        );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        // `..` 포함 경로 — 실제로 바깥 파일을 탈출하려는 시도.
        let (status, _, _) = send_request(
            addr,
            "GET",
            &format!("/a/../orv_serve_outside_{}.txt", std::process::id()),
            None,
        )
        .await;
        // hyper 가 `/a/..` 를 정규화할 수 있으므로 403 또는 404 / 200
        // 중에 secret 은 절대 안 나와야 한다. 핵심: 200 이면 body 에
        // "secret" 이 나오지 않아야 한다.
        if status == 200 {
            panic!("traversal should not succeed");
        }

        handle.abort();
        std::fs::remove_dir_all(&tmp).ok();
        std::fs::remove_file(&outside).ok();
    })
    .await;
}

#[tokio::test]
async fn serve_missing_file_returns_404() {
    run_on_localset(async {
        let missing = std::env::temp_dir().join("orv_serve_nonexistent_xyz.html");
        let _ = std::fs::remove_file(&missing);
        let src = format!(
            r#"@server {{
                    @listen 0
                    @route GET /missing {{ @serve "{}" }}
                }}"#,
            missing.display()
        );
        let ServerTestCase {
            listen,
            routes,
            body_stmts,
            captured_env,
        } = extract_server_case(&src);
        let (addr, handle, _boot) = spawn_for_test(
            listen.as_deref(),
            &routes,
            &body_stmts,
            captured_env,
            std::future::pending::<()>(),
        )
        .await
        .expect("spawn");

        let (status, _, _) = send_request(addr, "GET", "/missing", None).await;
        assert_eq!(status, 404);

        handle.abort();
    })
    .await;
}
