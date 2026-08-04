//! Backfill from the rate history UPower already keeps.
//!
//! The log files under `/var/lib/upower/` are root-only, but the same series is on the
//! system bus unprivileged — so the chart is populated at launch without a collector
//! daemon and without asking for privileges.

use anyhow::{Context, Result};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::ObjectPath;

use crate::history::{Sample, normalize};

const DEST: &str = "org.freedesktop.UPower";
const IFACE: &str = "org.freedesktop.UPower.Device";

/// Rate history for `battery` over the last `timespan_secs`, oldest first.
///
/// `max_points` is UPower's resolution argument: it thins the series to at most that
/// many samples, so ask for more than the chart can draw and let the chart decide.
pub fn history(battery: &str, timespan_secs: u32, max_points: u32) -> Result<Vec<Sample>> {
    let connection = Connection::system().context("connecting to the system bus")?;
    let path = ObjectPath::try_from(format!("/org/freedesktop/UPower/devices/battery_{battery}"))
        .context("building the UPower device path")?;
    let proxy = Proxy::new(&connection, DEST, path, IFACE).context("opening the UPower device")?;

    // Parsed structurally — the wire type is a(udu), whatever `gdbus` prints.
    let raw: Vec<(u32, f64, u32)> = proxy
        .call("GetHistory", &("rate", timespan_secs, max_points))
        .context("calling GetHistory")?;

    Ok(normalize(&raw))
}
