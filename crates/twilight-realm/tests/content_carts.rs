//! `content/carts.toml`, the cart routes `tools/twilights-eve/road_path.py` traces along the road
//! texture (spec #560): stops, path and the cart's numbers.

use std::collections::HashSet;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct File {
    cart: Cart,
    route: Vec<Route>,
}

#[derive(serde::Deserialize)]
struct Cart {
    speed_yd_s: f32,
    dwell_s: f32,
    seats: u32,
    use_range_yd: f32,
}

#[derive(serde::Deserialize)]
struct Route {
    key: String,
    stops: Vec<Stop>,
    path: Vec<[f32; 3]>,
}

#[derive(serde::Deserialize)]
struct Stop {
    key: String,
    position: [f32; 3],
}

fn carts() -> File {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("content/carts.toml");
    let text = std::fs::read_to_string(&path).expect("content/carts.toml");
    toml::from_str::<File>(&text).expect("it parses")
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn every_route_has_two_or_more_stops_with_keys_unique_in_the_file() {
    let file = carts();
    assert!(!file.route.is_empty(), "carts.toml has a route");
    let mut keys = HashSet::new();
    for r in &file.route {
        assert!(
            r.stops.len() >= 2,
            "route {} has {} stops",
            r.key,
            r.stops.len()
        );
        for s in &r.stops {
            assert!(keys.insert(s.key.clone()), "stop key {} repeats", s.key);
        }
    }
}

#[test]
fn every_path_runs_along_its_stops() {
    for r in carts().route {
        let (first, last) = (r.stops[0].position, r.stops[r.stops.len() - 1].position);
        let (start, end) = (
            *r.path.first().expect("a traced path"),
            *r.path.last().unwrap(),
        );
        assert!(
            dist(start, first) <= 3.0,
            "{} starts off its first stop",
            r.key
        );
        assert!(dist(end, last) <= 3.0, "{} ends off its last stop", r.key);
        for w in r.path.windows(2) {
            let d = dist(w[0], w[1]);
            assert!(d <= 8.0, "{}: {:?} to {:?} is {d} yd", r.key, w[0], w[1]);
        }
        let length: f32 = r.path.windows(2).map(|w| dist(w[0], w[1])).sum();
        let straight: f32 = r
            .stops
            .windows(2)
            .map(|w| dist(w[0].position, w[1].position))
            .sum();
        let ratio = length / straight;
        assert!(
            (1.0..=1.6).contains(&ratio),
            "{}: {length} yd is {ratio}x the stops' {straight} yd",
            r.key
        );
    }
}

#[test]
fn the_cart_numbers_are_above_zero() {
    let cart = carts().cart;
    assert!(cart.speed_yd_s > 0.0);
    assert!(cart.dwell_s > 0.0);
    assert!(cart.seats > 0);
    assert!(cart.use_range_yd > 0.0);
}
