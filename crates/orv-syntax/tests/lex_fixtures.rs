//! fixtures/plan/*.orv 파일들을 렉싱하는 integration 테스트.
//!
//! 실제 스펙 예제가 렉서를 통과하는 것이 첫 실전 기준이다.
//! 에러 진단 없이 토큰 스트림이 생성되면 통과.

use orv_diagnostics::FileId;
use orv_syntax::{lex, TokenKind};

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/plan")
        .join(name)
}

fn lex_fixture(name: &str) -> orv_syntax::LexResult {
    let path = fixture_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    lex(&source, FileId(0))
}

#[test]
fn lex_01_basics_without_errors() {
    let r = lex_fixture("01-basics.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
    assert!(r.tokens.len() > 10);
    assert_eq!(r.tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
}

#[test]
fn lex_02_types_without_errors() {
    let r = lex_fixture("02-types.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
    assert!(r.tokens.len() > 10);
}

#[test]
fn lex_03_domains_without_errors() {
    let r = lex_fixture("03-domains.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
    assert!(r.tokens.len() > 10);
}

#[test]
fn lex_04_web_without_errors() {
    let r = lex_fixture("04-web.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
    assert!(r.tokens.len() > 10);
}

#[test]
fn lex_05_server_without_errors() {
    let r = lex_fixture("05-server.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
}

#[test]
fn lex_06_optimization_without_errors() {
    let r = lex_fixture("06-optimization.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
}

#[test]
fn lex_07_fullstack_without_errors() {
    let r = lex_fixture("07-fullstack-showcase.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
}

#[test]
fn lex_08_superapp_without_errors() {
    let r = lex_fixture("08-superapp-simulation.orv");
    assert!(
        r.diagnostics.is_empty(),
        "diagnostics: {:?}",
        r.diagnostics
    );
}

#[test]
fn every_fixture_token_span_is_in_bounds() {
    let fixtures = [
        "01-basics.orv",
        "02-types.orv",
        "03-domains.orv",
        "04-web.orv",
        "05-server.orv",
        "06-optimization.orv",
        "07-fullstack-showcase.orv",
        "08-superapp-simulation.orv",
    ];
    for name in fixtures {
        let path = fixture_path(name);
        let source = std::fs::read_to_string(&path).unwrap();
        let source_len = source.len() as u32;
        let r = lex(&source, FileId(0));
        for tok in &r.tokens {
            assert!(
                tok.span.range.end <= source_len,
                "{name}: token span {} exceeds source length {source_len}",
                tok.span.range
            );
            assert!(
                tok.span.range.start <= tok.span.range.end,
                "{name}: inverted span {}",
                tok.span.range
            );
        }
    }
}
