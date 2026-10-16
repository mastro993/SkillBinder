//! Rebuilds the engine when a migration is added, so `embed_migrations!` embeds it.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
