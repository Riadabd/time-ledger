use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, Screen};

pub(super) fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
    let line_count = app.warnings_line_count();
    // Modal overlay: consume navigation keys so the main UI doesn't move underneath.
    match key {
        KeyEvent {
            code: KeyCode::Char('w'),
            modifiers: KeyModifiers::NONE,
            ..
        } => {
            app.screen = Screen::Main;
        }
        KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }
        | KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            app.screen = Screen::Main;
        }
        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                state.scroll_by(-1, line_count);
            }
        }
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                state.scroll_by(1, line_count);
            }
        }
        KeyEvent {
            code: KeyCode::PageUp,
            ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                let delta = state.page_size.max(1) as i32;
                state.scroll_by(-delta, line_count);
            }
        }
        KeyEvent {
            code: KeyCode::PageDown,
            ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                let delta = state.page_size.max(1) as i32;
                state.scroll_by(delta, line_count);
            }
        }
        KeyEvent {
            code: KeyCode::Home,
            ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                state.scroll = 0;
            }
        }
        KeyEvent {
            code: KeyCode::End, ..
        } => {
            if let Some(state) = app.warnings_overlay_state_mut() {
                state.scroll = state.max_scroll(line_count);
            }
        }
        // Ignore all other keys while the overlay is open.
        _ => {}
    }

    Ok(false)
}
