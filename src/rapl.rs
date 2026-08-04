//! CPU power from the RAPL energy counters under `/sys/class/powercap`.
//!
//! These are free-running energy accumulators, not power readings: watts come from the
//! difference between two samples over the time between them.
//!
//! This is **not** the same quantity as battery draw. It covers the CPU package only —
//! not the display, the radios, or anything else the pack is feeding.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const POWERCAP_ROOT: &str = "/sys/class/powercap";

/// Longest interval a delta is trusted over.
///
/// The counter also resets across suspend, and a reset is indistinguishable from a wrap,
/// so a long interval is discarded rather than turned into an invented spike.
const MAX_TRUSTED_INTERVAL: Duration = Duration::from_secs(10);

/// Why there is no CPU power to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// No powercap tree — a VM, a non-x86 machine, or a kernel without the driver.
    NotPresent,
    /// The counters are there but root-only, which is the kernel's default.
    PermissionDenied,
}

impl Unavailable {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NotPresent => "no RAPL counters on this machine",
            Self::PermissionDenied => "RAPL counters are root-only — see the README",
        }
    }
}

/// Energy accumulated between two readings, in microjoules.
///
/// `max` is the counter's wrap point, from `max_energy_range_uj`.
pub fn delta_uj(previous: u64, current: u64, max: u64) -> u64 {
    if current >= previous {
        current - previous
    } else {
        // Wrapped: what was left to the top, plus what has accumulated since.
        max.saturating_sub(previous).saturating_add(current)
    }
}

/// Average power over an interval, or `None` when the interval is not worth trusting.
pub fn power(previous: u64, current: u64, max: u64, elapsed: Duration) -> Option<f64> {
    if elapsed.is_zero() || elapsed > MAX_TRUSTED_INTERVAL {
        return None;
    }
    // µJ / s = µW; scale to watts.
    Some(delta_uj(previous, current, max) as f64 / elapsed.as_secs_f64() / 1e6)
}

/// One RAPL domain: a package, or a zone within it.
pub struct Domain {
    /// Sysfs directory name, e.g. `intel-rapl:0` or `intel-rapl:0:1`. Nesting encodes
    /// parentage: `intel-rapl:0:1` is a zone within `intel-rapl:0`.
    pub id: String,
    pub name: String,
    energy: PathBuf,
    max_uj: u64,
    previous: Option<(u64, Instant)>,
    /// Most recent power reading, once two samples exist.
    pub watts: Option<f64>,
}

impl Domain {
    fn read_uj(&self) -> Option<u64> {
        read_num(&self.energy)
    }

    /// Take a reading. The first one only establishes a baseline.
    fn sample(&mut self, at: Instant) {
        let Some(current) = self.read_uj() else {
            self.watts = None;
            return;
        };
        self.watts = self
            .previous
            .and_then(|(prev, then)| power(prev, current, self.max_uj, at.duration_since(then)));
        self.previous = Some((current, at));
    }
}

pub struct Rapl {
    pub domains: Vec<Domain>,
}

impl Rapl {
    pub fn discover() -> Result<Self, Unavailable> {
        Self::discover_at(Path::new(POWERCAP_ROOT))
    }

    /// Split out so tests can point at a fixture tree.
    pub fn discover_at(root: &Path) -> Result<Self, Unavailable> {
        let Ok(entries) = fs::read_dir(root) else {
            return Err(Unavailable::NotPresent);
        };

        // The mmio zones mirror the msr ones; taking both would double-count.
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("intel-rapl:"))
            })
            .collect();
        paths.sort();

        if paths.is_empty() {
            return Err(Unavailable::NotPresent);
        }

        let mut domains = Vec::new();
        let mut denied = false;
        for path in paths {
            let energy = path.join("energy_uj");
            // Distinguish "not readable" from "not there": the first is fixable by the
            // user, the second is not, and telling them apart is the whole point.
            match fs::read_to_string(&energy) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    denied = true;
                    continue;
                }
                Err(_) => continue,
            }
            let (Some(name), Some(max_uj)) = (
                read_str(&path.join("name")),
                read_num(&path.join("max_energy_range_uj")),
            ) else {
                continue;
            };
            let Some(id) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
                continue;
            };
            domains.push(Domain {
                id,
                name,
                energy,
                max_uj,
                previous: None,
                watts: None,
            });
        }

        match (domains.is_empty(), denied) {
            (true, true) => Err(Unavailable::PermissionDenied),
            (true, false) => Err(Unavailable::NotPresent),
            _ => Ok(Self { domains }),
        }
    }

    pub fn sample(&mut self) {
        let at = Instant::now();
        for domain in &mut self.domains {
            domain.sample(at);
        }
    }

    /// The domain to chart: the CPU package, else the whole-platform zone.
    pub fn primary(&self) -> Option<&Domain> {
        self.domains
            .iter()
            .find(|d| d.name.starts_with("package-"))
            .or_else(|| self.domains.iter().find(|d| d.name == "psys"))
            .or_else(|| self.domains.first())
    }

    /// Zones *within* the charted package — core, uncore, dram.
    ///
    /// Kept apart from [`Self::platform`] because they are components of the number on the
    /// chart, whereas a platform zone is a different measurement altogether.
    pub fn subzones(&self) -> impl Iterator<Item = &Domain> {
        let prefix = self.primary().map(|d| format!("{}:", d.id));
        self.domains.iter().filter(move |d| {
            prefix
                .as_ref()
                .is_some_and(|p| d.id.starts_with(p.as_str()))
        })
    }

    /// Top-level zones that are not the charted package.
    ///
    /// `psys` is the whole platform, not the CPU: on this hardware it reads roughly double
    /// the package. Reporting it inside a CPU breakdown would invite exactly the
    /// conflation this tool is trying to avoid.
    pub fn platform(&self) -> impl Iterator<Item = &Domain> {
        let primary = self.primary().map(|d| d.id.clone());
        self.domains.iter().filter(move |d| {
            // A top-level zone has one colon: `intel-rapl:1`, not `intel-rapl:0:1`.
            d.id.matches(':').count() == 1 && Some(&d.id) != primary.as_ref()
        })
    }
}

fn read_str(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn read_num(path: &Path) -> Option<u64> {
    read_str(path)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wrap point this code was written against.
    const MAX: u64 = 262_143_328_850;

    #[test]
    fn measures_power_between_two_readings() {
        // One joule over one second is one watt.
        assert_eq!(power(0, 1_000_000, MAX, Duration::from_secs(1)), Some(1.0));
        // Seven and a half joules over three seconds is two and a half watts.
        assert_eq!(
            power(1_000_000, 8_500_000, MAX, Duration::from_secs(3)),
            Some(2.5)
        );
    }

    #[test]
    fn accounts_for_counter_wraparound() {
        // 2 J short of the top, wrapping round to 1 J past it: 3 J total, not a negative.
        let previous = MAX - 2_000_000;
        let current = 1_000_000;
        assert_eq!(delta_uj(previous, current, MAX), 3_000_000);
        assert_eq!(
            power(previous, current, MAX, Duration::from_secs(1)),
            Some(3.0)
        );
    }

    #[test]
    fn rejects_an_interval_too_long_to_trust() {
        // Across a suspend the counter resets, which reads exactly like a wrap. Reporting
        // a number here would invent a spike that never happened.
        assert_eq!(power(0, 5_000_000, MAX, Duration::from_secs(3600)), None);
        assert_eq!(power(0, 5_000_000, MAX, Duration::ZERO), None);
    }

    fn fixture(name: &str, domains: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("pp-rapl-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (i, (domain, energy)) in domains.iter().enumerate() {
            let dir = root.join(format!("intel-rapl:{i}"));
            fs::create_dir_all(&dir).expect("fixture dir");
            fs::write(dir.join("name"), domain).expect("name");
            fs::write(dir.join("max_energy_range_uj"), MAX.to_string()).expect("max");
            if !energy.is_empty() {
                fs::write(dir.join("energy_uj"), energy).expect("energy");
            }
        }
        root
    }

    #[test]
    fn discovers_readable_domains() {
        let root = fixture("ok", &[("package-0", "1000"), ("core", "500")]);
        let rapl = Rapl::discover_at(&root).expect("both domains are readable");
        assert_eq!(rapl.domains.len(), 2);
        assert_eq!(rapl.primary().map(|d| d.name.as_str()), Some("package-0"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_an_absent_tree_separately_from_an_unreadable_one() {
        assert_eq!(
            Rapl::discover_at(Path::new("/nonexistent")).err(),
            Some(Unavailable::NotPresent)
        );

        // A domain whose energy_uj is missing entirely is not a permissions problem.
        let root = fixture("empty", &[("package-0", "")]);
        assert_eq!(
            Rapl::discover_at(&root).err(),
            Some(Unavailable::NotPresent)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn charts_the_platform_zone_when_there_is_no_package() {
        let root = fixture("psys", &[("psys", "1000"), ("dram", "500")]);
        let rapl = Rapl::discover_at(&root).expect("readable");
        assert_eq!(rapl.primary().map(|d| d.name.as_str()), Some("psys"));
        let _ = fs::remove_dir_all(&root);
    }
}
