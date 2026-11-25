//! Repository verification and registry documentation.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod catalog;
mod clippy_policy;
mod design_tokens;
mod dev;
mod evidence;
mod fixture;
mod graph;
mod manifests;
mod mcp_smoke;
mod off_token;
mod package;
mod registry;
mod update_manifest;
mod version;

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workspace directory is missing"))?;
    match std::env::args().nth(1).as_deref() {
        Some("dev") => dev::run(root, std::env::args_os().skip(2)),
        Some("verify") => {
            registry::verify(root)?;
            evidence::verify(root)?;
            graph::verify(root)?;
            design_tokens::verify(root)?;
            off_token::verify(root)?;
            println!(
                "Verified the registry, its skill-directory overlay, licensed artwork, preserved baseline integrity, crate boundaries, generated design tokens, and the off-token ratchet."
            );
            println!("This command does not establish native behavior or visual parity.");
            Ok(())
        }
        Some("registry") => registry::print(root),
        Some("clippy-policy") => clippy_policy::write(root),
        Some("design-tokens") => design_tokens::write(root),
        Some("off-token") => off_token::lower(root),
        Some("package") => package::package(root, std::env::args().nth(2).as_deref()),
        Some("version") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            version::run(root, &args)
        }
        Some("mcp-smoke") => mcp_smoke::run(std::env::args_os().skip(2)),
        Some("fixture") => fixture::build(root, std::env::args().nth(2).as_deref()),
        Some("catalog") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            match catalog::run(root, &args)? {
                catalog::Outcome::Fresh => Ok(()),
                catalog::Outcome::StaleCounts(error) => {
                    // The workflow deploys on 2, then fails the run.
                    println!("::error::Published the last counts: {error:#}");
                    std::process::exit(2)
                }
                catalog::Outcome::StaleUpdates(error) => {
                    println!("::error::Published the last update manifests: {error:#}");
                    std::process::exit(2)
                }
            }
        }
        Some("update-manifest") => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            update_manifest::run(&args)
        }
        _ => anyhow::bail!(
            "Usage: cargo run -p xtask -- <dev [<name>] [--fresh] [-- <app arguments>]|verify|registry|clippy-policy|design-tokens|off-token|package [target]|version [next <part>|set <version>]|mcp-smoke <app-binary> <gpui-mcp-binary>|fixture <directory>|catalog <--out <directory>|updates --out <directory>|check <directory>>|update-manifest --version <version> --dist <directory>>"
        ),
    }
}
