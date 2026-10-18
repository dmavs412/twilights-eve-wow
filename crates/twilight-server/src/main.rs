//! `twilight-server --join-password pw [--listen addr] [--data-dir dir] [--data wow-data]`: hosts
//! one realm for friends to join. `--data` is the 1.12.1 `Data` folder (default `WOW_DATA`), for
//! the terrain; the join password is what a friend's client must present to get in.

use std::net::TcpListener;
use std::path::PathBuf;

const USAGE: &str =
    "usage: twilight-server --join-password pw [--listen addr] [--data-dir dir] [--data wow-data]";

fn main() {
    let mut listen = twilight_server::DEFAULT_LISTEN.to_string();
    let mut data_dir = PathBuf::from("server-data");
    let mut wow_data = std::env::var_os("WOW_DATA").map(PathBuf::from);
    let mut join_password = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--listen", Some(v)) => listen = v,
            ("--data-dir", Some(v)) => data_dir = v.into(),
            ("--data", Some(v)) => wow_data = Some(v.into()),
            ("--join-password", Some(v)) => join_password = Some(v),
            _ => {
                eprintln!("{USAGE}");
                std::process::exit(2);
            }
        }
    }
    let Some(join_password) = join_password.filter(|p| !p.is_empty()) else {
        eprintln!(
            "twilight-server: a join password is required
{USAGE}"
        );
        std::process::exit(2);
    };
    let Some(wow_data) = wow_data else {
        eprintln!("twilight-server: no game data; pass --data or set WOW_DATA");
        std::process::exit(2);
    };
    let terrain = match twilight_server::Terrain::open(&wow_data) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("twilight-server: cannot open {}: {e:#}", wow_data.display());
            std::process::exit(1);
        }
    };
    let listener = match TcpListener::bind(&listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("twilight-server: cannot listen on {listen}: {e}");
            std::process::exit(1);
        }
    };
    let build_id = twilight_realm::build_id::build_id(env!("BENILLA_GIT_SHA"));
    eprintln!(
        "twilight-server: build {build_id}, listening on {listen}, saves in {}",
        data_dir.display()
    );
    let config = twilight_server::Config {
        data_dir,
        join_password,
        build_id,
    };
    twilight_server::serve(listener, config, Box::new(terrain));
}
