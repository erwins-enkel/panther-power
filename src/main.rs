//! A live chart of laptop power draw, in the terminal.

mod app;
mod battery;
mod chart;
mod cli;
mod history;
mod power;
mod rapl;
mod stats;
mod theme;
mod ui;
mod upower;

use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use app::{App, Range};
use battery::Battery;
use cli::{Cli, wants_truecolor};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_batteries {
        return list_batteries();
    }

    theme::init(
        cli.theme.into(),
        wants_truecolor(cli.color, std::env::var("COLORTERM").ok().as_deref()),
    );

    let mut app = App::new(&cli)?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &cli);
    ratatui::restore();
    result
}

/// Printed plainly rather than drawn, so it can be piped and read without a terminal.
fn list_batteries() -> Result<()> {
    let found = Battery::discover_all()?;
    if found.is_empty() {
        println!("no battery with a readable power draw");
        return Ok(());
    }
    for battery in &found {
        let pack = battery
            .pack_wh()
            .map_or_else(|| "unknown capacity".to_owned(), |wh| format!("{wh:.1} Wh"));
        println!("{}  {pack}", battery.name);
    }
    Ok(())
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App, cli: &Cli) -> Result<()> {
    let poll = cli.poll();
    let mut next_sample = Instant::now();

    loop {
        if Instant::now() >= next_sample {
            app.tick();
            // Scheduled from now rather than by advancing a fixed cadence: a laptop
            // monitor gets suspended, and a fixed cadence would come back owing hours of
            // missed ticks and spin through them all before drawing anything.
            next_sample = Instant::now() + poll;
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
