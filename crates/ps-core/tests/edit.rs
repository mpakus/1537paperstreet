use std::path::Path;

use ps_core::Error;
use ps_core::edit::{format_text, lint_config};

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
fn formats_yaml_without_losing_comments_or_aliases() {
    let path = Path::new("config.YML");
    let formatted =
        format_text(path, "# defaults\nbase: &base {a: 1,b: 2}\ncopy: *base\n").expect("yaml");
    assert_eq!(
        formatted,
        "# defaults\nbase: &base { a: 1, b: 2 }\ncopy: *base\n"
    );
    assert!(format_text(path, "broken: [1,\n").is_err());
}

#[test]
fn formats_toml_without_losing_comments_or_table_order() {
    let path = Path::new("config.TOML");
    let formatted = format_text(
        path,
        "# config\n[package]\nname=\"app\" # keep\nports=[1,2]",
    )
    .expect("toml");
    assert_eq!(
        formatted,
        "# config\n[package]\nname = \"app\" # keep\nports = [1, 2]"
    );
    assert!(format_text(path, "key = 1\nkey = 2").is_err());
}

#[test]
fn config_lint_points_to_invalid_lines_and_accepts_valid_documents() {
    let yaml = Path::new("workflow.yaml");
    let toml = Path::new("Cargo.toml");
    assert!(lint_config(yaml, "---\na: 1\n---\nb: !custom value\n").is_empty());
    assert_eq!(lint_config(yaml, "a: 1\nb: [2}\n")[0].line, 2);
    assert!(lint_config(toml, "[package]\nname = 'app'\n").is_empty());
    assert_eq!(lint_config(toml, "key = 1\nkey = 2\n")[0].line, 2);
}

#[test]
fn toml_format_preserves_comments_in_multiline_values() {
    let input = "[settings]\nports=[\n  21, # ssh\n  80,\n]\ninline={level=1}\n";
    let expected = "[settings]\nports = [\n  21, # ssh\n  80,\n]\ninline = {level = 1}\n";
    assert_eq!(
        format_text(Path::new("settings.toml"), input).expect("toml"),
        expected
    );
}

#[test]
fn format_rejects_source_files() {
    let error = format_text(Path::new("main.rs"), "fn main() {}").expect_err("source");
    assert!(matches!(error, Error::FormatUnavailable));
}
