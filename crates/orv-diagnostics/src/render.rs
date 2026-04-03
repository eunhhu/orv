use codespan_reporting::diagnostic as crd;
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{Buffer, ColorChoice, StandardStream};
use orv_span::SourceMap;

use crate::{Diagnostic, Severity};

/// Converts our diagnostics into `codespan-reporting` diagnostics and renders
/// them to stderr with colors.
pub fn render_diagnostics(source_map: &SourceMap, diagnostics: &[Diagnostic]) {
    let (files, diags) = build_codespan_data(source_map, diagnostics);
    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();
    for diag in &diags {
        let _ = term::emit(&mut writer.lock(), &config, &files, diag);
    }
}

/// Converts our diagnostics into `codespan-reporting` diagnostics and renders
/// them to a plain-text string (no ANSI colors). Useful for testing.
pub fn render_diagnostics_to_string(source_map: &SourceMap, diagnostics: &[Diagnostic]) -> String {
    let (files, diags) = build_codespan_data(source_map, diagnostics);
    let mut buffer = Buffer::no_color();
    let config = term::Config::default();
    for diag in &diags {
        let _ = term::emit(&mut buffer, &config, &files, diag);
    }
    String::from_utf8_lossy(buffer.as_slice()).into_owned()
}

fn build_codespan_data(
    source_map: &SourceMap,
    diagnostics: &[Diagnostic],
) -> (SimpleFiles<String, String>, Vec<crd::Diagnostic<usize>>) {
    let mut files = SimpleFiles::new();

    // Add all source files. The codespan file id is the insertion order index.
    for i in 0..source_map.file_count() {
        let file_id = orv_span::FileId::new(u32::try_from(i).expect("too many files"));
        files.add(
            source_map.name(file_id).to_owned(),
            source_map.source(file_id).to_owned(),
        );
    }

    let diags = diagnostics.iter().map(convert_diagnostic).collect();
    (files, diags)
}

fn convert_diagnostic(diag: &Diagnostic) -> crd::Diagnostic<usize> {
    let severity = match diag.severity {
        Severity::Error => crd::Severity::Error,
        Severity::Warning => crd::Severity::Warning,
        Severity::Help => crd::Severity::Help,
    };

    let labels = diag
        .labels
        .iter()
        .map(|l| {
            let file_id = l.span.file().raw() as usize;
            let range = l.span.start() as usize..l.span.end() as usize;
            if l.is_primary {
                crd::Label::primary(file_id, range).with_message(&l.message)
            } else {
                crd::Label::secondary(file_id, range).with_message(&l.message)
            }
        })
        .collect();

    crd::Diagnostic::new(severity)
        .with_message(&diag.message)
        .with_labels(labels)
        .with_notes(diag.notes.clone())
}
