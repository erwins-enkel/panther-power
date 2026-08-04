//! Catppuccin Mocha, and the magnitude gradient built from it.

use catppuccin::{PALETTE, Rgb};
use ratatui::style::Color;

const FLAVOR: &catppuccin::Flavor = &PALETTE.mocha;

pub fn text() -> Color {
    FLAVOR.colors.text.into()
}

pub fn dim() -> Color {
    FLAVOR.colors.overlay1.into()
}

pub fn accent() -> Color {
    FLAVOR.colors.mauve.into()
}

pub fn border() -> Color {
    FLAVOR.colors.surface1.into()
}

pub fn charging() -> Color {
    FLAVOR.colors.green.into()
}

/// btop's cool-to-hot ramp, in Mocha: green at rest, red at the peak.
const RAMP: [Rgb; 4] = [
    FLAVOR.colors.green.rgb,
    FLAVOR.colors.yellow.rgb,
    FLAVOR.colors.peach.rgb,
    FLAVOR.colors.red.rgb,
];

/// Sample the ramp at `t`, clamped to `0.0..=1.0`.
pub fn gradient(t: f64) -> Color {
    let t = t.clamp(0.0, 1.0) * (RAMP.len() - 1) as f64;
    let i = (t.floor() as usize).min(RAMP.len() - 2);
    let f = t - i as f64;
    let (a, b) = (RAMP[i], RAMP[i + 1]);
    let lerp = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f).round() as u8;
    Color::Rgb(lerp(a.r, b.r), lerp(a.g, b.g), lerp(a.b, b.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_runs_from_green_to_red() {
        let green = PALETTE.mocha.colors.green.rgb;
        let red = PALETTE.mocha.colors.red.rgb;
        assert_eq!(gradient(0.0), Color::Rgb(green.r, green.g, green.b));
        assert_eq!(gradient(1.0), Color::Rgb(red.r, red.g, red.b));
        // Out-of-range input clamps rather than wrapping or panicking.
        assert_eq!(gradient(-3.0), gradient(0.0));
        assert_eq!(gradient(9.0), gradient(1.0));
    }
}
