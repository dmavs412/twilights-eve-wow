//! `content/elwynn.toml` is what `examples/import_elwynn.rs` makes of the research dump.

#[path = "../examples/import_elwynn.rs"]
#[allow(dead_code)]
mod import_elwynn;

#[test]
fn elwynn_toml_is_the_import_of_the_dump() {
    let want = import_elwynn::convert(&import_elwynn::dump());
    let have = std::fs::read_to_string(import_elwynn::output_path())
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        have == want,
        "content/elwynn.toml is stale: run `cargo run -p twilight-realm --example import_elwynn`"
    );
}
