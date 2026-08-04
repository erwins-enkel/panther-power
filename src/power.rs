//! Battery state and the derivation of watts from raw sysfs counters.

/// Charge direction, normalised across the sysfs and UPower spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Unknown,
    Charging,
    Discharging,
    Full,
}

impl State {
    /// The `UP_DEVICE_STATE_*` codes carried in `GetHistory` tuples.
    pub fn from_upower(code: u32) -> Self {
        match code {
            1 => Self::Charging,
            2 => Self::Discharging,
            4 => Self::Full,
            _ => Self::Unknown,
        }
    }

    /// One lowercase word, for display and for machine-readable output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Charging => "charging",
            Self::Discharging => "discharging",
            Self::Full => "full",
            Self::Unknown => "unknown",
        }
    }

    /// The words in `/sys/class/power_supply/*/status`.
    pub fn from_sysfs(status: &str) -> Self {
        match status.trim() {
            "Charging" => Self::Charging,
            "Discharging" => Self::Discharging,
            "Full" => Self::Full,
            _ => Self::Unknown,
        }
    }
}

/// Raw counters for one battery read. Units are the sysfs units: µW, µA, µV.
#[derive(Debug, Clone, Copy, Default)]
pub struct Raw {
    pub power_now: Option<i64>,
    pub current_now: Option<i64>,
    pub voltage_now: Option<i64>,
}

/// Power draw in watts.
///
/// Prefers `power_now` where the vendor exposes it, else `current_now × voltage_now`.
/// Magnitude only — `current_now` stays positive while charging on some firmware and
/// negative on others, so direction comes from [`State`], never from the sign here.
pub fn watts(raw: &Raw) -> Option<f64> {
    if let Some(p) = raw.power_now {
        return Some((p as f64).abs() / 1e6);
    }
    let (i, v) = (raw.current_now?, raw.voltage_now?);
    Some((i as f64 * v as f64).abs() / 1e12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_power_now_where_the_vendor_exposes_it() {
        let raw = Raw {
            power_now: Some(8_440_000),
            current_now: Some(1),
            voltage_now: Some(1),
        };
        assert_eq!(watts(&raw), Some(8.44));
    }

    #[test]
    fn reports_magnitude_whichever_sign_the_firmware_uses() {
        // Some firmware signs the current while charging; direction comes from `status`.
        let negative = Raw {
            power_now: None,
            current_now: Some(-507_000),
            voltage_now: Some(16_642_000),
        };
        let positive = Raw {
            current_now: Some(507_000),
            ..negative
        };
        assert_eq!(watts(&negative), watts(&positive));
    }

    #[test]
    fn has_no_reading_without_both_current_and_voltage() {
        let raw = Raw {
            power_now: None,
            current_now: Some(507_000),
            voltage_now: None,
        };
        assert_eq!(watts(&raw), None);
    }

    #[test]
    fn reads_direction_from_both_spellings() {
        assert_eq!(State::from_sysfs("Discharging\n"), State::Discharging);
        assert_eq!(State::from_sysfs("Charging"), State::Charging);
        assert_eq!(State::from_sysfs("Full"), State::Full);
        assert_eq!(State::from_sysfs("Not charging"), State::Unknown);

        assert_eq!(State::from_upower(2), State::Discharging);
        assert_eq!(State::from_upower(1), State::Charging);
        assert_eq!(State::from_upower(4), State::Full);
        assert_eq!(State::from_upower(0), State::Unknown);
    }

    #[test]
    fn derives_watts_from_current_and_voltage() {
        // Measured against real hardware: 507 mA at 16.642 V read as 8.44 W.
        let raw = Raw {
            power_now: None,
            current_now: Some(507_000),
            voltage_now: Some(16_642_000),
        };
        let w = watts(&raw).expect("current and voltage are both present");
        assert!((w - 8.44).abs() < 0.005, "expected ~8.44 W, got {w}");
    }
}
