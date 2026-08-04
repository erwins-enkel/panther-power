//! Command-line surface.

use std::time::Duration;

use clap::{Parser, ValueEnum};

use crate::app::Range;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Live terminal chart of laptop power draw, in braille"
)]
pub struct Cli {
    /// Battery to chart, e.g. BAT0. Defaults to the first with a readable draw
    #[arg(short, long, value_name = "NAME")]
    pub battery: Option<String>,

    /// List the batteries this machine exposes, then exit
    #[arg(long)]
    pub list_batteries: bool,

    /// Print one JSON snapshot and exit, instead of drawing. Waits one interval when the
    /// CPU panel is on, because its watts are a difference between two readings
    #[arg(long)]
    pub json: bool,

    /// Seconds between readings. Most embedded controllers only refresh about once a
    /// second, so anything faster repeats readings rather than resolving them
    #[arg(short, long, value_name = "SECS", default_value_t = 1.0)]
    pub interval: f64,

    /// Time range shown at startup
    #[arg(short, long, value_enum, default_value_t = Range::H1)]
    pub range: Range,

    /// Catppuccin flavour
    #[arg(long, value_enum, default_value_t = Flavour::Mocha)]
    pub theme: Flavour,

    /// Chart marker. Braille is finest but needs a font with the Braille Patterns block
    #[arg(long, value_enum, default_value_t = MarkerKind::Braille)]
    pub marker: MarkerKind,

    /// Colour depth. `auto` looks at COLORTERM
    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,

    /// CPU package power panel, read from the RAPL counters
    #[arg(long, value_enum, default_value_t = RaplMode::Auto)]
    pub rapl: RaplMode,
}

impl Cli {
    /// Poll period, floored at 100 ms so a stray `--interval 0` cannot spin the loop.
    pub fn poll(&self) -> Duration {
        Duration::from_secs_f64(self.interval.max(0.1))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Flavour {
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

impl From<Flavour> for catppuccin::FlavorName {
    fn from(flavour: Flavour) -> Self {
        match flavour {
            Flavour::Latte => Self::Latte,
            Flavour::Frappe => Self::Frappe,
            Flavour::Macchiato => Self::Macchiato,
            Flavour::Mocha => Self::Mocha,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum MarkerKind {
    Braille,
    HalfBlock,
    Block,
    Dot,
}

impl From<MarkerKind> for ratatui::symbols::Marker {
    fn from(kind: MarkerKind) -> Self {
        match kind {
            MarkerKind::Braille => Self::Braille,
            MarkerKind::HalfBlock => Self::HalfBlock,
            MarkerKind::Block => Self::Block,
            MarkerKind::Dot => Self::Dot,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum RaplMode {
    /// Show the panel when the counters are readable, and stay quiet when they are not
    Auto,
    /// Require the panel, and refuse to start if the counters cannot be read
    On,
    /// Never show it
    Off,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    /// Use 24-bit colour if `COLORTERM` advertises it
    Auto,
    /// Always emit 24-bit colour
    Truecolor,
    /// Use the terminal's own 16 colours, so the palette follows its theme
    Ansi,
}

/// Whether to emit 24-bit colour.
///
/// Terminals that cannot render it quantise every shade of the gradient to the nearest
/// entry in their own palette, which collapses sixteen bands into a handful of steps.
/// Falling back to named colours instead keeps the ramp legible and lets the terminal's
/// theme drive it.
pub fn wants_truecolor(mode: ColorMode, colorterm: Option<&str>) -> bool {
    match mode {
        ColorMode::Truecolor => true,
        ColorMode::Ansi => false,
        ColorMode::Auto => colorterm.is_some_and(|v| {
            v.contains("truecolor") || v.contains("24bit") || v.contains("24-bit")
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_detects_truecolor_from_the_environment() {
        assert!(wants_truecolor(ColorMode::Auto, Some("truecolor")));
        assert!(wants_truecolor(ColorMode::Auto, Some("24bit")));
        assert!(!wants_truecolor(ColorMode::Auto, Some("")));
        assert!(!wants_truecolor(ColorMode::Auto, None));
    }

    #[test]
    fn explicit_modes_ignore_the_environment() {
        assert!(wants_truecolor(ColorMode::Truecolor, None));
        assert!(!wants_truecolor(ColorMode::Ansi, Some("truecolor")));
    }

    #[test]
    fn poll_never_reaches_zero() {
        let cli = Cli::parse_from(["wattmeter", "--interval", "0"]);
        assert_eq!(cli.poll(), Duration::from_millis(100));
    }

    #[test]
    fn verifies_the_command_line() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
