//! A live chart of laptop power draw, in the terminal.

mod app;
mod battery;
mod chart;
mod history;
mod power;
mod stats;
mod theme;
mod ui;
mod upower;

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use app::{App, Range};

/// The embedded controller only refreshes about once a second; polling faster repeats
/// readings rather than resolving them.
const POLL: Duration = Duration::from_secs(1);

fn main() -> Result<()> {
    let mut app = App::new()?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    let mut next_sample = Instant::now();

    loop {
        if Instant::now() >= next_sample {
            app.tick();
            // Scheduled from now rather than by advancing a fixed cadence: a laptop
            // monitor gets suspended, and a fixed cadence would come back owing hours of
            // missed ticks and spin through them all before drawing anything.
            next_sample = Instant::now() + POLL;
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        let wait = next_sample.saturating_duration_since(Instant::now());
        if event::poll(wait)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            // Raw mode swallows the terminal's own interrupt, so ctrl-c has to be answered
            // here or the only way out is `q`.
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('1') => app.range = Range::M15,
                KeyCode::Char('2') => app.range = Range::H1,
                KeyCode::Char('3') => app.range = Range::H3,
                KeyCode::Char('4') => app.range = Range::H12,
                _ => {}
            }
        }
    }
}
