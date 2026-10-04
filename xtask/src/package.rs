use anyhow::{Context, bail};
use std::{fs, path::Path, process::Command};

pub(crate) fn package(root: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let triple = target.map(str::to_owned).unwrap_or_else(host_triple);
    let executable = if cfg!(windows) {
        "skillbinder.exe"
    } else {
        "skillbinder"
    };
    let binary = target.map_or_else(
        || root.join("target/release").join(executable),
        |target| {
            root.join("target")
                .join(target)
                .join("release")
                .join(executable)
        },
    );
    if !binary.is_file() {
        bail!("Build the locked release application for {triple} before packaging.");
    }
    let output = root.join("dist").join(&triple);
    fs::create_dir_all(&output)?;
    let name = format!("skillbinder-{triple}");
    if cfg!(target_os = "macos") {
        let app = output.join("SkillBinder.app");
        let contents = app.join("Contents");
        fs::create_dir_all(contents.join("MacOS"))?;
        fs::create_dir_all(contents.join("Resources"))?;
        fs::copy(&binary, contents.join("MacOS/skillbinder"))?;
        fs::copy(
            root.join("assets/icon.png"),
            contents.join("Resources/icon.png"),
        )?;
        fs::write(
            contents.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>SkillBinder</string>
<key>CFBundleDisplayName</key><string>SkillBinder</string>
<key>CFBundleExecutable</key><string>skillbinder</string>
<key>CFBundleIdentifier</key><string>com.skillbinder.desktop</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#,
        )?;
        licenses(root, &contents.join("Resources"))?;
        let image = output.join(format!("{name}.dmg"));
        let mut command = Command::new("hdiutil");
        command
            .args([
                "create",
                "-ov",
                "-format",
                "UDZO",
                "-volname",
                "SkillBinder",
                "-srcfolder",
            ])
            .arg(&app)
            .arg(&image);
        run(&mut command)?;
        println!("Packaged {} and {}", app.display(), image.display());
    } else {
        let directory = output.join("SkillBinder");
        fs::create_dir_all(&directory)?;
        fs::copy(binary, directory.join(executable))?;
        licenses(root, &directory)?;
        fs::copy(
            root.join("assets/icon.png"),
            directory.join("skillbinder.png"),
        )?;
        if cfg!(windows) {
            let archive = output.join(format!("{name}.zip"));
            run(Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "Compress-Archive -LiteralPath $env:SKILLBINDER_PACKAGE_DIRECTORY -DestinationPath $env:SKILLBINDER_PACKAGE_ARCHIVE -Force"]).env("SKILLBINDER_PACKAGE_DIRECTORY", &directory).env("SKILLBINDER_PACKAGE_ARCHIVE", &archive))?;
            println!("Packaged {}", archive.display());
        } else {
            fs::write(
                directory.join("skillbinder.desktop"),
                "[Desktop Entry]\nType=Application\nName=SkillBinder\nComment=Manage a local library of agent skills\nExec=skillbinder\nIcon=skillbinder\nTerminal=false\nCategories=Development;Utility;\n",
            )?;
            fs::write(
                directory.join("INSTALL.txt"),
                "Place skillbinder on PATH. Copy skillbinder.desktop to ~/.local/share/applications/ and skillbinder.png to ~/.local/share/icons/. The application needs a working X11 or Wayland desktop and a directory-selection portal.\n",
            )?;
            let archive = output.join(format!("{name}.tar.gz"));
            run(Command::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg("-C")
                .arg(&output)
                .arg("SkillBinder"))?;
            println!("Packaged {}", archive.display());
        }
    }
    Ok(())
}
fn licenses(root: &Path, destination: &Path) -> anyhow::Result<()> {
    for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"] {
        fs::copy(root.join(name), destination.join(name))?;
    }
    Ok(())
}
fn run(command: &mut Command) -> anyhow::Result<()> {
    let status = command
        .status()
        .context("Could not start the platform packaging tool")?;
    if !status.success() {
        bail!("Platform packaging failed with {status}.");
    }
    Ok(())
}
fn host_triple() -> String {
    let architecture = std::env::consts::ARCH;
    let platform = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(windows) {
        "pc-windows-msvc"
    } else {
        "unknown-linux-gnu"
    };
    format!("{architecture}-{platform}")
}
