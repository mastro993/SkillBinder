//! Repository verification and registry documentation.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod evidence;
mod graph;
mod manifests;
mod mcp_smoke;
mod package;
mod registry;

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace directory is missing"))?;
    match std::env::args().nth(1).as_deref() {
        Some("verify") => {
            registry::verify(root)?;
            evidence::verify(root)?;
            graph::verify(root)?;
            println!(
                "Verified registry, licensed artwork, preserved baseline integrity, and crate boundaries."
            );
            println!("This command does not establish native behavior or visual parity.");
            Ok(())
        }
        Some("registry") => registry::print(root),
        Some("package") => package::package(root, std::env::args().nth(2).as_deref()),
        Some("mcp-smoke") => mcp_smoke::run(std::env::args_os().skip(2)),
        _ => anyhow::bail!(
            "Usage: cargo run -p xtask -- <verify|registry|package [target]|mcp-smoke <app-binary> <gpui-mcp-binary>>"
        ),
    }
}
