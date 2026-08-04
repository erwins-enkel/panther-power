//! Geometry for the banded magnitude gradient.
//!
//! `ratatui` paints one dataset in one style, so a btop-style green-to-red gradient is
//! built from one filled dataset per colour band. Each band clamps the series into its
//! own slice of the y-range: readings below the band collapse onto its floor and draw
//! nothing, readings above it saturate at its ceiling and fill it solid.

use crate::history::Sample;

/// Split one gap-free segment into the runs that reach into the band `lo..hi`, with x as
/// seconds since `t0` and y clamped to the band.
///
/// Only the stretches that actually rise above `lo` are emitted. Clamping the whole
/// segment instead would leave every below-band reading sitting exactly on the band
/// floor, painting a flat line across the full width of each band.
///
/// Each run is bracketed by its neighbouring readings pinned to the floor, so the filled
/// area rises out of the band boundary instead of starting with a vertical wall.
pub fn band_runs(segment: &[Sample], lo: f64, hi: f64, t0: u64) -> Vec<Vec<(f64, f64)>> {
    let point = |s: &Sample, y: f64| (s.ts.saturating_sub(t0) as f64, y);
    let mut runs = Vec::new();
    let mut i = 0;

    while i < segment.len() {
        if segment[i].watts <= lo {
            i += 1;
            continue;
        }
        let start = i;
        while i < segment.len() && segment[i].watts > lo {
            i += 1;
        }

        let mut run = Vec::with_capacity(i - start + 2);
        if start > 0 {
            run.push(point(&segment[start - 1], lo));
        }
        run.extend(segment[start..i].iter().map(|s| point(s, s.watts.min(hi))));
        if let Some(next) = segment.get(i) {
            run.push(point(next, lo));
        }
        runs.push(run);
    }
    runs
}

/// Round a y-axis ceiling up to the next 1, 2, 2.5 or 5 × 10ⁿ, so the axis labels land
/// on numbers a reader can divide in their head.
pub fn nice_ceil(x: f64) -> f64 {
    if x.is_nan() || x <= 0.0 {
        return 1.0;
    }
    let magnitude = 10f64.powf(x.log10().floor());
    let normalised = x / magnitude;
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|s| normalised <= *s + f64::EPSILON)
        .unwrap_or(10.0);
    step * magnitude
}

/// Hours as `13h 19m`.
pub fn fmt_hm(hours: f64) -> String {
    let total_minutes = (hours * 60.0).round() as u64;
    format!("{}h {:02}m", total_minutes / 60, total_minutes % 60)
}

/// Seconds before now, as an x-axis tick.
pub fn fmt_ago(secs: u64) -> String {
    match secs {
        0 => "now".to_owned(),
        s if s < 3600 => format!("-{}m", s / 60),
        s if s % 3600 == 0 => format!("-{}h", s / 3600),
        s => format!("-{}h{:02}m", s / 3600, (s % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::State;

    #[test]
    fn rounds_the_axis_ceiling_to_a_readable_number() {
        assert_eq!(nice_ceil(15.09), 20.0);
        assert_eq!(nice_ceil(8.4), 10.0);
        assert_eq!(nice_ceil(4.2), 5.0);
        assert_eq!(nice_ceil(2.1), 2.5);
        assert_eq!(nice_ceil(0.9), 1.0);
        assert_eq!(nice_ceil(0.0), 1.0, "an empty chart still needs an axis");
    }

    #[test]
    fn formats_projected_runtime() {
        // The 2026-08-04 reference session: 73.5 Wh pack at a 5.52 W median.
        assert_eq!(fmt_hm(73.5 / 5.52), "13h 19m");
    }

    #[test]
    fn formats_x_axis_ticks() {
        assert_eq!(fmt_ago(0), "now");
        assert_eq!(fmt_ago(900), "-15m");
        assert_eq!(fmt_ago(3600), "-1h");
        assert_eq!(fmt_ago(5400), "-1h30m");
    }

    fn sample(ts: u64, watts: f64) -> Sample {
        Sample {
            ts,
            watts,
            state: State::Discharging,
        }
    }

    #[test]
    fn emits_only_the_runs_that_reach_into_the_band() {
        let seg = [
            sample(100, 1.0),
            sample(130, 5.0),
            sample(160, 9.0),
            sample(190, 1.0),
            sample(220, 6.0),
        ];
        let runs = band_runs(&seg, 4.0, 8.0, 100);

        assert_eq!(runs.len(), 2, "the dip below 4 W splits the band");
        assert_eq!(
            runs[0],
            vec![(0.0, 4.0), (30.0, 5.0), (60.0, 8.0), (90.0, 4.0)],
            "in-band readings pass through, over-band saturates at the ceiling, \
             and the run is bracketed on the floor"
        );
        assert_eq!(
            runs[1],
            vec![(90.0, 4.0), (120.0, 6.0)],
            "a run still open at the end of the segment has no closing bracket"
        );
    }

    #[test]
    fn emits_nothing_for_a_band_the_segment_never_reaches() {
        let seg = [sample(100, 1.0), sample(130, 2.0)];
        assert!(band_runs(&seg, 4.0, 8.0, 100).is_empty());
    }
}
