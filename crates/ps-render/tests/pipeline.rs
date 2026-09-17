use ps_render::render;

#[test]
fn renders_the_required_markdown_extensions() {
    let markdown = r#"# Title {#custom}

| Name | Done |
| --- | --- |
| Reader | ~~no~~ yes |

- [x] shipped

> [!NOTE]
> Reader build.

Footnote[^1] -- "quoted"

Inline math: $x + y$.

$$z = 1$$

[^1]: note
"#;

    let html = render(markdown);

    for expected in [
        "<h1 id=\"custom\">Title</h1>",
        "table-scroll",
        "class=\"cell\"",
        "<table>",
        "<del>no</del>",
        "type=\"checkbox\"",
        "class=\"footnote-reference\"",
        "markdown-alert",
        "– “quoted”",
        "class=\"math math-inline\"",
        "class=\"math math-display\"",
    ] {
        assert!(html.contains(expected), "missing {expected:?} in {html}");
    }
}

#[test]
fn minimized_fuzz_regression_for_malformed_task_list() {
    // The incomplete UTF-8 byte mirrors the byte-oriented fuzz target exactly.
    let bytes = [45, 32, 91, 120, 93, 58, 210, 111, 13, 12, 13, 13];
    let markdown = String::from_utf8_lossy(&bytes);

    let html = render(&markdown);

    assert!(!html.is_empty());
}

#[test]
fn minimized_fuzz_regression_for_lone_carriage_return() {
    let bytes = [
        13, 45, 32, 91, 97, 93, 58, 110, 10, 12, 12, 12, 12, 12, 12, 10,
    ];
    let markdown = String::from_utf8_lossy(&bytes);

    let html = render(&markdown);

    assert!(!html.is_empty());
}
