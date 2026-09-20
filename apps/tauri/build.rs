fn main() {
    println!("cargo:rerun-if-changed=icons/icon.png");
    let manifest = tauri_build::AppManifest::new().commands(&[
        "system_bootstrap",
        "git_environment_verify",
        "onboarding_progress_update",
        "onboarding_complete_local",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to build SkillBinder Tauri application");
}
