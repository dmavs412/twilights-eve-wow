//! The test host with water, `common::Pond`, that the fishing tests stand beside.

mod common;

use common::*;

#[test]
fn a_pond_has_water_inside_its_circle_and_none_outside() {
    let pond = Pond {
        centre: [10.0, 20.0],
        radius: 5.0,
        area: None,
    };
    assert_eq!(pond.liquid_at(0, 12.0, 21.0), Some(0.5));
    assert_eq!(pond.liquid_at(0, 16.0, 20.0), None);
    assert_eq!(pond.ground_height(0, 12.0, 21.0, 5.0), Some(0.0));
}
