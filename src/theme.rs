//! Catppuccin palette, and the magnitude gradient built from it.

use std::sync::OnceLock;

use catppuccin::{Flavor, FlavorName, PALETTE, Rgb};
use ratatui::style::Color;

/// Chosen once at startup and read from every draw.
static THEME: OnceLock<Theme> = OnceLock::new();

pub struct Theme {
    flavor: &'static Flavor,
    truecolor: bool,
}

/// Fix the palette for the process. Later calls are ignored.
pub fn init(flavor: FlavorName, truecolor: bool) {
    let _ = THEME.set(Theme {
        flavor: PALETTE.get_flavor(flavor),
        truecolor,
    });
}

fn theme() -> &'static Theme {
    THEME.get_or_init(|| Theme {
        flavor: PALETTE.get_flavor(FlavorName::Mocha),
        truecolor: true,
    })
}

/// A palette colour, or its nearest named equivalent where 24-bit colour is unavailable.
fn shade(pick: impl Fn(&Flavor) -> Rgb, fallback: Color) -> Color {
    let theme = theme();
    if theme.truecolor {
        let rgb = pick(theme.flavor);
        Color::Rgb(rgb.r, rgb.g, rgb.b)
    } else {
        fallback
    }
}

pub fn text() -> Color {
    shade(|f| f.colors.text.rgb, Color::White)
}

pub fn dim() -> Color {
    shade(|f| f.colors.overlay1.rgb, Color::DarkGray)
}

pub fn accent() -> Color {
    shade(|f| f.colors.mauve.rgb, Color::Magenta)
}

pub fn border() -> Color {
    shade(|f| f.colors.surface1.rgb, Color::DarkGray)
}

pub fn charging() -> Color {
    shade(|f| f.colors.green.rgb, Color::Green)
}

/// btop's cool-to-hot ramp: green at rest, red at the peak.
fn ramp(flavor: &Flavor) -> [Rgb; 4] {
    [
        flavor.colors.green.rgb,
        flavor.colors.yellow.rgb,
        flavor.colors.peach.rgb,
        flavor.colors.red.rgb,
    ]
}

/// The same four stops as named colours, for terminals without 24-bit colour. Stepped
/// rather than interpolated, so the ramp stays legible under the terminal's own theme.
const ANSI_RAMP: [Color; 4] = [Color::Green, Color::Yellow, Color::LightRed, Color::Red];

/// Sample the ramp at `t`, clamped to `0.0..=1.0`.
pub fn gradient(t: f64) -> Color {
    let theme = theme();
    let t = t.clamp(0.0, 1.0);

    if !theme.truecolor {
        let i = (t * ANSI_RAMP.len() as f64) as usize;
        return ANSI_RAMP[i.min(ANSI_RAMP.len() - 1)];
    }

    let stops = ramp(theme.flavor);
    let scaled = t * (stops.len() - 1) as f64;
    let i = (scaled.floor() as usize).min(stops.len() - 2);
    let f = scaled - i as f64;
    let (a, b) = (stops[i], stops[i + 1]);
    let lerp = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f).round() as u8;
    Color::Rgb(lerp(a.r, b.r), lerp(a.g, b.g), lerp(a.b, b.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_runs_from_green_to_red() {
        // Exercises the default (mocha, truecolor); `init` is global, so the tests cannot
        // each pick their own theme.
        let green = PALETTE.mocha.colors.green.rgb;
        let red = PALETTE.mocha.colors.red.rgb;
        assert_eq!(gradient(0.0), Color::Rgb(green.r, green.g, green.b));
        assert_eq!(gradient(1.0), Color::Rgb(red.r, red.g, red.b));
        // Out-of-range input clamps rather than wrapping or panicking.
        assert_eq!(gradient(-3.0), gradient(0.0));
        assert_eq!(gradient(9.0), gradient(1.0));
    }

    #[test]
    fn every_flavour_exposes_the_ramp() {
        for flavor in PALETTE.all_flavors() {
            let stops = ramp(flavor);
            assert_eq!(stops.len(), 4, "{:?} is missing ramp stops", flavor.name);
        }
    }
}
