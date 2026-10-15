//! The enUS name and rank of each spell id, read from `Spell.dbc` in the patch chain: the name is
//! column 120 and the rank (`SpellNameSubtext`) column 129. A content author's check of an id
//! before it goes into `crates/twilight-realm/content/`.
//!
//! `cargo run -p benilla-formats --example spell_names -- 12723 12880`
fn main() -> anyhow::Result<()> {
    let data = benilla_formats::wow_data().expect("no WoW install found (set $WOW_DATA)");
    let mut chain = benilla_formats::open_chain(&data)?;
    let catalog = benilla_formats::load_spell_catalog(&mut chain)?;
    for arg in std::env::args().skip(1) {
        let id: u32 = arg.parse()?;
        match catalog.get(id) {
            Some(d) => println!("{id}\t{}\t{}", d.name, d.rank.as_deref().unwrap_or("")),
            None => println!("{id}\tNOT IN Spell.dbc"),
        }
    }
    Ok(())
}
