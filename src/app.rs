//! Application state: the series, the visible range, and how they are summarised.

use anyhow::Result;
use clap::ValueEnum;

use crate::battery::{Battery, now_unix};
use crate::cli::Cli;
use crate::history::{self, Sample};
use crate::power::State;
use crate::stats::Stats;

/// Selectable spans of the chart's x-axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Range {
    #[value(name = "15m")]
    M15,
    #[value(name = "1h")]
    H1,
    #[value(name = "3h")]
    H3,
    #[value(name = "12h")]
    H12,
}

impl Range {
    pub const ALL: [Self; 4] = [Self::M15, Self::H1, Self::H3, Self::H12];

    pub const fn secs(self) -> u64 {
        match self {
            Self::M15 => 15 * 60,
            Self::H1 => 60 * 60,
            Self::H3 => 3 * 60 * 60,
            Self::H12 => 12 * 60 * 60,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::H3 => "3h",
            Self::H12 => "12h",
        }
    }
}

/// Everything held in memory is bounded by the widest selectable range.
const RETAIN_SECS: u64 = Range::H12.secs();

/// Ask UPower for more detail than any terminal can draw, and let the chart thin it.
const BACKFILL_POINTS: u32 = 20_000;

pub struct App {
    pub battery: Battery,
    /// Chart marker, chosen once so terminals without Braille support stay usable.
    pub marker: ratatui::symbols::Marker,
    pub series: Vec<Sample>,
    pub range: Range,
    pub pack_wh: Option<f64>,
    /// Set when backfill failed, so the empty chart is explained rather than silent.
    pub notice: Option<String>,
}

impl App {
    pub fn new(cli: &Cli) -> Result<Self> {
        let battery = Battery::select(cli.battery.as_deref())?;
        let pack_wh = battery.pack_wh();

        // A failed backfill is not fatal: live sampling still works, the chart just
        // starts empty. Say so instead of implying there was no draw.
        let timespan = u32::try_from(RETAIN_SECS).unwrap_or(u32::MAX);
        let (series, notice) =
            match crate::upower::history(&battery.name, timespan, BACKFILL_POINTS) {
                Ok(series) => (series, None),
                Err(e) => (Vec::new(), Some(format!("no history from UPower: {e}"))),
            };

        Ok(Self {
            battery,
            marker: cli.marker.into(),
            series,
            range: cli.range,
            pack_wh,
            notice,
        })
    }

    /// Take one live reading.
    pub fn tick(&mut self) {
        if let Some(sample) = self.battery.sample() {
            history::push(&mut self.series, sample, RETAIN_SECS);
        }
    }

    /// Unix second at the left edge of the chart.
    pub fn window_start(&self) -> u64 {
        now_unix().saturating_sub(self.range.secs())
    }

    pub fn visible(&self) -> &[Sample] {
        history::window(&self.series, self.window_start())
    }

    /// Most recent reading, whatever its direction.
    pub fn latest(&self) -> Option<&Sample> {
        self.series.last()
    }

    /// Visible readings taken on battery.
    ///
    /// While charging, the same counters measure energy going into the pack rather than
    /// the draw of the machine. Plotting both as one series would conflate two different
    /// quantities, so time on AC is simply absent — and reads as the gap it is.
    pub fn visible_discharging(&self) -> Vec<Sample> {
        self.visible()
            .iter()
            .filter(|s| s.state == State::Discharging)
            .copied()
            .collect()
    }

    /// Whether the machine is currently off the battery, and so not being charted.
    ///
    /// Anything that is not an explicit discharge counts. Firmware with a charge threshold
    /// reports `Not charging` while plugged in and holding at the limit — sysfs has no
    /// dedicated word for it, so it arrives as [`State::Unknown`], and matching only on
    /// `Charging | Full` would blank the chart with "no samples" on every such machine.
    pub fn on_ac(&self) -> bool {
        self.latest().is_some_and(|s| s.state != State::Discharging)
    }

    /// Summary over the visible window, discharge only.
    ///
    /// Charging samples measure energy going the other way; averaging them with draw
    /// would report a number that describes neither.
    pub fn discharge_stats(&self) -> Option<Stats> {
        let watts: Vec<f64> = self.visible_discharging().iter().map(|s| s.watts).collect();
        Stats::of(&watts)
    }
}
