//! Summary statistics over a window of watt readings.

use crate::history::{Sample, segments};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p90: f64,
}

impl Stats {
    /// Summarise a window. Returns `None` for an empty one rather than inventing zeros.
    pub fn of(watts: &[f64]) -> Option<Self> {
        if watts.is_empty() {
            return None;
        }
        let mut sorted = watts.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        Some(Self {
            n,
            min: sorted[0],
            max: sorted[n - 1],
            mean: sorted.iter().sum::<f64>() / n as f64,
            median: median(&sorted),
            // Nearest rank, so p90 is always a reading that actually occurred.
            p90: sorted[(((n as f64) * 0.9).ceil() as usize).max(1) - 1],
        })
    }
}

/// Midpoint of an already-sorted window, averaging the middle pair when even.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

/// Hours the pack would last at this draw. `None` when nothing is being drawn.
pub fn runtime_hours(pack_wh: f64, watts: f64) -> Option<f64> {
    (watts > 0.0).then(|| pack_wh / watts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarises_a_window() {
        // Ten readings, deliberately unsorted. Worked by hand:
        // sorted 1..=10 → min 1, max 10, mean 5.5, median (5+6)/2 = 5.5,
        // p90 by nearest rank = ceil(0.9 × 10) = 9th smallest = 9.
        let watts = [7.0, 2.0, 9.0, 4.0, 1.0, 10.0, 3.0, 8.0, 5.0, 6.0];
        let s = Stats::of(&watts).expect("a non-empty window has stats");
        assert_eq!(s.n, 10);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 10.0);
        assert_eq!(s.mean, 5.5);
        assert_eq!(s.median, 5.5);
        assert_eq!(s.p90, 9.0);
    }

    #[test]
    fn has_no_stats_for_an_empty_window() {
        assert_eq!(Stats::of(&[]), None);
    }
}

/// Energy observed in a window, and how much of that window it actually covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Energy {
    pub wh: f64,
    /// Seconds of real sampling behind the figure.
    ///
    /// Reported alongside the total because they are rarely the same: an hour-wide window
    /// containing twenty minutes on mains has forty minutes of evidence in it, and
    /// presenting the total as "energy used in the last hour" would overstate the coverage.
    pub covered_secs: u64,
}

/// Integrate power over time, within each gap-free segment.
///
/// Trapezoidal, so a ramp between two readings counts as a ramp rather than a step. Gaps
/// contribute nothing at all: there is no evidence for what happened across them, and
/// bridging one would manufacture energy that was never observed.
pub fn energy(samples: &[Sample], gap_secs: u64) -> Energy {
    let mut wh = 0.0;
    let mut covered_secs = 0;

    for segment in segments(samples, gap_secs) {
        for pair in segment.windows(2) {
            let seconds = pair[1].ts.saturating_sub(pair[0].ts);
            let mean_watts = (pair[0].watts + pair[1].watts) / 2.0;
            wh += mean_watts * seconds as f64 / 3600.0;
            covered_secs += seconds;
        }
    }
    Energy { wh, covered_secs }
}

#[cfg(test)]
mod energy_tests {
    use super::*;
    use crate::history::GAP_SECS;
    use crate::power::State;

    fn sample(ts: u64, watts: f64) -> Sample {
        Sample {
            ts,
            watts,
            state: State::Discharging,
        }
    }

    #[test]
    fn integrates_power_over_time() {
        // A flat 10 W held for an hour is 10 Wh.
        let flat: Vec<Sample> = (0..=60).map(|m| sample(m * 60, 10.0)).collect();
        let e = energy(&flat, GAP_SECS);
        assert!((e.wh - 10.0).abs() < 1e-9, "expected 10 Wh, got {}", e.wh);
        assert_eq!(e.covered_secs, 3600);
    }

    #[test]
    fn treats_a_ramp_as_a_ramp() {
        // 0 W rising steadily to 10 W over an hour averages 5 W, so 5 Wh — not 0, and not
        // the 10 Wh a last-value-wins sum would give. Sampled every minute, because two
        // readings an hour apart are a gap, not a ramp.
        let ramp: Vec<Sample> = (0..=60)
            .map(|m| sample(m * 60, 10.0 * m as f64 / 60.0))
            .collect();
        let e = energy(&ramp, GAP_SECS);
        assert!((e.wh - 5.0).abs() < 1e-9, "expected 5 Wh, got {}", e.wh);
        assert_eq!(e.covered_secs, 3600);
    }

    #[test]
    fn counts_no_energy_across_a_gap() {
        // Half an hour at 10 W, three hours absent, half an hour at 10 W: 10 Wh observed
        // over one hour of evidence. Bridging the gap would invent about 30 Wh.
        let mut samples: Vec<Sample> = (0..=30).map(|m| sample(m * 60, 10.0)).collect();
        samples.extend((0..=30).map(|m| sample(11_000 + m * 60, 10.0)));

        let e = energy(&samples, GAP_SECS);
        assert!((e.wh - 10.0).abs() < 1e-9, "expected 10 Wh, got {}", e.wh);
        assert_eq!(
            e.covered_secs, 3600,
            "only the sampled hour counts as covered"
        );
    }

    #[test]
    fn a_lone_reading_is_not_evidence_of_energy() {
        let e = energy(&[sample(0, 10.0)], GAP_SECS);
        assert_eq!(e.wh, 0.0);
        assert_eq!(e.covered_secs, 0);
    }
}
