use super::run_str;

#[test]
fn lambda_error_restores_caller_bindings() {
    let output = run_str(
        r#"let fail = () -> { throw "expected failure" }
let value: int = 42
try { fail() } catch error { @out value }
@out value"#,
    )
    .expect("continue in caller scope after catch");
    assert_eq!(output, "42\n42\n");
}

#[test]
fn native_lambda_error_restores_caller_bindings() {
    let output = run_str(
        r#"let parse = () -> int.from("invalid")
let value: int = 42
try { parse() } catch error { @out value }
@out value"#,
    )
    .expect("continue in caller scope after native error");
    assert_eq!(output, "42\n42\n");
}

#[test]
fn recursive_error_restores_the_catching_call_frame() {
    let output = run_str(
        r#"function visit(n: int): int -> {
  if n == 0 { throw "expected failure" }
  try { visit(n - 1) } catch error { @out n }
  return n
}
@out visit(1)"#,
    )
    .expect("continue in outer recursive frame after catch");
    assert_eq!(output, "1\n1\n");
}

#[test]
fn caught_function_error_restores_html_rendering() {
    let output = run_str(
        r#"function fail(): int -> { throw "expected failure" }
let page = @html {
  @div {
    try { fail() } catch error { @span "recovered" }
    @span "continued"
  }
}
@out page"#,
    )
    .expect("continue rendering after catch");
    assert!(output.contains("<span>recovered</span>"), "{output}");
    assert!(output.contains("<span>continued</span>"), "{output}");
}

#[test]
fn sqlite_transaction_rollback_survives_reconnect() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "orv-sqlite-recovery-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).expect("create test directory");
    let path = directory.join("shop.sqlite");
    let source = format!(
        r#"let db = @db.connect "sqlite://{}"
let original = db.create("Order", {{ status: "original" }})
try {{
  db.transaction({{
    db.update("Order", {{ id: original.id }}, {{ status: "changed" }})
    db.create("Order", {{ status: "temporary" }})
    db.delete("Order", {{ id: original.id }})
    throw "cancel"
  }})
}} catch error {{ @out "rolled back" }}
@out db.count("Order")
@out db.find("Order", {{ id: original.id }}).status
let reopened = @db.connect "sqlite://{}"
@out reopened.count("Order")
@out reopened.find("Order", {{ id: original.id }}).status
let next = reopened.create("Order", {{ status: "next" }})
@out next.id"#,
        path.display(),
        path.display(),
    );
    let output = run_str(&source);
    std::fs::remove_dir_all(directory).expect("remove test directory");
    assert_eq!(
        output.expect("rollback and reconnect"),
        "rolled back\n1\noriginal\n1\noriginal\n2\n"
    );
}
