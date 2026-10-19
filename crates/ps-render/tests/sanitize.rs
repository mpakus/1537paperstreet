use ps_render::{RenderOptions, render, render_with_options};

#[test]
fn rejects_the_xss_corpus_by_default() {
    let markdown = r#"
<script>alert('script')</script>

<img src="x" onerror="alert('image')">

<svg onload="alert('svg')"><circle /></svg>

<iframe srcdoc="<script>alert('frame')</script>"></iframe>

<a href="javascript:alert('raw-link')">raw JavaScript link</a>

<a href="data:text/html,<script>alert('data')</script>">raw data link</a>

<a href="JaVaScRiPt:alert('mixed-case')">mixed-case link</a>

<a href="java&#x0a;script:alert('entity-control')">entity control link</a>

[Markdown JavaScript link](javascript:alert('markdown-link'))

[Markdown data link](data:text/html,<script>alert('markdown-data')</script>)

<form action="javascript:alert('form')"><button>Submit</button></form>

<object data="data:text/html,unsafe"></object>

<embed src="javascript:alert('embed')">

<details open ontoggle="alert('details')">details</details>

<meta http-equiv="refresh" content="0;url=javascript:alert('meta')">

<style>body { background: url('javascript:alert(1)') }</style>
"#;

    let html = render(markdown);

    for rejected in [
        "<script",
        "onerror",
        "<svg",
        "onload",
        "<iframe",
        "srcdoc",
        "javascript:",
        "data:text/html",
        "<form",
        "action=",
        "<object",
        "<embed",
        "ontoggle",
        "<meta",
        "<style",
    ] {
        assert!(
            !html.to_ascii_lowercase().contains(rejected),
            "XSS vector {rejected:?} survived in {html}"
        );
    }

    assert!(html.contains("raw JavaScript link"));
    assert!(html.contains("Markdown JavaScript link"));
}

#[test]
fn rejects_css_advisory_payloads_and_preserves_safe_marker_colors() {
    for style in [
        r#"background:url("cid:\");position:fixed;background:url(\"cid:")"#,
        r#"background-image:URL("https://example.invalid/tracking-pixel")"#,
        r#"background-image:UrL("javascript:alert(1)")"#,
    ] {
        let html = render(&format!("<mark style='{style}'>visible text</mark>"));
        assert!(html.contains("<mark>visible text</mark>"), "{html}");
        assert!(!html.contains("style="), "{html}");
    }

    let html = render("=={#123456}safe highlight==");
    assert!(html.contains("background-color:#123456"), "{html}");
    assert!(html.contains("safe highlight</mark>"), "{html}");
}

#[test]
fn rejects_all_svg_animation_advisory_payloads() {
    for tag in ["animateMotion", "animateTransform", "animateColor"] {
        let html = render(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><a><{tag} attributeName=\"href\" \
             to=\"javascript:alert(1)\" from=\"javascript:alert(2)\" \
             values=\"javascript:alert(3)\"></{tag}><text>Click</text></a></svg>"
        ));
        let normalized = html.to_ascii_lowercase();
        assert!(!normalized.contains("<svg"), "{html}");
        assert!(!normalized.contains("<animate"), "{html}");
        assert!(!normalized.contains("javascript:"), "{html}");
    }
}

#[test]
fn raw_html_can_be_enabled_explicitly() {
    let html = render_with_options(
        "<mark data-reader=\"yes\">raw</mark>",
        RenderOptions {
            allow_raw_html: true,
        },
    );

    assert!(html.starts_with("<section class=\"chunk\">"));
    assert!(html.contains("<mark data-reader=\"yes\">raw</mark>"));
    assert!(html.ends_with("</section>\n"));
}

#[test]
fn dangerous_markdown_links_are_rejected_even_when_raw_html_is_enabled() {
    let html = render_with_options(
        "[JavaScript](JaVaScRiPt:alert(1)) [data](data:text/html,unsafe) [entity](java&#x73;cript:alert(1)) [control](java&#x0a;script:alert(1))",
        RenderOptions {
            allow_raw_html: true,
        },
    );

    assert_eq!(html.matches("href=\"#invalid-url\"").count(), 4, "{html}");
    assert!(!html.to_ascii_lowercase().contains("javascript:"));
    assert!(!html.to_ascii_lowercase().contains("data:text/html"));
}
