//! The **Power Curve** (`GLOSSARY.md`, ADR 0012): P(level), the one number a level that a mob's
//! health and damage and a piece of Gear's budget grow by. The shape is `content/power_curve.toml`.

use std::sync::OnceLock;

use serde::Deserialize;

/// `content/power_curve.toml`.
#[derive(Deserialize)]
struct Curve {
    linear_to: u8,
    late_growth: f32,
}

fn curve() -> &'static Curve {
    static CURVE: OnceLock<Curve> = OnceLock::new();
    CURVE.get_or_init(|| {
        toml::from_str(include_str!("../../content/power_curve.toml"))
            .expect("content/power_curve.toml parses")
    })
}

/// P(`level`): `level / 10` up to the curve's `linear_to`, then `late_growth` compounding a
/// level after it. P(10) is 1.
pub fn at(level: u8) -> f32 {
    let c = curve();
    let level = level.max(1);
    if level <= c.linear_to {
        return f32::from(level) / 10.0;
    }
    f32::from(c.linear_to) / 10.0 * (1.0 + c.late_growth).powi(i32::from(level - c.linear_to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_is_a_tenth_a_level_to_30_then_three_percent_a_level() {
        assert_eq!(at(1), 0.1);
        assert_eq!(at(10), 1.0);
        assert_eq!(at(20), 2.0);
        assert_eq!(at(30), 3.0);
        assert!((at(31) - 3.09).abs() < 1e-5);
        assert!((at(60) - 3.0 * 1.03f32.powi(30)).abs() < 1e-4);
    }
}
