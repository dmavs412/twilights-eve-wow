//! The pull near the edge of the mist (#662, decision 6): within [`EDGE_NEAR_YD`] of the nearest
//! closed chunk the scene fog closes in and greys, so the whole screen feels the Hush. The mist
//! takes the scene fog's colour, so it greys with the air. The sound fades by the same
//! [`edge_pull`].

/// The pull starts this far from the nearest closed chunk.
pub const EDGE_NEAR_YD: f32 = 60.0;
/// The scene fog's start at the edge.
pub const PULL_FOG_START_YD: f32 = 20.0;
/// The scene fog's end at the edge (Elwynn by day is 125 to 500).
pub const PULL_FOG_END_YD: f32 = 200.0;
/// How far the fog colour moves toward its own grey at the edge.
pub const PULL_GREY: f32 = 0.6;
/// The most the applied pull changes in a second: a zone that opens or a teleport eases the fog
/// over about 4 s instead of snapping.
pub const PULL_EASE_PER_S: f32 = 0.25;

/// The pull for the distance in yards to the nearest closed chunk: 1 at the edge, 0 at
/// [`EDGE_NEAR_YD`] and beyond, 0 for `None` (no closed chunk that near).
pub fn edge_pull(distance: Option<f32>) -> f32 {
    let Some(d) = distance else {
        return 0.0;
    };
    let t = (d / EDGE_NEAR_YD).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// `current` moved toward `target` by at most [`PULL_EASE_PER_S`] × `dt`, never past it.
pub fn ease(current: f32, target: f32, dt: f32) -> f32 {
    let step = PULL_EASE_PER_S * dt;
    current + (target - current).clamp(-step, step)
}

/// `c` moved toward its own grey (luma 0.299, 0.587, 0.114) by `k`.
fn grey(c: [f32; 3], k: f32) -> [f32; 3] {
    let luma = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
    c.map(|v| v + (luma - v) * k)
}

/// The scene fog `(start, end, color, sky)` under `pull`: the pair closes toward
/// ([`PULL_FOG_START_YD`], [`PULL_FOG_END_YD`]) but never opens past its own, and the colour and
/// the five sky stops grey by [`PULL_GREY`] × `pull`, so the mist's top still meets the sky.
pub fn pulled(
    pull: f32,
    fog_start: f32,
    fog_end: f32,
    fog_color: [f32; 3],
    sky: [[f32; 3]; 5],
) -> (f32, f32, [f32; 3], [[f32; 3]; 5]) {
    let lerp = |a: f32, b: f32| a + (b - a) * pull;
    let end = fog_end.min(lerp(fog_end, PULL_FOG_END_YD));
    let start = fog_start.min(lerp(fog_start, PULL_FOG_START_YD)).min(end);
    let k = PULL_GREY * pull;
    (start, end, grey(fog_color, k), sky.map(|s| grey(s, k)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_pull_is_full_at_the_edge_and_gone_at_60_yd() {
        assert_eq!(edge_pull(Some(0.0)), 1.0);
        assert_eq!(edge_pull(Some(60.0)), 0.0);
        assert_eq!(edge_pull(Some(200.0)), 0.0);
        assert_eq!(edge_pull(None), 0.0);
        let mut last = edge_pull(Some(0.0));
        for i in 1..=80 {
            let p = edge_pull(Some(i as f32));
            assert!(p <= last, "rises at {i} yd");
            last = p;
        }
    }

    #[test]
    fn ease_moves_a_quarter_a_second_and_never_past_the_target() {
        assert_eq!(ease(0.0, 1.0, 1.0), 0.25);
        assert_eq!(ease(0.9, 1.0, 1.0), 1.0);
        assert_eq!(ease(1.0, 0.0, 1.0), 0.75);
        assert_eq!(ease(0.1, 0.0, 1.0), 0.0);
    }

    #[test]
    fn pulled_closes_the_fog_and_greys_the_colour() {
        let sky = [[0.2, 0.4, 0.8]; 5];
        let c = [0.3, 0.5, 0.9];
        let (s, e, _, _) = pulled(1.0, 125.0, 500.0, c, sky);
        assert!(
            (s - 20.0).abs() < 1e-4 && (e - 200.0).abs() < 1e-4,
            "({s}, {e})"
        );
        assert_eq!(pulled(0.0, 125.0, 500.0, c, sky), (125.0, 500.0, c, sky));
        let (s, e, _, _) = pulled(1.0, -139.0, 278.0, c, sky);
        assert!(
            (s + 139.0).abs() < 1e-4 && (e - 200.0).abs() < 1e-4,
            "storm ({s}, {e})"
        );

        let luma = |c: [f32; 3]| c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
        let (_, _, g, gsky) = pulled(1.0, 125.0, 500.0, c, sky);
        assert!((luma(g) - luma(c)).abs() < 1e-5);
        for i in 0..3 {
            let (before, after) = (c[i] - luma(c), g[i] - luma(c));
            assert!((after - 0.4 * before).abs() < 1e-5, "channel {i}");
        }
        for s in gsky {
            assert!((luma(s) - luma(sky[0])).abs() < 1e-5);
            assert!((s[2] - luma(sky[0]) - 0.4 * (sky[0][2] - luma(sky[0]))).abs() < 1e-5);
        }
    }
}
