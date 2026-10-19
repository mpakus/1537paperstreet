//! Reproducible rendering latency for synthetic Markdown containing raw HTML.
use std::{hint::black_box, time::Instant};

fn main() {
    const SEED: &str = concat!(
        "# Heading\n\nA **Markdown** paragraph with [a link](notes.md).\n\n",
        "<mark style=\"background-color:#123456;color:#ffffff\">safe color</mark>\n\n",
        "<img src=\"image.png\" onerror=\"alert(1)\">\n\n",
        "<svg><a><animateMotion attributeName=\"href\" to=\"javascript:alert(1)\">",
        "</animateMotion><text>unsafe SVG</text></a></svg>\n\n",
    );
    for bytes in [10_usize * 1024, 100 * 1024] {
        let mut markdown = SEED.repeat(bytes.div_ceil(SEED.len()));
        markdown.truncate(bytes);
        for _ in 0..20 {
            black_box(ps_render::render(black_box(&markdown)));
        }
        let mut samples = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = Instant::now();
            black_box(ps_render::render(black_box(&markdown)));
            samples.push(start.elapsed().as_nanos());
        }
        samples.sort_unstable();
        println!(
            "{bytes} bytes: p50 {} ns, p95 {} ns",
            samples[100], samples[190]
        );
    }
}
