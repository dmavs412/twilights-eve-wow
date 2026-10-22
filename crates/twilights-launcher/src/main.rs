//! Stub: prints the plan for the current settings and exits. The window ticket replaces it.

use twilights_launcher::manifest::{sha256_file, Manifest};
use twilights_launcher::plan::{plan, Installed};
use twilights_launcher::settings::Settings;
use twilights_launcher::{github, swap, Result};

fn run() -> Result<()> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    swap::clean_old_launcher(&dir);
    let settings = Settings::load(&dir.join("launcher.toml"))?;
    let release = github::fetch_release(settings.channel)?;
    let url = release
        .assets
        .get("manifest.json")
        .ok_or_else(|| twilights_launcher::Error::Parse("release has no manifest.json".into()))?;
    let (_, mut reader) = github::open_download(url)?;
    let mut text = String::new();
    std::io::Read::read_to_string(&mut reader, &mut text)?;
    let manifest = Manifest::parse(&text)?;
    let installed_manifest = std::fs::read_to_string(dir.join("game").join("manifest.json"))
        .ok()
        .and_then(|t| Manifest::parse(&t).ok());
    let launcher_sha256 = sha256_file(&exe)?;
    let steps = plan(
        Installed {
            game_exe_exists: dir.join("game").join("twilights-eve.exe").exists(),
            manifest: installed_manifest.as_ref(),
            launcher_sha256: &launcher_sha256,
        },
        &manifest,
    );
    println!("release {}: {steps:?}", release.tag);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
