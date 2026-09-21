use std::path::Path;

use ps_core::Error;
use ps_core::edit::format_text;

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
fn formats_json() {
    let formatted = format_text(Path::new("config.json"), "{\"a\":1}").expect("json");
    assert_eq!(formatted, "{\n  \"a\": 1\n}");
}

#[test]
fn format_rejects_source_files() {
    let error = format_text(Path::new("main.rs"), "fn main() {}").expect_err("source");
    assert!(matches!(error, Error::FormatUnavailable));
}
