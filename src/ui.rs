//! Rendering.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, FilledLine};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, Range};
use crate::chart::{band_runs, fmt_ago, fmt_hm, fmt_watts, nice_ceil};
use crate::history::{GAP_SECS, Sample, segments};
use crate::power::State;
use crate::rapl::Unavailable;
use crate::stats::{Stats, runtime_hours};
use crate::theme;

/// Colour steps in the magnitude gradient.
const BANDS: usize = 16;

pub fn draw(frame: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_header(frame, header, app);

    let battery = app.visible_discharging();
    let cpu = app.visible_cpu();

    // One gutter width for both panels: different y-ceilings would otherwise indent the
    // two time axes differently, and the whole point of stacking them is to line up a
    // spike in draw against a spike in the CPU.
    let gutter = gutter_width(&[y_ceiling(&battery), y_ceiling(cpu)]);

    if cpu.is_empty() {
        draw_chart(frame, body, app, &battery, battery_title(app), gutter, true);
    } else {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(body);
        draw_chart(frame, top, app, &battery, battery_title(app), gutter, true);
        draw_chart(frame, bottom, app, cpu, cpu_title(app), gutter, false);
    }

    draw_footer(frame, footer, app);
}

fn block(title: Line<'_>) -> Block<'_> {
    Block::bordered()
        .border_style(Style::default().fg(theme::border()))
        .title(title)
}

fn stat<'a>(label: &'a str, value: String) -> Vec<Span<'a>> {
    vec![
        Span::styled(label, Style::default().fg(theme::dim())),
        Span::raw(" "),
        Span::styled(value, Style::default().fg(theme::text())),
        Span::raw("   "),
    ]
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.latest().map_or(State::Unknown, |s| s.state);
    let state_label = match state {
        State::Charging => "charging",
        State::Discharging => "discharging",
        State::Full => "full",
        State::Unknown => "unknown",
    };
    let state_color = match state {
        State::Charging | State::Full => theme::charging(),
        _ => theme::dim(),
    };

    let mut title = vec![
        Span::styled(" panther-power ", Style::default().fg(theme::accent())),
        Span::styled(&app.battery.name, Style::default().fg(theme::text())),
        Span::raw(" "),
        Span::styled(state_label, Style::default().fg(state_color)),
    ];
    if let Some(pct) = app.battery.capacity() {
        title.push(Span::styled(
            format!(" {pct}%"),
            Style::default().fg(theme::dim()),
        ));
    }
    title.push(Span::raw(" "));

    let stats = app.discharge_stats();
    // On AC the same counters read the charge rate, so the live figure is labelled for
    // what it is rather than passed off as draw.
    let live_label = match state {
        State::Charging => "charging at",
        State::Full => "topping up",
        _ => "now",
    };
    let mut top = stat(
        live_label,
        app.latest()
            .map_or_else(|| "—".to_owned(), |s| format!("{:.2} W", s.watts)),
    );
    top.extend(stat("median", opt_watts(stats.map(|s| s.median))));
    top.extend(stat("mean", opt_watts(stats.map(|s| s.mean))));
    top.extend(stat("p90", opt_watts(stats.map(|s| s.p90))));

    let mut bottom = stat("min", opt_watts(stats.map(|s| s.min)));
    bottom.extend(stat("peak", opt_watts(stats.map(|s| s.max))));
    bottom.extend(stat(
        "pack",
        app.pack_wh
            .map_or_else(|| "—".to_owned(), |wh| format!("{wh:.1} Wh")),
    ));

    // Projected only from the discharge median, and only when the pack size is known —
    // an extrapolation from a charging window would be meaningless.
    let projection = app
        .pack_wh
        .zip(stats.map(|s| s.median))
        .and_then(|(wh, median)| runtime_hours(wh, median))
        .map(fmt_hm);
    // Endurance of a *full* pack, which is the benchmark figure — not time remaining at
    // the current charge. Labelled so it cannot be read as the latter next to the
    // percentage in the title.
    bottom.extend(stat(
        "full-pack at median",
        projection.unwrap_or_else(|| "—".to_owned()),
    ));
    frame.render_widget(
        Paragraph::new(vec![Line::from(top), Line::from(bottom)]).block(block(Line::from(title))),
        area,
    );
}

fn opt_watts(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |w| format!("{w:.2} W"))
}

/// The count belongs with the chart it describes, and keeps the stat row short enough to
/// survive a narrow terminal.
fn sample_count(stats: Option<Stats>) -> String {
    match stats {
        Some(s) => format!("{} discharging samples ", s.n),
        None => "no discharging samples ".to_owned(),
    }
}

fn battery_title(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::raw(" battery draw "),
        Span::styled(
            format!("last {} · ", app.range.label()),
            Style::default().fg(theme::dim()),
        ),
        Span::styled(
            sample_count(app.discharge_stats()),
            Style::default().fg(theme::dim()),
        ),
    ])
}

/// Names the zone and breaks out the others, so the number is not mistaken for the
/// machine's draw — it is the CPU package alone.
fn cpu_title(app: &App) -> Line<'static> {
    let Some(rapl) = &app.rapl else {
        return Line::from(" cpu ");
    };
    let zone = rapl.primary().map_or("cpu", |d| d.name.as_str());
    let mut spans = vec![Span::raw(format!(" cpu {zone} "))];

    let breakdown: Vec<String> = rapl
        .subzones()
        .filter_map(|d| d.watts.map(|w| format!("{} {w:.2} W", d.name)))
        .collect();
    if !breakdown.is_empty() {
        spans.push(Span::styled(
            format!("· {} ", breakdown.join(" · ")),
            Style::default().fg(theme::dim()),
        ));
    }

    // Named apart from the zones above: `psys` measures the whole platform, so summing it
    // with them, or reading it as a CPU figure, would be wrong.
    let platform: Vec<String> = rapl
        .platform()
        .filter_map(|d| d.watts.map(|w| format!("{} {w:.2} W", d.name)))
        .collect();
    if !platform.is_empty() {
        spans.push(Span::styled(
            format!("· platform {} ", platform.join(" · ")),
            Style::default().fg(theme::dim()),
        ));
    }
    Line::from(spans)
}

/// Y-axis ceiling for a series, or `None` when there is nothing to scale to.
fn y_ceiling(samples: &[Sample]) -> Option<f64> {
    (!samples.is_empty()).then(|| nice_ceil(samples.iter().fold(0.0_f64, |m, s| m.max(s.watts))))
}

fn gutter_width(ceilings: &[Option<f64>]) -> u16 {
    ceilings
        .iter()
        .flatten()
        .flat_map(|c| y_labels(*c))
        .map(|l| l.width())
        .max()
        .unwrap_or(1) as u16
        + 1
}

fn draw_chart(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    visible: &[Sample],
    title: Line<'_>,
    gutter: u16,
    is_battery: bool,
) {
    // Read the clock once, before the window is taken: a second ticking over between the
    // two would put `t0` after the cutoff the samples were filtered on, and the oldest
    // reading would saturate to x = 0 and be drawn on the left edge.
    let t0 = app.window_start();

    if visible.is_empty() {
        let message = if !is_battery {
            "waiting for a second reading".to_owned()
        } else if app.on_ac() {
            "on AC — charting resumes on battery".to_owned()
        } else {
            app.notice
                .clone()
                .unwrap_or_else(|| "no samples in this range".to_owned())
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme::dim()))
                .alignment(Alignment::Center)
                .block(block(title)),
            area,
        );
        return;
    }

    let y_max = y_ceiling(visible).unwrap_or(1.0);
    let span = app.range.secs() as f64;
    let runs = band_data(visible, y_max, t0);

    let outer = block(title);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let labels = y_labels(y_max);
    let [plot_row, x_axis_row] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
    let columns = Layout::horizontal([Constraint::Length(gutter), Constraint::Min(1)]);
    let [gutter_area, plot] = columns.areas(plot_row);
    // The tick row is inset by the same gutter, or every label sits `gutter` columns left
    // of the moment it names.
    let [_, x_axis] = columns.areas(x_axis_row);

    draw_y_axis(frame, gutter_area, &labels);
    draw_x_axis(frame, x_axis, app.range);

    // Every band is painted into a single canvas layer, deliberately: `Chart` puts each
    // dataset in its own layer, where a later layer's cell replaces the earlier one
    // outright, so a band boundary falling inside a character cell erases the band below
    // it and stripes the chart. Within one layer the dots merge instead.
    //
    // Bands are drawn top-down and every one fills to the axis, so each cell ends up
    // coloured by the lowest band that reaches it — which is the band that owns that
    // height.
    frame.render_widget(
        Canvas::default()
            .marker(app.marker)
            .x_bounds([0.0, span])
            .y_bounds([0.0, y_max])
            .paint(|ctx| {
                for (band, points) in &runs {
                    let color = theme::gradient((*band as f64 + 0.5) / BANDS as f64);
                    for pair in points.windows(2) {
                        ctx.draw(&FilledLine {
                            x1: pair[0].0,
                            y1: pair[0].1,
                            x2: pair[1].0,
                            y2: pair[1].1,
                            fill_to_y: 0.0,
                            color,
                        });
                    }
                    // A lone reading has no pair, and would otherwise vanish.
                    if let [only] = points[..] {
                        ctx.draw(&FilledLine {
                            x1: only.0,
                            y1: only.1,
                            x2: only.0,
                            y2: only.1,
                            fill_to_y: 0.0,
                            color,
                        });
                    }
                }
            }),
        plot,
    );
}

/// Watt labels down the left gutter: ceiling at the top, zero on the axis.
fn draw_y_axis(frame: &mut Frame, area: Rect, labels: &[Line<'static>]) {
    let style = Style::default().fg(theme::dim());
    for (i, label) in labels.iter().rev().enumerate() {
        let Some(y) = tick_row(area, i, labels.len()) else {
            continue;
        };
        frame.render_widget(
            Paragraph::new(label.clone())
                .style(style)
                .alignment(Alignment::Right),
            Rect {
                x: area.x,
                y,
                width: area.width.saturating_sub(1),
                height: 1,
            },
        );
    }
}

/// Row for tick `i` of `count`, counting down from the top of `area`.
///
/// Rounded, not truncated: the canvas rounds when it maps a value to a dot row, so a
/// truncating tick puts the midpoint label a row above the height it labels whenever the
/// plot is an even number of rows tall.
fn tick_row(area: Rect, i: usize, count: usize) -> Option<u16> {
    if area.height == 0 {
        return None;
    }
    let span = u32::from(area.height - 1);
    let steps = (count.max(2) - 1) as u32;
    let offset = (span * i as u32 + steps / 2) / steps;
    Some(area.y + offset as u16)
}

/// Start, midpoint and `now`, spread across the width in one pass.
fn draw_x_axis(frame: &mut Frame, area: Rect, range: Range) {
    let secs = range.secs();
    let (start, mid, end) = (fmt_ago(secs), fmt_ago(secs / 2), fmt_ago(0));
    let width = area.width as usize;
    let used = start.len() + mid.len() + end.len();
    if width <= used {
        return;
    }

    let before_mid = (width - used) / 2;
    let after_mid = width - used - before_mid;
    let line = format!(
        "{start}{:before_mid$}{mid}{:after_mid$}{end}",
        "",
        "",
        before_mid = before_mid,
        after_mid = after_mid
    );
    frame.render_widget(
        Paragraph::new(line).style(Style::default().fg(theme::dim())),
        area,
    );
}

const fn band_floor(band: usize, y_max: f64) -> f64 {
    y_max * band as f64 / BANDS as f64
}

/// One entry per (band, run) that has anything to draw, gaps already broken.
///
/// Ordered from the top band down, because each band is painted all the way to the axis
/// and so must be overpainted by every band below it.
///
/// Band is the outer loop, not segment: a cell carries one colour, so ordering by segment
/// first would let a later segment's high band land on a cell an earlier segment's low
/// band already owns, and colour it for a height it does not reach.
fn band_data(visible: &[Sample], y_max: f64, t0: u64) -> Vec<(usize, Vec<(f64, f64)>)> {
    let segments = segments(visible, GAP_SECS);
    let mut out = Vec::new();
    for band in (0..BANDS).rev() {
        let (lo, hi) = (band_floor(band, y_max), band_floor(band + 1, y_max));
        for segment in &segments {
            out.extend(
                band_runs(segment, lo, hi, t0)
                    .into_iter()
                    .map(|run| (band, run)),
            );
        }
    }
    out
}

fn y_labels(y_max: f64) -> Vec<Line<'static>> {
    (0..=2)
        .map(|i| Line::from(fmt_watts(y_max * f64::from(i) / 2.0)))
        .collect()
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = Vec::new();
    for (i, range) in Range::ALL.iter().enumerate() {
        let selected = *range == app.range;
        spans.push(Span::styled(
            format!(" {} ", i + 1),
            Style::default().fg(theme::accent()),
        ));
        spans.push(Span::styled(
            format!("{}  ", range.label()),
            if selected {
                Style::default()
                    .fg(theme::text())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::dim())
            },
        ));
    }
    spans.push(Span::styled(" q ", Style::default().fg(theme::accent())));
    spans.push(Span::styled("quit", Style::default().fg(theme::dim())));

    // Keep a failed backfill visible even once live sampling has filled the chart.
    if let Some(notice) = &app.notice {
        spans.push(Span::styled(
            format!("   {notice}"),
            Style::default().fg(theme::dim()),
        ));
    }

    // Only when there is something the reader could do about it. A machine with no RAPL
    // counters at all cannot be fixed by telling them about it every frame.
    if app.rapl_missing == Some(Unavailable::PermissionDenied) {
        spans.push(Span::styled(
            format!("   {}", Unavailable::PermissionDenied.reason()),
            Style::default().fg(theme::dim()),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Sample;
    use crate::power::State;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const NOW: u64 = 1_800_000_000;

    fn sample(ts: u64, watts: f64, state: State) -> Sample {
        Sample { ts, watts, state }
    }

    fn render(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
        terminal.draw(|frame| draw(frame, app)).expect("draw");
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..buffer.area().width)
            .filter_map(|x| buffer.cell((x, y)))
            .map(|cell| cell.symbol())
            .collect()
    }

    /// Which columns of the chart body have anything painted in them.
    fn painted_columns(buffer: &ratatui::buffer::Buffer) -> Vec<bool> {
        let area = *buffer.area();
        // The chart block sits below the four-row header; skip its border and the tick row.
        let rows = 6..area.height.saturating_sub(3);
        (0..area.width)
            .map(|x| {
                rows.clone().any(|y| {
                    buffer
                        .cell((x, y))
                        .is_some_and(|c| !c.symbol().trim().is_empty())
                })
            })
            .collect()
    }

    #[test]
    fn renders_a_gap_as_blank_columns() {
        // An hour of draw, an hour asleep, an hour of draw — in a three-hour window.
        let mut series = Vec::new();
        for i in 0..120 {
            series.push(sample(NOW - 10_800 + i * 30, 6.0, State::Discharging));
        }
        for i in 0..120 {
            series.push(sample(NOW - 3_600 + i * 30, 6.0, State::Discharging));
        }
        let app = App::for_test(series, NOW, Range::H3);

        let painted = painted_columns(&render(&app, 100, 24));
        let width = painted.len();
        let filled = |range: std::ops::Range<usize>| painted[range].iter().filter(|p| **p).count();

        // Thirds of the plot: drawn, absent, drawn. The middle third must stay empty —
        // interpolating across it would invent an hour of readings that never happened.
        assert!(filled(10..width / 3) > 10, "the first hour should be drawn");
        assert_eq!(
            filled(width / 2 - 8..width / 2 + 8),
            0,
            "the sleeping hour must be blank, not bridged"
        );
        assert!(
            filled(width * 3 / 4..width - 2) > 10,
            "the last hour should be drawn"
        );
    }

    #[test]
    fn explains_an_empty_window_rather_than_drawing_nothing() {
        let app = App::for_test(vec![sample(NOW - 5, 40.0, State::Charging)], NOW, Range::H1);
        let buffer = render(&app, 90, 20);
        let text: String = (0..buffer.area().height)
            .map(|y| row(&buffer, y))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            text.contains("on AC"),
            "an AC window needs a reason, got:\n{text}"
        );
        assert!(
            text.contains("charging at"),
            "the live figure is charge rate, not draw"
        );
    }

    #[test]
    fn stat_row_survives_a_narrow_terminal() {
        let series = (0..60)
            .map(|i| sample(NOW - 3_600 + i * 60, 6.0, State::Discharging))
            .collect();
        let buffer = render(&App::for_test(series, NOW, Range::H1), 88, 20);

        // The last stat on each row is the one a truncating layout drops first.
        assert!(
            row(&buffer, 1).contains("p90"),
            "row 1: {}",
            row(&buffer, 1)
        );
        assert!(
            row(&buffer, 2).contains("full-pack at median"),
            "row 2: {}",
            row(&buffer, 2)
        );
    }
}
