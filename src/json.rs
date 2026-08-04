//! One-shot machine-readable snapshot, for status bars and scripts.

use anyhow::Result;
use serde_json::{Value, json};

use crate::app::App;
use crate::cli::Cli;
use crate::power::State;

/// Print a snapshot and return.
///
/// CPU power is a difference between two counter readings, so when the panel is available
/// this waits one poll interval to take the second one. Without it there would be nothing
/// to report but a baseline. `--rapl off` skips the wait.
pub fn emit(app: &mut App, cli: &Cli) -> Result<()> {
    app.tick();
    if app.rapl.is_some() {
        std::thread::sleep(cli.poll());
        app.tick();
    }
    println!("{}", serde_json::to_string_pretty(&snapshot(app))?);
    Ok(())
}

fn snapshot(app: &App) -> Value {
    let stats = app.discharge_stats();
    let energy = app.window_energy();
    let state = app.latest().map_or(State::Unknown, |s| s.state);
    let watts = app.latest().map(|s| s.watts);

    // Split rather than shared: while charging these counters measure energy going into
    // the pack, and reporting that under `draw_watts` would hand a consumer a number that
    // means the opposite of what its name says.
    let (draw, charge) = match state {
        State::Discharging => (watts, None),
        State::Charging => (None, watts),
        _ => (None, None),
    };

    json!({
        "timestamp": app.now(),
        "battery": {
            "name": app.battery.name,
            "state": state.as_str(),
            "capacity_percent": app.battery.capacity(),
            "pack_wh": app.pack_wh,
            "draw_watts": draw,
            "charge_watts": charge,
        },
        "window": {
            "range": app.range.label(),
            "discharging_samples": stats.map_or(0, |s| s.n),
            "median_watts": stats.map(|s| s.median),
            "mean_watts": stats.map(|s| s.mean),
            "p90_watts": stats.map(|s| s.p90),
            "min_watts": stats.map(|s| s.min),
            "max_watts": stats.map(|s| s.max),
            "energy_wh": energy.wh,
            // Stated so a consumer can tell a full window from one that is mostly gap.
            "covered_seconds": energy.covered_secs,
        },
        "full_pack_hours_at_median": app
            .pack_wh
            .zip(stats.map(|s| s.median))
            .and_then(|(wh, median)| crate::stats::runtime_hours(wh, median)),
        "cpu": cpu(app),
    })
}

fn cpu(app: &App) -> Value {
    let Some(rapl) = &app.rapl else {
        return json!({
            "available": false,
            "reason": app.rapl_missing.map_or("not requested", |e| e.reason()),
        });
    };

    let zones: Value = rapl
        .subzones()
        .filter_map(|d| d.watts.map(|w| (d.name.clone(), json!(w))))
        .collect::<serde_json::Map<_, _>>()
        .into();
    // Kept out of `zones`: a platform domain is not a component of the package.
    let platform: Value = rapl
        .platform()
        .filter_map(|d| d.watts.map(|w| (d.name.clone(), json!(w))))
        .collect::<serde_json::Map<_, _>>()
        .into();

    json!({
        "available": true,
        "zone": rapl.primary().map(|d| d.name.clone()),
        "watts": rapl.primary().and_then(|d| d.watts),
        "zones": zones,
        "platform": platform,
    })
}
