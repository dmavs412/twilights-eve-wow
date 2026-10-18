//! `twilight-server [--listen addr] [--data-dir dir]`: hosts one realm for friends to join.

use std::net::TcpListener;
use std::path::PathBuf;

fn main() {
    let mut listen = twilight_server::DEFAULT_LISTEN.to_string();
    let mut data_dir = PathBuf::from("server-data");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.next()) {
            ("--listen", Some(v)) => listen = v,
            ("--data-dir", Some(v)) => data_dir = v.into(),
            _ => {
                eprintln!("usage: twilight-server [--listen addr] [--data-dir dir]");
                std::process::exit(2);
            }
        }
    }
    let listener = match TcpListener::bind(&listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("twilight-server: cannot listen on {listen}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "twilight-server: listening on {listen}, saves in {}",
        data_dir.display()
    );
    twilight_server::serve(listener, data_dir);
}
