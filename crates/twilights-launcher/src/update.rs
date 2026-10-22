//! One update run: find the channel's release, read its manifest, then swap the Launcher and the
//! game exe as the plan says. Progress goes out as `Event`s; the window and `--check` both use it.

use std::io::Read;
use std::path::Path;

use crate::manifest::{sha256_file, Manifest, GAME_EXE, LAUNCHER_EXE};
use crate::plan::{plan, Installed, Step};
use crate::settings::Settings;
use crate::status::Event;
use crate::{github, swap, Error};

pub const NEW_LAUNCHER_EXE: &str = "twilights-launcher.new.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The window: the Launcher replaces itself and the caller exits.
    Window,
    /// `--check`: the Launcher reports a pending self-update and carries on with the game.
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    /// A final event was sent.
    Finished,
    /// The new Launcher is running; the caller must exit now.
    Relaunched,
}

pub fn game_installed(install_dir: &Path) -> bool {
    install_dir.join("game").join(GAME_EXE).exists()
}

fn final_event(e: Error) -> Event {
    match e {
        Error::InUse => Event::InUse,
        Error::Mismatch(_) => Event::Damaged,
        other => Event::Failed(other.to_string()),
    }
}

/// Runs the whole update. The last event sent is `Ready`, `CheckFailed`, `InUse`, `Damaged` or
/// `Failed`, unless the result is `Relaunched`.
pub fn run(
    install_dir: &Path,
    settings: &Settings,
    mode: Mode,
    mut emit: impl FnMut(Event),
) -> Done {
    emit(Event::Checking);
    let release = match github::fetch_release(settings.channel) {
        Ok(r) => r,
        Err(e) => {
            emit(Event::CheckFailed(e.to_string()));
            return Done::Finished;
        }
    };
    emit(Event::Release {
        tag: release.tag.clone(),
        notes: release.notes.clone(),
    });
    let (manifest_json, manifest) = match fetch_manifest(&release) {
        Ok(m) => m,
        Err(e) => {
            emit(Event::CheckFailed(e.to_string()));
            return Done::Finished;
        }
    };
    let game_dir = install_dir.join("game");
    let installed_manifest = std::fs::read_to_string(game_dir.join("manifest.json"))
        .ok()
        .and_then(|t| Manifest::parse(&t).ok());
    let launcher_sha256 = std::env::current_exe()
        .map_err(Error::from)
        .and_then(|p| sha256_file(&p))
        .unwrap_or_default();
    let steps = plan(
        Installed {
            game_exe_exists: game_installed(install_dir),
            manifest: installed_manifest.as_ref(),
            launcher_sha256: &launcher_sha256,
        },
        &manifest,
    );
    if steps.contains(&Step::UpdateLauncher) {
        emit(Event::LauncherUpdate(release.tag.clone()));
        if mode == Mode::Window {
            return match update_launcher(install_dir, &release, &manifest, &mut emit) {
                Ok(()) => Done::Relaunched,
                Err(e) => {
                    emit(final_event(e));
                    Done::Finished
                }
            };
        }
    }
    if steps.contains(&Step::PatchGame) {
        if let Err(e) = patch_game(install_dir, &release, &manifest, &manifest_json, &mut emit) {
            emit(final_event(e));
            return Done::Finished;
        }
    }
    emit(Event::Ready(release.tag));
    Done::Finished
}

fn fetch_manifest(release: &github::Release) -> crate::Result<(String, Manifest)> {
    let url = release
        .assets
        .get("manifest.json")
        .ok_or_else(|| Error::Parse("release has no manifest.json".into()))?;
    let (_, mut reader) = github::open_download(url)?;
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let manifest = Manifest::parse(&text)?;
    Ok((text, manifest))
}

fn asset_url<'a>(release: &'a github::Release, name: &str) -> crate::Result<&'a str> {
    release
        .assets
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| Error::Parse(format!("release has no {name}")))
}

fn update_launcher(
    install_dir: &Path,
    release: &github::Release,
    manifest: &Manifest,
    emit: &mut impl FnMut(Event),
) -> crate::Result<()> {
    let target = install_dir.join(NEW_LAUNCHER_EXE);
    swap::download_and_swap(
        asset_url(release, LAUNCHER_EXE)?,
        &target,
        manifest.entry(LAUNCHER_EXE)?,
        |done, total| {
            emit(Event::Downloading {
                what: "launcher".into(),
                done,
                total,
            })
        },
    )?;
    swap::self_swap(install_dir, &target, &[])
}

fn patch_game(
    install_dir: &Path,
    release: &github::Release,
    manifest: &Manifest,
    manifest_json: &str,
    emit: &mut impl FnMut(Event),
) -> crate::Result<()> {
    let game_dir = install_dir.join("game");
    let tag = release.tag.clone();
    swap::download_and_swap(
        asset_url(release, GAME_EXE)?,
        &game_dir.join(GAME_EXE),
        manifest.entry(GAME_EXE)?,
        |done, total| {
            emit(Event::Downloading {
                what: tag.clone(),
                done,
                total,
            })
        },
    )?;
    swap::write_game_manifest(&game_dir, manifest_json)
}
