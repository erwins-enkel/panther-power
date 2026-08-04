//! Live sampling from `/sys/class/power_supply`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};

use crate::history::Sample;
use crate::power::{Raw, State, watts};

const SUPPLY_ROOT: &str = "/sys/class/power_supply";

pub struct Battery {
    pub name: String,
    root: PathBuf,
}

impl Battery {
    /// First battery that reports a usable draw. Not every machine calls it `BAT0`.
    pub fn discover() -> Result<Self> {
        let mut entries: Vec<PathBuf> = fs::read_dir(SUPPLY_ROOT)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        entries.sort();

        entries
            .into_iter()
            .find(|p| {
                read_str(p, "type").as_deref() == Some("Battery")
                    && (p.join("power_now").exists() || p.join("current_now").exists())
            })
            .map(|root| Self {
                name: root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                root,
            })
            .ok_or_else(|| anyhow!("no battery with a readable power draw under {SUPPLY_ROOT}"))
    }

    pub fn state(&self) -> State {
        read_str(&self.root, "status")
            .map(|s| State::from_sysfs(&s))
            .unwrap_or(State::Unknown)
    }

    /// Charge remaining, as a percentage.
    pub fn capacity(&self) -> Option<i64> {
        self.num("capacity")
    }

    /// Usable pack energy in watt-hours.
    ///
    /// Charge-reporting firmware (µAh) has no energy counter, so pair it with the design
    /// voltage — the nominal figure, not the sagging instantaneous one.
    pub fn pack_wh(&self) -> Option<f64> {
        if let Some(uwh) = self.num("energy_full") {
            return Some(uwh as f64 / 1e6);
        }
        let uah = self.num("charge_full")?;
        let uv = self
            .num("voltage_min_design")
            .or_else(|| self.num("voltage_now"))?;
        Some(uah as f64 * uv as f64 / 1e12)
    }

    /// One reading, stamped with the wall clock so it lines up with UPower history.
    pub fn sample(&self) -> Option<Sample> {
        let raw = Raw {
            power_now: self.num("power_now"),
            current_now: self.num("current_now"),
            voltage_now: self.num("voltage_now"),
        };
        Some(Sample {
            ts: now_unix(),
            watts: watts(&raw)?,
            state: self.state(),
        })
    }

    fn num(&self, file: &str) -> Option<i64> {
        read_str(&self.root, file)?.parse().ok()
    }
}

fn read_str(root: &Path, file: &str) -> Option<String> {
    fs::read_to_string(root.join(file))
        .ok()
        .map(|s| s.trim().to_owned())
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
