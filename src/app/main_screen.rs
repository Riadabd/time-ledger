use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::scroll_state::ScrollState;
use super::{App, Screen};
use crate::ledger::{apply_computed_times, save_week};

pub(super) fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
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
            code: KeyCode::Char('w'),
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            app.push_screen(Screen::Warnings(ScrollState {
                offset: 0,
                page_size: 5,
            }));
        }
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            if app.selected_day == 0 {
                app.shift_week(-1)?;
            } else {
                app.selected_day = app.selected_day.saturating_sub(1);
            }
        }
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            if app.selected_day == 6 {
                app.shift_week(1)?;
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
