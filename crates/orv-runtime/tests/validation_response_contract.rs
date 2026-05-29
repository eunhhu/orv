use std::collections::{BTreeSet, HashMap};

use orv_diagnostics::FileId;
use orv_hir::{HirBlock, HirExpr, HirExprKind, HirStmt, Type};
use orv_runtime::{run_handler_with_request, HandlerOutcome, RequestCtx, RuntimeError, Value};
use orv_syntax::{lex, parse_with_newlines};

const VALIDATION_ERROR_RESPONSE_GOLDEN: &str =
    include_str!("../../../docs/samples/validation-error-response-v1.golden.json");
const DIAGNOSTIC_MESSAGE_PLACEHOLDER: &str = "<diagnostic message>";

fn assert_keys(value: &serde_json::Value, expected: &[&str], context: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be an object"));
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{context} keys drifted");
}

fn lower_handler(src: &str) -> HirExpr {
    let lexed = lex(src, FileId(0));
    assert!(
        lexed.diagnostics.is_empty(),
        "lex errors: {:?}",
        lexed.diagnostics
    );
    let parsed = parse_with_newlines(lexed.tokens, FileId(0), lexed.newlines);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse errors: {:?}",
        parsed.diagnostics
    );
    let resolved = orv_resolve::resolve(&parsed.program);
    assert!(
        resolved.diagnostics.is_empty(),
        "resolve errors: {:?}",
        resolved.diagnostics
    );
    let hir = orv_analyzer::lower(&parsed.program, &resolved);
    if hir.items.len() == 1 {
        let HirStmt::Expr(expr) = &hir.items[0] else {
            panic!("expected handler expression");
        };
        return expr.clone();
    }
    HirExpr {
        kind: HirExprKind::Block(HirBlock {
            stmts: hir.items,
            span: hir.span,
        }),
        ty: Type::Unknown,
        span: hir.span,
    }
}

fn run_handler_json(
    src: &str,
    request: RequestCtx,
) -> Result<(HandlerOutcome, String), RuntimeError> {
    let handler = lower_handler(src);
    let mut output = Vec::new();
    let outcome = run_handler_with_request(&handler, request, &mut output)?;
    Ok((outcome, String::from_utf8(output).expect("utf8 output")))
}

fn value_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Void => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Int(value) => serde_json::Value::Number((*value).into()),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Value::Str(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) | Value::Tuple(values) => {
            serde_json::Value::Array(values.iter().map(value_json).collect())
        }
        Value::Object(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), value_json(value)))
                .collect(),
        ),
        other => serde_json::Value::String(other.to_string()),
    }
}

fn normalize_validation_messages(mut value: serde_json::Value) -> serde_json::Value {
    let fields = value
        .get_mut("fields")
        .and_then(serde_json::Value::as_array_mut)
        .expect("validation fields");
    for field in fields {
        let object = field.as_object_mut().expect("validation field object");
        let message = object
            .get("message")
            .and_then(serde_json::Value::as_str)
            .expect("validation field message");
        assert!(!message.is_empty(), "validation message must not be empty");
        object.insert(
            "message".to_string(),
            serde_json::json!(DIAGNOSTIC_MESSAGE_PLACEHOLDER),
        );
    }
    value
}

#[test]
fn validation_error_response_contract_matches_published_golden_fixture() {
    let request = RequestCtx {
        body: Value::Object(vec![
            ("email".to_string(), Value::Str("buyer@orv.dev".to_string())),
            ("coupon".to_string(), Value::Str("SAVE10".to_string())),
        ]),
        ..Default::default()
    };

    let (outcome, output) = run_handler_json(
        r#"struct CheckoutForm {
  email: string(trim, lower, min=3)
  quantity: int(min=1)
}
@body: CheckoutForm
@out "unreachable""#,
        request,
    )
    .expect("handler run");

    assert_eq!(output, "");
    let response = outcome.response.expect("validation response");
    assert_eq!(response.status, 400);
    let actual = normalize_validation_messages(value_json(&response.payload));
    let expected: serde_json::Value =
        serde_json::from_str(VALIDATION_ERROR_RESPONSE_GOLDEN).expect("golden json");
    assert_eq!(actual, expected, "validation response golden drift");
}

#[test]
fn validation_error_response_contract_freezes_public_object_keys_and_types() {
    let request = RequestCtx {
        body: Value::Object(vec![
            ("email".to_string(), Value::Str("buyer@orv.dev".to_string())),
            ("quantity".to_string(), Value::Str("0".to_string())),
        ]),
        ..Default::default()
    };

    let (outcome, output) = run_handler_json(
        r#"struct CheckoutForm {
  email: string(trim, lower, min=3)
  quantity: int(min=1)
}
@body: CheckoutForm
@out "unreachable""#,
        request,
    )
    .expect("handler run");

    assert_eq!(output, "");
    let response = outcome.response.expect("validation response");
    assert_eq!(response.status, 400);
    let body = value_json(&response.payload);
    assert_keys(
        &body,
        &["schema_version", "kind", "error", "fields"],
        "validation response",
    );
    assert_eq!(body["schema_version"], serde_json::json!(1));
    assert_eq!(body["kind"], serde_json::json!("orv.validation.error"));
    assert_eq!(body["error"], serde_json::json!("validation_failed"));

    let fields = body["fields"].as_array().expect("validation fields");
    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    assert_keys(
        field,
        &["path", "code", "message", "expected", "actual"],
        "validation field",
    );
    assert_eq!(field["path"], serde_json::json!("$.quantity"));
    assert_eq!(field["code"], serde_json::json!("type_mismatch"));
    assert!(field["message"].as_str().is_some_and(|message| {
        message.contains("constraint mismatch") && message.contains("min=1")
    }));
    assert_eq!(field["expected"], serde_json::json!("int(min=1)"));
    assert_eq!(field["actual"], serde_json::json!("0"));
}

#[test]
fn validation_error_response_contract_preserves_multi_error_order_and_null_actuals() {
    let request = RequestCtx {
        body: Value::Object(vec![
            ("email".to_string(), Value::Str("buyer@orv.dev".to_string())),
            ("coupon".to_string(), Value::Str("SAVE10".to_string())),
        ]),
        ..Default::default()
    };

    let (outcome, output) = run_handler_json(
        r#"struct CheckoutForm {
  email: string(trim, lower, min=3)
  quantity: int(min=1)
}
@body: CheckoutForm
@out "unreachable""#,
        request,
    )
    .expect("handler run");

    assert_eq!(output, "");
    let response = outcome.response.expect("validation response");
    assert_eq!(response.status, 400);
    let body = value_json(&response.payload);
    let fields = body["fields"].as_array().expect("validation fields");
    assert_eq!(fields.len(), 2);
    for field in fields {
        assert_keys(
            field,
            &["path", "code", "message", "expected", "actual"],
            "validation field",
        );
    }

    assert_eq!(fields[0]["path"], serde_json::json!("$.quantity"));
    assert_eq!(fields[0]["code"], serde_json::json!("missing_required"));
    assert_eq!(fields[0]["expected"], serde_json::json!("int(min=1)"));
    assert_eq!(fields[0]["actual"], serde_json::Value::Null);

    assert_eq!(fields[1]["path"], serde_json::json!("$.coupon"));
    assert_eq!(fields[1]["code"], serde_json::json!("unknown_property"));
    assert_eq!(fields[1]["expected"], serde_json::json!("CheckoutForm"));
    assert_eq!(fields[1]["actual"], serde_json::json!("SAVE10"));
}

#[test]
fn validation_error_response_contract_distinguishes_constraint_mismatch() {
    let request = RequestCtx {
        body: Value::Object(vec![
            ("email".to_string(), Value::Str("buyer@orv.dev".to_string())),
            ("quantity".to_string(), Value::Int(0)),
        ]),
        ..Default::default()
    };

    let (outcome, output) = run_handler_json(
        r#"struct CheckoutForm {
  email: string(trim, lower, min=3)
  quantity: int(min=1)
}
@body: CheckoutForm
@out "unreachable""#,
        request,
    )
    .expect("handler run");

    assert_eq!(output, "");
    let response = outcome.response.expect("validation response");
    assert_eq!(response.status, 400);
    let body = value_json(&response.payload);
    let fields = body["fields"].as_array().expect("validation fields");
    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    assert_keys(
        field,
        &["path", "code", "message", "expected", "actual"],
        "validation field",
    );
    assert_eq!(field["path"], serde_json::json!("$.quantity"));
    assert_eq!(field["code"], serde_json::json!("constraint_mismatch"));
    assert!(field["message"]
        .as_str()
        .is_some_and(|message| message.contains("min=1")));
    assert_eq!(field["expected"], serde_json::json!("int(min=1)"));
    assert_eq!(field["actual"], serde_json::json!(0));
}

#[test]
fn validation_error_response_contract_covers_query_and_form_binding_producers() {
    let cases = [
        (
            r#"struct SearchQuery {
  page: int(min=1)
  q: string(trim, lower, min=1)
}
@query: SearchQuery
@out "unreachable""#,
            RequestCtx {
                query: HashMap::from([
                    ("page".to_string(), "0".to_string()),
                    ("q".to_string(), "tea".to_string()),
                ]),
                ..Default::default()
            },
            "$.page",
            "0",
        ),
        (
            r#"struct SignupForm {
  email: string(trim, lower, min=3)
  age: int(min=13)
}
@form: SignupForm
@out "unreachable""#,
            RequestCtx {
                body: Value::Object(vec![
                    ("email".to_string(), Value::Str("buyer@orv.dev".to_string())),
                    ("age".to_string(), Value::Str("12".to_string())),
                ]),
                ..Default::default()
            },
            "$.age",
            "12",
        ),
    ];

    for (src, request, expected_path, expected_actual) in cases {
        let (outcome, output) = run_handler_json(src, request).expect("handler run");

        assert_eq!(output, "");
        let response = outcome.response.expect("validation response");
        assert_eq!(response.status, 400);
        let body = value_json(&response.payload);
        assert_keys(
            &body,
            &["schema_version", "kind", "error", "fields"],
            "validation response",
        );
        assert_eq!(body["schema_version"], serde_json::json!(1));
        assert_eq!(body["kind"], serde_json::json!("orv.validation.error"));
        assert_eq!(body["error"], serde_json::json!("validation_failed"));

        let fields = body["fields"].as_array().expect("validation fields");
        assert_eq!(fields.len(), 1);
        let field = &fields[0];
        assert_keys(
            field,
            &["path", "code", "message", "expected", "actual"],
            "validation field",
        );
        assert_eq!(field["path"], serde_json::json!(expected_path));
        assert_eq!(field["code"], serde_json::json!("type_mismatch"));
        assert!(field["message"]
            .as_str()
            .is_some_and(|message| message.contains("constraint mismatch")));
        assert!(field["expected"].as_str().is_some());
        assert_eq!(field["actual"], serde_json::json!(expected_actual));
    }
}
