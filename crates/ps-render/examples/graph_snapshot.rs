//! Export a graph snapshot for local inspection and UI fixtures.
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: graph_snapshot PROJECT CACHE_DIRECTORY".into());
    }
    let graph = ps_render::project_graph(
        Path::new(&args[2]),
        Path::new(&args[1]),
        &Default::default(),
    )?;
    println!("{}", serde_json::to_string_pretty(&graph)?);
    Ok(())
}
