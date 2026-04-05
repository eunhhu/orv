//! Integration tests for the name resolver using larger orv programs.

use orv_diagnostics::DiagnosticBag;
use orv_span::FileId;
use orv_syntax::lexer::Lexer;
use orv_syntax::parser::parse;
use pretty_assertions::assert_eq;

use crate::SymbolKind;
use crate::resolver::{ResolveResult, resolve};

fn resolve_source(src: &str) -> (ResolveResult, DiagnosticBag) {
    let file = FileId::new(0);
    let lexer = Lexer::new(src, file);
    let (tokens, lex_diags) = lexer.tokenize();
    assert!(!lex_diags.has_errors(), "lexer errors: {lex_diags:?}");
    let (module, parse_diags) = parse(tokens);
    assert!(!parse_diags.has_errors(), "parse errors: {parse_diags:?}");
    resolve(&module)
}

#[test]
fn full_program_counter() {
    let src = "\
let count = 0

function increment() -> {
    count = count + 1
}

function get_count() -> count

pub define Counter() -> @html {
    @button \"Increment\" {
        increment()
    }
    @text count
}
";
    let (result, diags) = resolve_source(src);
    assert!(!diags.has_errors(), "unexpected errors: {diags:?}");

    let names: Vec<&str> = result
        .symbols
        .iter()
        .map(|(_, s)| s.name.as_str())
        .collect();
    // count, increment, get_count, Counter
    assert!(names.contains(&"count"));
    assert!(names.contains(&"increment"));
    assert!(names.contains(&"get_count"));
    assert!(names.contains(&"Counter"));
}

#[test]
fn program_with_imports_and_define() {
    let src = "\
import ui.{Button, Text}
import utils.format as fmt

pub define App() -> @html {
    Button()
    Text()
    fmt(\"hello\")
}
";
    let (_, diags) = resolve_source(src);
    assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
}

#[test]
fn program_for_loop_with_shadowing() {
    let src = "\
let items = [1, 2, 3]
let total = 0

function sum() -> {
    for item of items {
        total = total + item
    }
    total
}
";
    let (_, diags) = resolve_source(src);
    assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
}

#[test]
fn program_nested_if_else() {
    let src = "\
let x = 10

function classify() -> {
    if x > 0 {
        let label = \"positive\"
        label
    } else {
        if x == 0 {
            let label = \"zero\"
            label
        } else {
            let label = \"negative\"
            label
        }
    }
}
";
    let (_, diags) = resolve_source(src);
    assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
}

#[test]
fn program_multiple_errors() {
    let src = "\
function foo() -> {
    let a = unknown1
    unknown2 + a
}
";
    let (_, diags) = resolve_source(src);
    assert!(diags.has_errors());
    // Should have 2 unresolved errors: unknown1, unknown2
    let error_count = diags.iter().filter(|d| d.is_error()).count();
    assert_eq!(error_count, 2);
}

#[test]
fn program_struct_and_enum_declared() {
    let src = "\
struct Point {
    x: i32
    y: i32
}
enum Direction {
    North
    South
    East
    West
}
type Pos = Point

function origin() -> Point
";
    let (result, diags) = resolve_source(src);
    assert!(!diags.has_errors());

    let kinds: Vec<_> = result.symbols.iter().map(|(_, s)| s.kind).collect();
    assert!(kinds.contains(&SymbolKind::Struct));
    assert!(kinds.contains(&SymbolKind::Enum));
    assert!(kinds.contains(&SymbolKind::TypeAlias));
    assert!(kinds.contains(&SymbolKind::Function));
}

#[test]
fn program_define_with_node_children() {
    let src = "\
import components.Icon

pub define NavItem(label: string, href: string) -> @html {
    @a href {
        Icon()
        @span label
    }
}
";
    let (_, diags) = resolve_source(src);
    assert!(!diags.has_errors(), "unexpected errors: {diags:?}");
}
