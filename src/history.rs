//! UPower `GetHistory` normalisation and gap segmentation.

use crate::power::State;

/// One power-draw observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Unix seconds.
    pub ts: u64,
    pub watts: f64,
    pub state: State,
}

/// Normalise raw `(unix_ts, watts, state)` tuples from `GetHistory`.
///
/// UPower hands them back newest-first, and emits `rate == 0.0` artifacts around AC
/// transitions. Sorting is on the timestamp alone — a series left partly out of order
/// fabricates gaps and reorders the extremes.
pub fn normalize(raw: &[(u32, f64, u32)]) -> Vec<Sample> {
    let mut out: Vec<Sample> = raw
        .iter()
        .filter(|(_, watts, _)| *watts != 0.0)
        .map(|&(ts, watts, state)| Sample {
            ts: u64::from(ts),
            watts,
            state: State::from_upower(state),
        })
        .collect();
    out.sort_by_key(|s| s.ts);
    out
}

/// Longest silence still treated as continuous sampling.
///
/// UPower logs about every second under load and every thirty at rest, so the threshold
/// has to clear 30 s comfortably without swallowing a real absence.
pub const GAP_SECS: u64 = 120;

/// Split a normalised series wherever sampling stopped for longer than `gap_secs`.
///
/// Drawing one line across a gap would invent data: a three-hour suspend has to read as
/// absence, not as a straight interpolation between the samples either side of it.
pub fn segments(samples: &[Sample], gap_secs: u64) -> Vec<&[Sample]> {
    let mut segs = Vec::new();
    let mut start = 0;
    for i in 1..samples.len() {
        if samples[i].ts.saturating_sub(samples[i - 1].ts) > gap_secs {
            segs.push(&samples[start..i]);
            start = i;
        }
    }
    if start < samples.len() {
        segs.push(&samples[start..]);
    }
    segs
}

/// Append a live reading, then drop anything older than `retain_secs` behind it.
///
/// Readings at or before the newest one are ignored: the backfill and the first live
/// polls overlap, and the EC only refreshes about once a second, so a faster poll
/// re-reads the same value rather than resolving a new one.
pub fn push(series: &mut Vec<Sample>, sample: Sample, retain_secs: u64) {
    if series.last().is_some_and(|last| sample.ts <= last.ts) {
        return;
    }
    series.push(sample);
    let horizon = sample.ts.saturating_sub(retain_secs);
    let stale = series.partition_point(|s| s.ts < horizon);
    series.drain(..stale);
}

/// The tail of a series at or after `since`.
pub fn window(series: &[Sample], since: u64) -> &[Sample] {
    &series[series.partition_point(|s| s.ts < since)..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: u64, watts: f64) -> Sample {
        Sample {
            ts,
            watts,
            state: State::Discharging,
        }
    }

    #[test]
    fn push_ignores_readings_the_series_already_covers() {
        let mut series = vec![sample(1_000, 5.0), sample(1_030, 5.2)];
        push(&mut series, sample(1_030, 9.9), 3_600);
        push(&mut series, sample(1_010, 9.9), 3_600);
        assert_eq!(
            series.len(),
            2,
            "an overlapping backfill must not duplicate"
        );
        push(&mut series, sample(1_060, 5.1), 3_600);
        assert_eq!(series.len(), 3);
    }

    #[test]
    fn push_evicts_readings_past_the_retention_horizon() {
        let mut series = vec![sample(1_000, 5.0), sample(4_000, 5.2)];
        push(&mut series, sample(5_000, 5.1), 2_000);
        assert_eq!(
            series.iter().map(|s| s.ts).collect::<Vec<_>>(),
            vec![4_000, 5_000],
            "only the last 2000 s survive"
        );
    }

    #[test]
    fn window_takes_the_tail_at_or_after_the_cutoff() {
        let series = [sample(100, 5.0), sample(200, 5.1), sample(300, 5.2)];
        assert_eq!(window(&series, 200).len(), 2);
        assert_eq!(window(&series, 301).len(), 0);
        assert_eq!(window(&series, 0).len(), 3);
    }

    #[test]
    fn breaks_the_series_across_a_suspend() {
        // 30 s at rest is normal cadence; the three-hour hole is a suspend.
        let samples = [
            sample(1_000, 5.0),
            sample(1_030, 5.2),
            sample(1_060, 5.1),
            sample(12_000, 6.0),
            sample(12_030, 6.1),
        ];
        let segs = segments(&samples, GAP_SECS);
        assert_eq!(segs.len(), 2, "the suspend must break the line");
        assert_eq!(segs[0].len(), 3);
        assert_eq!(segs[1].len(), 2);
    }

    #[test]
    fn sorts_ascending_and_drops_zero_rate_artifacts() {
        // GetHistory returns newest-first, and emits rate == 0.0 at AC transitions.
        let raw = [
            (1_785_873_687, 7.63, 2u32),
            (1_785_873_657, 0.0, 2),
            (1_785_873_627, 19.98, 2),
        ];
        let out = normalize(&raw);
        assert_eq!(out.len(), 2, "the zero-rate artifact must be dropped");
        assert_eq!(out[0].ts, 1_785_873_627);
        assert_eq!(out[1].ts, 1_785_873_687);
    }
}
