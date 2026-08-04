//! Summary statistics over a window of watt readings.

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
