//! Instance maps (ADR 0032, amended 2026-11-04): maps behind a loading screen that hold Phased
//! Dungeons, such as the Stockade's map 34 (`content/instance_maps.toml`). Each has an entrance
//! trigger on the open world that worldports a character in and an exit trigger inside that
//! sends them back out to the door, in the Wailing Caverns' [`Trigger`] form. The map outside
//! every dungeon's footprint is open world, and the map's Dungeon mark stands at its door.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::dungeon::Trigger;

/// One instance map.
#[derive(Deserialize)]
pub(crate) struct InstanceMap {
    /// Its `Map.dbc` id.
    pub(crate) map: u32,
    /// What its Dungeon mark is called.
    pub(crate) name: String,
    /// The `AreaTable.dbc` area a character on it is listed in.
    pub(crate) zone: u32,
    /// The levels its Dungeon mark carries.
    pub(crate) levels: [u8; 2],
    /// The trigger on the open world that worldports a character in.
    pub(crate) entrance: Trigger,
    /// The trigger on this map that sends them back out to the door.
    pub(crate) exit: Trigger,
}

#[derive(Deserialize)]
struct File {
    instance_map: Vec<InstanceMap>,
}

/// Every instance map.
pub(crate) fn content() -> &'static [InstanceMap] {
    static MAPS: OnceLock<Vec<InstanceMap>> = OnceLock::new();
    MAPS.get_or_init(|| {
        let file: File = toml::from_str(include_str!("../content/instance_maps.toml"))
            .expect("content/instance_maps.toml parses");
        for m in &file.instance_map {
            assert!(
                !crate::content::open_world(m.map),
                "{}: map {} is an open-world continent",
                m.name,
                m.map
            );
            assert!(
                m.entrance.to.map == m.map && m.exit.map == m.map,
                "{}: its entrance leads to map {} and its exit stands on map {}",
                m.name,
                m.entrance.to.map,
                m.exit.map
            );
            assert!(
                crate::content::open_world(m.entrance.map) && m.exit.to.map == m.entrance.map,
                "{}: its entrance stands on map {} and its exit leads to map {}",
                m.name,
                m.entrance.map,
                m.exit.to.map
            );
        }
        file.instance_map
    })
}

/// The entrance or exit trigger of this `AreaTrigger.dbc` id, of the instance map that has it.
pub(crate) fn with_trigger(id: u32) -> Option<&'static Trigger> {
    content()
        .iter()
        .flat_map(|m| [&m.entrance, &m.exit])
        .find(|t| t.trigger == id)
}

/// The instance map of this `Map.dbc` id, if it is one.
pub(crate) fn of_map(map: u32) -> Option<&'static InstanceMap> {
    content().iter().find(|m| m.map == map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stockade_is_map_34_with_triggers_101_and_503() {
        let m = of_map(34).expect("the Stockade");
        assert_eq!(m.name, "The Stockade");
        assert_eq!((m.zone, m.levels), (717, [30, 40]));
        assert_eq!(with_trigger(101).map(|t| t.to.map), Some(34));
        assert_eq!(with_trigger(503).map(|t| t.to.map), Some(0));
        assert!(of_map(0).is_none() && with_trigger(102).is_none());
    }

    #[test]
    fn the_deeprun_tram_is_map_369_with_triggers_2173_and_2171() {
        let m = of_map(369).expect("the Deeprun Tram");
        assert_eq!(m.name, "The Deeprun Tram");
        assert_eq!((m.zone, m.levels), (2257, [38, 42]));
        assert_eq!(with_trigger(2173).map(|t| t.to.map), Some(369));
        assert_eq!(with_trigger(2171).map(|t| t.to.map), Some(0));
    }
}
