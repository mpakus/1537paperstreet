use ps_core::edit::HighlightRange;
use ps_render::{apply_highlight, highlight_source, render};

fn range(source: &str, word: &str, from: usize, to: usize) -> HighlightRange {
    let start = source.find(word).unwrap();
    HighlightRange {
        start,
        end: start + word.len(),
        from,
        to,
    }
}

#[test]
fn renders_default_presets_hex_and_nested_formatting() {
    let html = render("==plain== ==🟢green== =={#fc0}custom== ==one **two** &amp; three==");
    assert!(html.contains("data-highlight=\"default\""));
    assert!(html.contains("data-highlight=\"green\""));
    assert!(html.contains("background-color:#ffcc00"));
    assert!(html.contains("<strong><mark"));
    assert!(!html.contains("=="));
    assert!(html.contains("&amp;"));
}

#[test]
fn leaves_code_math_escapes_and_bad_colors_literal() {
    let source = "`==code==`\n\n```\n==fenced==\n```\n\n$==math==$\n\n\\==escaped==\n\n=={#red;bad}literal==";
    let html = render(source);
    assert!(!html.contains("<mark"), "{html}");
}

#[test]
fn mapping_is_only_built_on_demand_and_keeps_source_hashes() {
    let source = "A **bold** &amp; 🐈.\n";
    let normal = render(source);
    let mapped = highlight_source(source);
    assert!(!normal.contains("data-ps-map"));
    assert!(
        mapped
            .html
            .contains(&format!("data-ps-map=\"{}:", mapped.hash))
    );
    assert!(mapped.html.contains("data-hash="));
}

#[test]
fn edits_only_the_selected_occurrence_and_preserves_unicode() {
    let source = "same same 🐈 café";
    let selected = range(source, source, 5, 9);
    assert_eq!(
        apply_highlight(source, &[selected], Some("#AbC")).unwrap(),
        "same =={#aabbcc}same== 🐈 café"
    );
    let selected = range(source, source, 10, 12);
    assert_eq!(
        apply_highlight(source, &[selected], Some("blue")).unwrap(),
        "same same ==🔵🐈== café"
    );
}

#[test]
fn preserves_markdown_when_selection_crosses_emphasis() {
    let source = "one **two** three";
    let ranges = [range(source, "one ", 0, 4), range(source, "two", 0, 3)];
    assert_eq!(
        apply_highlight(source, &ranges, Some("default")).unwrap(),
        "==one ==**==two==** three"
    );
}

#[test]
fn recolors_or_removes_an_existing_marker_without_nesting() {
    let source = "A ==🟢green== word";
    let selected = range(source, "green", 1, 3);
    assert_eq!(
        apply_highlight(source, std::slice::from_ref(&selected), Some("purple")).unwrap(),
        "A ==🟣green== word"
    );
    assert_eq!(
        apply_highlight(source, &[selected], None).unwrap(),
        "A green word"
    );
}

#[test]
fn rejects_invalid_ranges_and_colors_without_producing_a_replacement() {
    let source = "hello 🐈";
    assert!(apply_highlight(source, &[range(source, source, 7, 8)], Some("red")).is_err());
    assert!(
        apply_highlight(
            source,
            &[range(source, source, 0, 3)],
            Some("red; background:url(x)")
        )
        .is_err()
    );
    let bad = HighlightRange {
        start: 1,
        end: 5,
        from: 0,
        to: 2,
    };
    assert!(apply_highlight(source, &[bad], Some("red")).is_err());
}

#[test]
fn custom_colors_survive_sanitization_without_allowing_arbitrary_styles() {
    let html = render(
        "=={#123456}safe== <span style=\"position:fixed\">raw</span> <mark style=\"background:url(https://bad)\">bad</mark>",
    );
    assert!(
        html.contains("background-color:#123456;color:#ffffff"),
        "{html}"
    );
    assert!(!html.contains("position:"));
    assert!(!html.contains("url("));
}

#[test]
fn preserves_entities_smart_punctuation_links_and_paragraph_boundaries() {
    let html = render("==a &amp; b -- c \\* d==\n\n[==label==](notes.md)\n\n==first\n\nlast==");
    assert!(html.contains("&amp;"));
    assert!(html.contains("data-highlight=\"default\""));
    assert!(html.contains("href=\"notes.md\""));
    assert!(html.contains("==first"));
    assert!(html.contains("last=="));
}

#[test]
fn cannot_modify_link_destinations_or_code_through_forged_ranges() {
    for source in [
        "<https://example.com>",
        "[[topic]]",
        "`code`",
        "![image](x.png)",
    ] {
        let mapped = highlight_source(source);
        assert!(
            !mapped.html.contains("data-ps-map"),
            "{source}: {}",
            mapped.html
        );
    }
    let source = "[label](destination.md)";
    assert_eq!(
        apply_highlight(source, &[range(source, "label", 0, 5)], Some("red")).unwrap(),
        "[==🔴label==](destination.md)"
    );
}

#[test]
fn escaped_punctuation_round_trips_through_a_marker() {
    let source = "a \\* b";
    let mapped = highlight_source(source);
    assert!(mapped.html.contains("data-ps-map"));
    assert_eq!(
        apply_highlight(source, &[range(source, "* b", 0, 1)], Some("default")).unwrap(),
        "a ==\\*== b"
    );
    let source = "&amp;";
    assert_eq!(
        apply_highlight(source, &[range(source, source, 0, 1)], Some("yellow")).unwrap(),
        "==🟡&amp;=="
    );
}
