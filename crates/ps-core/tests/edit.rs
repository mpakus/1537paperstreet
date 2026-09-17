use std::path::Path;

use ps_core::Error;
use ps_core::edit::{DiagnosticSeverity, format_text, lint_text};

#[test]
fn formats_a_ragged_markdown_table() {
    let source = "| Name | Status |\n| :--- | ---: |\n| Alpha | Ready |\n| Beta | Waiting |\n";
    let formatted = format_text(Path::new("notes.md"), source).expect("format");
    assert_eq!(
        formatted,
        "| Name  |  Status |\n| :---- | ------: |\n| Alpha |   Ready |\n| Beta  | Waiting |\n"
    );
    assert_eq!(
        format_text(Path::new("notes.md"), &formatted).expect("idempotent"),
        formatted
    );
}

#[test]
fn leaves_tables_inside_fenced_code_alone() {
    let source = "```\n| A | B |\n| --- | --- |\n| 1 | 2 |\n```\n";
    assert_eq!(
        format_text(Path::new("notes.md"), source).expect("format"),
        source
    );
}

#[test]
fn lints_mismatched_table_columns_and_an_unclosed_fence() {
    let source = "| A | B |\n| --- | --- |\n| only |\n\n```\nnot closed\n";
    let issues = lint_text(Path::new("guide.md"), source);
    assert!(
        issues.iter().any(|issue| {
            issue.severity == DiagnosticSeverity::Error
                && issue.message.contains("header has 2")
                && issue.line == 3
        }),
        "{issues:?}"
    );
    assert!(
        issues.iter().any(|issue| {
            issue.severity == DiagnosticSeverity::Error
                && issue.message.contains("not closed")
                && issue.line == 5
        }),
        "{issues:?}"
    );
}

#[test]
fn formats_and_lints_json() {
    let formatted = format_text(Path::new("config.json"), "{\"a\":1}").expect("json");
    assert_eq!(formatted, "{\n  \"a\": 1\n}");
    assert!(lint_text(Path::new("config.json"), &formatted).is_empty());
    let broken = lint_text(Path::new("config.json"), "{");
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].severity, DiagnosticSeverity::Error);
    assert_eq!(broken[0].message, "This file is not valid JSON.");
}

#[test]
fn format_rejects_source_files() {
    let error = format_text(Path::new("main.rs"), "fn main() {}").expect_err("source");
    assert!(matches!(error, Error::FormatUnavailable));
    assert!(lint_text(Path::new("main.rs"), "fn main() {").is_empty());
}
