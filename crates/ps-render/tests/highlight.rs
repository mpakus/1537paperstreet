use ps_render::{render, render_source};

#[test]
fn emits_prefixed_classes_without_inline_styles() {
    let html = render("```rust\nfn main() { println!(\"<hello>\"); }\n```\n");

    assert!(html.contains("<pre class=\"code\"><code class=\"language-rust\">"));
    assert!(html.contains("<span class=\"syntax-source syntax-rust\">"));
    assert!(html.contains("&lt;hello&gt;"));
    assert!(!html.contains("style="));
}

#[test]
fn leaves_unknown_fenced_languages_to_the_markdown_renderer() {
    let html = render("```unknown-language\n<plain>\n```\n");

    assert!(html.contains("<pre><code class=\"language-unknown-language\">"));
    assert!(html.contains("&lt;plain&gt;"));
    assert!(!html.contains("syntax-"));
}

#[test]
fn highlights_common_language_aliases() {
    for (fence, needle) in [
        ("```js\nconst n = 1;\n```\n", "syntax-source syntax-js"),
        (
            "```python\ndef hi():\n    return 1\n```\n",
            "syntax-source syntax-python",
        ),
        ("```rb\nputs 1\n```\n", "syntax-source syntax-ruby"),
        (
            "```elixir\ndefmodule M do\nend\n```\n",
            "syntax-source syntax-elixir",
        ),
        ("```yaml\nkey: value\n```\n", "syntax-source syntax-yaml"),
        ("```rust\nlet n = 1;\n```\n", "syntax-source syntax-rust"),
        (
            "```ts\nconst n: number = 1;\n```\n",
            "syntax-source syntax-ts",
        ),
        ("```cs\nclass App {}\n```\n", "syntax-source syntax-cs"),
        ("```csharp\nclass App {}\n```\n", "syntax-source syntax-cs"),
        ("```java\nclass App {}\n```\n", "syntax-source syntax-java"),
        ("```php\n<?php echo 1;\n```\n", "syntax-source syntax-php"),
        ("```go\npackage main\n```\n", "syntax-source syntax-go"),
    ] {
        let html = render(fence);
        assert!(
            html.contains("<pre class=\"code\">"),
            "missing highlighted pre for {fence:?} in {html}"
        );
        assert!(
            html.contains(needle),
            "missing {needle:?} for {fence:?} in {html}"
        );
    }
}

#[test]
fn source_files_are_highlighted_without_markdown() {
    let html = render_source("```\nnot a fence\n{\"ok\": true}\n", "json");
    assert!(html.contains("<section class=\"chunk\">"));
    assert!(html.contains("<pre class=\"code\">"));
    assert!(html.contains("syntax-"));
    assert!(!html.contains("<p>"));
    assert!(!html.contains("<h1"));

    for (language, source, needle) in [
        ("js", "const n = 1;\n", "syntax-source syntax-js"),
        (
            "html",
            "<div class=\"x\"></div>\n",
            "syntax-text syntax-html",
        ),
        ("css", "body { color: red; }\n", "syntax-source syntax-css"),
        ("go", "package main\n", "syntax-source syntax-go"),
        ("rust", "fn main() {}\n", "syntax-source syntax-rust"),
        ("ruby", "puts 1\n", "syntax-source syntax-ruby"),
        (
            "elixir",
            "defmodule M do\nend\n",
            "syntax-source syntax-elixir",
        ),
        (
            "py",
            "def hi():\n    return 1\n",
            "syntax-source syntax-python",
        ),
        ("toml", "name = \"app\"\n", "syntax-source syntax-toml"),
        ("ts", "const n: number = 1;\n", "syntax-source syntax-ts"),
        ("cs", "class App {}\n", "syntax-source syntax-cs"),
        ("java", "class App {}\n", "syntax-source syntax-java"),
        ("php", "<?php echo 1;\n", "syntax-source syntax-php"),
    ] {
        let html = render_source(source, language);
        assert!(
            html.contains(needle),
            "missing {needle:?} for {language} in {html}"
        );
    }

    let plain = render_source("hello <plain>\n", "not-a-real-language");
    assert!(plain.contains("<pre class=\"code\">"));
    assert!(plain.contains("&lt;plain&gt;"));
    assert!(!plain.contains("<p>"));
    assert!(!plain.contains("syntax-"));
}

#[test]
fn repeated_code_blocks_keep_identical_highlighted_output() {
    let html = render("```rust\nlet answer = 42;\n```\n\n```rust\nlet answer = 42;\n```\n");

    assert_eq!(html.matches("syntax-source syntax-rust").count(), 2);
    assert_eq!(
        html.matches("syntax-storage syntax-type syntax-rust")
            .count(),
        2
    );
}
