use std::path::Path;
use std::process::Command;

use crate::settings::{Mode, Settings};

/// The command for `game/twilights-eve.exe`, run in `game/` with `WOW_DATA` set. Solo passes no
/// arguments; join passes the server and password.
pub fn command(install_dir: &Path, data: &Path, settings: &Settings) -> Command {
    let game_dir = install_dir.join("game");
    let mut cmd = Command::new(game_dir.join(crate::manifest::GAME_EXE));
    cmd.current_dir(&game_dir).env("WOW_DATA", data);
    if settings.mode == Mode::Join {
        cmd.arg("--server")
            .arg(&settings.server)
            .arg("--join-password")
            .arg(&settings.join_password);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn solo_has_no_arguments() {
        let cmd = command(Path::new("L"), Path::new("D"), &Settings::default());
        assert_eq!(cmd.get_args().count(), 0);
        assert_eq!(
            cmd.get_current_dir(),
            Some(Path::new("L").join("game").as_path())
        );
        let env: Vec<_> = cmd.get_envs().collect();
        assert_eq!(env, vec![(OsStr::new("WOW_DATA"), Some(OsStr::new("D")))]);
    }

    #[test]
    fn join_passes_server_and_password() {
        let s = Settings {
            mode: Mode::Join,
            server: "h:1".into(),
            join_password: "pw".into(),
            ..Settings::default()
        };
        let cmd = command(Path::new("L"), Path::new("D"), &s);
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["--server", "h:1", "--join-password", "pw"]);
    }
}
