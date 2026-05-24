use std::collections::{BTreeSet, HashMap};

use orv_runtime::server::{request_trace_json, ServerRequestFrame};

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

#[test]
fn request_trace_json_contract_freezes_public_object_keys_and_types() {
    let trace = request_trace_json(&[ServerRequestFrame {
        method: "POST".to_string(),
        path: "/checkout".to_string(),
        route_method: Some("POST".to_string()),
        route_path: Some("/checkout".to_string()),
        route_origin_id: Some("ori_route_checkout".to_string()),
        response_origin_id: Some("ori_response_checkout".to_string()),
        status: 201,
        params: HashMap::from([("order".to_string(), "42".to_string())]),
        query: HashMap::from([("coupon".to_string(), "SAVE".to_string())]),
        body: r#"{"sku":"tea"}"#.to_string(),
    }]);

    assert_keys(
        &trace,
        &["schema_version", "kind", "frame_count", "frames"],
        "request trace",
    );
    assert_eq!(trace["schema_version"], serde_json::json!(1));
    assert_eq!(trace["kind"], serde_json::json!("orv.production.trace"));
    assert_eq!(trace["frame_count"], serde_json::json!(1));

    let frame = &trace["frames"].as_array().expect("frames array")[0];
    assert_keys(
        frame,
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
        "request trace frame",
    );
    assert_eq!(frame["method"], serde_json::json!("POST"));
    assert_eq!(frame["path"], serde_json::json!("/checkout"));
    assert_eq!(frame["status"], serde_json::json!(201));
    assert_eq!(frame["route_method"], serde_json::json!("POST"));
    assert_eq!(frame["route_path"], serde_json::json!("/checkout"));
    assert_eq!(
        frame["route_origin_id"],
        serde_json::json!("ori_route_checkout")
    );
    assert_eq!(
        frame["response_origin_id"],
        serde_json::json!("ori_response_checkout")
    );
    assert!(frame["params"].is_object());
    assert!(frame["query"].is_object());
    assert_eq!(frame["body"], serde_json::json!(r#"{"sku":"tea"}"#));
}

#[test]
fn request_trace_json_contract_serializes_unknown_route_metadata_as_null() {
    let trace = request_trace_json(&[ServerRequestFrame {
        method: "GET".to_string(),
        path: "/missing".to_string(),
        route_method: None,
        route_path: None,
        route_origin_id: None,
        response_origin_id: None,
        status: 404,
        params: HashMap::new(),
        query: HashMap::new(),
        body: String::new(),
    }]);

    let frame = &trace["frames"].as_array().expect("frames array")[0];

    assert_eq!(trace["frame_count"], serde_json::json!(1));
    assert_eq!(frame["route_method"], serde_json::Value::Null);
    assert_eq!(frame["route_path"], serde_json::Value::Null);
    assert_eq!(frame["route_origin_id"], serde_json::Value::Null);
    assert_eq!(frame["response_origin_id"], serde_json::Value::Null);
    assert_eq!(frame["params"], serde_json::json!({}));
    assert_eq!(frame["query"], serde_json::json!({}));
}
