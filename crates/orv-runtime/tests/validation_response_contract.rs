use std::collections::BTreeSet;

use orv_diagnostics::FileId;
use orv_hir::{HirBlock, HirExpr, HirExprKind, HirStmt, Type};
use orv_runtime::{run_handler_with_request, HandlerOutcome, RequestCtx, RuntimeError, Value};
use orv_syntax::{lex, parse_with_newlines};

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
