mod app;
mod cli;
mod ledger;
mod time_amount;
mod ui;

use std::io;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Local};
use clap::Parser;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::cli::Cli;
use crate::ledger::{empty_week, load_week, week_file_path, week_start_for};

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("error: {err}");
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let today = Local::now().date_naive();
    let week_start = week_start_for(today);
    let file_path = week_file_path(&cli.ledger_dir, today);

    let mut week = load_week(&file_path, week_start)?;
    if week.days.is_empty() {
        week = empty_week(week.week_start);
    }

    let mut app = App::new(week, file_path, cli.ledger_dir);
    app.selected_day = today.weekday().num_days_from_monday() as usize;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    let stdout = terminal.backend_mut();
    crossterm::execute!(stdout, LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(StdDuration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && app.handle_key(key)?
        {
            break;
        }
    }
    Ok(())
}
