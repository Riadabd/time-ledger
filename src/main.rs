mod ledger;
mod time_amount;
mod ui;

use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use chrono::{Datelike, Duration, Local};
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::ledger::{
    apply_computed_times, empty_week, load_week, load_week_if_exists, save_week, week_file_name,
    week_start_for,
};
use crate::ui::App;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:?}");
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let today = Local::now().date_naive();
    let week_start = week_start_for(today);
    let file_name = week_file_name(today);
    let file_path = PathBuf::from("data").join(file_name);

    let mut week = load_week(&file_path, week_start)?;
    if week.days.is_empty() {
        week = empty_week(week.week_start);
    }

    let mut app = App::new(week, file_path);
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
            && handle_key(app, key)?
        {
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }
        | KeyEvent {
            code: KeyCode::Esc, ..
        } => return Ok(true),
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            apply_computed_times(&mut app.week)?;
            save_week(&app.file_path, &app.week)?;
            app.refresh();
            app.status = "Saved".to_string();
        }
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            if app.selected_day == 0 {
                shift_week(app, -1)?;
            } else {
                app.selected_day = app.selected_day.saturating_sub(1);
            }
        }
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            if app.selected_day == 6 {
                shift_week(app, 1)?;
            } else {
                app.selected_day = (app.selected_day + 1).min(6);
            }
        }
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            app.selected_task = app.selected_task.saturating_sub(1);
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            if !app.tasks.is_empty() {
                app.selected_task = (app.selected_task + 1).min(app.tasks.len() - 1);
            }
        }
        _ => {}
    }

    Ok(false)
}

fn shift_week(app: &mut App, direction: i64) -> Result<(), Box<dyn Error>> {
    let week_start = week_start_for(Local::now().date_naive());
    let candidate_week = app.week.week_start + Duration::days(7 * direction);
    let file_name = week_file_name(candidate_week);
    let file_path = PathBuf::from("data").join(file_name);

    let week = if candidate_week == week_start {
        load_week(&file_path, candidate_week)?
    } else {
        match load_week_if_exists(&file_path, candidate_week)? {
            Some(week) => week,
            None => return Ok(()),
        }
    };

    app.week = week;
    app.file_path = file_path;
    app.refresh();
    app.selected_day = if direction < 0 { 6 } else { 0 };

    Ok(())
}
