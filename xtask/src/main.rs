//! Repository verification and registry documentation.

mod evidence;
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
            println!("Verified registry, licensed artwork, and preserved baseline integrity.");
            println!("This command does not establish native behavior or visual parity.");
            Ok(())
        }
        Some("registry") => registry::print(root),
        _ => anyhow::bail!("Usage: cargo run -p xtask -- <verify|registry>"),
    }
}
