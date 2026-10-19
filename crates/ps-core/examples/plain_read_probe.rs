//! Reproducible ordinary Markdown open latency, using only a temporary fixture.
use std::{fs, path::Path, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    for bytes in [10 * 1024, 1024 * 1024] {
        fs::write(root.path().join("plain.md"), "x".repeat(bytes))?;
        for _ in 0..20 {
            ps_core::docio::read_doc(root.path(), Path::new("plain.md"))?;
        }
        let mut samples = Vec::new();
        for _ in 0..200 {
            let start = Instant::now();
            std::hint::black_box(ps_core::docio::read_doc(
                root.path(),
                Path::new("plain.md"),
            )?);
            samples.push(start.elapsed().as_nanos());
        }
        samples.sort_unstable();
        println!(
            "{bytes} bytes: p50 {} ns, p95 {} ns",
            samples[100], samples[190]
        );
    }
    Ok(())
}
