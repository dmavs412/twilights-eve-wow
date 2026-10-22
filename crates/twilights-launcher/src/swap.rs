use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::manifest::{hex, FileEntry, LAUNCHER_EXE};
use crate::{github, Error, Result};

pub const OLD_LAUNCHER_EXE: &str = "twilights-launcher.old.exe";

fn download_path(target: &Path) -> PathBuf {
    let mut name = OsString::from(target.file_name().unwrap_or_default());
    name.push(".download");
    target.with_file_name(name)
}

fn rename_error(e: std::io::Error) -> Error {
    // 32 is ERROR_SHARING_VIOLATION; Windows reports a running exe as access denied or 32.
    if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) {
        Error::InUse
    } else {
        Error::Io(e)
    }
}

/// Downloads `url` to `<target>.download`, checks it against `entry`, then renames it over
/// `target`. `progress` gets bytes done and the Content-Length total.
pub fn download_and_swap(
    url: &str,
    target: &Path,
    entry: &FileEntry,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<()> {
    let (total, reader) = github::open_download(url)?;
    swap_from_reader(reader, total, target, entry, progress)
}

/// The network-free half of `download_and_swap`.
pub fn swap_from_reader(
    mut reader: impl Read,
    total: Option<u64>,
    target: &Path,
    entry: &FileEntry,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<()> {
    let part = download_path(target);
    if let Err(e) = write_checked(&mut reader, total, &part, entry, progress) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&part, target) {
        let _ = std::fs::remove_file(&part);
        return Err(rename_error(e));
    }
    Ok(())
}

fn write_checked(
    reader: &mut impl Read,
    total: Option<u64>,
    part: &Path,
    entry: &FileEntry,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<()> {
    if let Some(dir) = part.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::File::create(part)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        progress(done, total);
    }
    file.flush()?;
    drop(file);
    if done != entry.size {
        return Err(Error::Mismatch(format!(
            "size {done}, expected {}",
            entry.size
        )));
    }
    let sha = hex(&hasher.finalize());
    if sha != entry.sha256 {
        return Err(Error::Mismatch(format!(
            "sha256 {sha}, expected {}",
            entry.sha256
        )));
    }
    Ok(())
}

/// Writes the installed release's manifest after the game exe is in place.
pub fn write_game_manifest(game_dir: &Path, manifest_json: &str) -> Result<()> {
    std::fs::create_dir_all(game_dir)?;
    std::fs::write(game_dir.join("manifest.json"), manifest_json)?;
    Ok(())
}

/// Deletes `twilights-launcher.old.exe` beside the Launcher if one exists. A failure is ignored.
pub fn clean_old_launcher(install_dir: &Path) {
    let _ = std::fs::remove_file(install_dir.join(OLD_LAUNCHER_EXE));
}

/// Renames the running exe to `twilights-launcher.old.exe`, renames the verified download to
/// `twilights-launcher.exe` and starts it with `args`. The caller exits after it returns.
pub fn self_swap(install_dir: &Path, verified_download: &Path, args: &[OsString]) -> Result<()> {
    let exe = install_dir.join(LAUNCHER_EXE);
    let old = install_dir.join(OLD_LAUNCHER_EXE);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(rename_error)?;
    if let Err(e) = std::fs::rename(verified_download, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(rename_error(e));
    }
    Command::new(&exe)
        .args(args)
        .current_dir(install_dir)
        .spawn()?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Cursor;

    /// A fresh empty folder under the system temp dir, named for the test.
    pub fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("twilights-launcher-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry_for(bytes: &[u8]) -> FileEntry {
        FileEntry {
            sha256: hex(&Sha256::digest(bytes)),
            size: bytes.len() as u64,
        }
    }

    #[test]
    fn matching_download_replaces_target() {
        let dir = scratch_dir("swap-ok");
        let target = dir.join("twilights-eve.exe");
        std::fs::write(&target, b"old").unwrap();
        let new = b"new game bytes";
        let mut last = (0, None);
        swap_from_reader(
            Cursor::new(new),
            Some(new.len() as u64),
            &target,
            &entry_for(new),
            |d, t| last = (d, t),
        )
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), new);
        assert!(!dir.join("twilights-eve.exe.download").exists());
        assert_eq!(last, (new.len() as u64, Some(new.len() as u64)));
    }

    #[test]
    fn wrong_hash_keeps_old_target_and_removes_download() {
        let dir = scratch_dir("swap-bad");
        let target = dir.join("twilights-eve.exe");
        std::fs::write(&target, b"old").unwrap();
        let mut entry = entry_for(b"new game bytes");
        entry.sha256 = "0".repeat(64);
        let err = swap_from_reader(
            Cursor::new(b"new game bytes"),
            None,
            &target,
            &entry,
            |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(err, Error::Mismatch(_)));
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert!(!dir.join("twilights-eve.exe.download").exists());
    }

    #[test]
    fn wrong_size_is_a_mismatch() {
        let dir = scratch_dir("swap-size");
        let target = dir.join("a.exe");
        let mut entry = entry_for(b"abc");
        entry.size = 99;
        let err =
            swap_from_reader(Cursor::new(b"abc"), None, &target, &entry, |_, _| {}).unwrap_err();
        assert!(matches!(err, Error::Mismatch(_)));
        assert!(!target.exists());
    }

    #[test]
    fn in_use_message() {
        assert_eq!(
            Error::InUse.to_string(),
            "Close Twilight's Eve to finish patching"
        );
    }

    #[test]
    fn game_manifest_is_written() {
        let dir = scratch_dir("swap-manifest");
        write_game_manifest(&dir.join("game"), "{}").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("game").join("manifest.json")).unwrap(),
            "{}"
        );
    }
}
