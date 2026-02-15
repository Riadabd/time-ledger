use std::error::Error;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edit_core::{Action, Buffer, Editor, Viewport};

use crate::app::App;
use crate::app::scroll_state::ScrollState;
use crate::ledger::{
    Day, apply_computed_times, format_minutes, parse_day, render_day, resolve_entry, save_week,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DayPaneKind {
    View,
    Edit,
}

#[derive(Clone, Debug)]
pub(super) struct DayEditState {
    editor: Editor,
    diagnostics: Vec<String>,
    diagnostics_scroll: ScrollState,
}

pub(super) enum DayPane {
    View,
    Edit(DayEditState),
}

pub(super) fn kind(pane: &DayPane) -> DayPaneKind {
    match pane {
        DayPane::View => DayPaneKind::View,
        DayPane::Edit(_) => DayPaneKind::Edit,
    }
}

pub(super) fn enter_edit_mode(app: &mut App) {
    if !matches!(app.day_pane, DayPane::View) {
        return;
    }

    let text = selected_day_text(app);
    let (parsed_day, diagnostics) = parse_and_diagnostics(&text);
    let canonical_text = match parsed_day {
        Some(day) => render_day(&day),
        None => text,
    };

    app.day_pane = DayPane::Edit(DayEditState {
        editor: Editor::new(Buffer::from_text(&canonical_text), Viewport::new(1, 1)),
        diagnostics,
        diagnostics_scroll: ScrollState {
            offset: 0,
            page_size: 5,
        },
    });
    clamp_diagnostics_scroll(app);
    app.status = "Edit mode".to_string();
}

pub(super) fn exit_edit_mode(app: &mut App) {
    if matches!(app.day_pane, DayPane::Edit(_)) {
        app.day_pane = DayPane::View;
        app.status = format!("Warnings: {}", app.week.warnings.len());
    }
}

pub(super) fn handle_edit_key(app: &mut App, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    if key.code == KeyCode::Esc {
        exit_edit_mode(app);
        return Ok(());
    }

    if matches_ctrl_s(key) {
        save_current_edit(app)?;
        return Ok(());
    }

    if handle_diagnostics_scroll_key(app, key) {
        return Ok(());
    }

    if let Some(action) = key_to_editor_action(key) {
        apply_editor_action(app, action);
    }

    Ok(())
}

pub(super) fn editor_visible_lines(app: &App) -> Option<Vec<String>> {
    match &app.day_pane {
        DayPane::View => None,
        DayPane::Edit(state) => Some(state.editor.visible_lines()),
    }
}

pub(super) fn editor_cursor_screen_pos(app: &App) -> Option<(usize, usize)> {
    match &app.day_pane {
        DayPane::View => None,
        DayPane::Edit(state) => Some(state.editor.cursor_screen_pos()),
    }
}

pub(super) fn set_editor_viewport(app: &mut App, height: usize, width: usize) {
    if let DayPane::Edit(state) = &mut app.day_pane {
        state
            .editor
            .set_viewport(Viewport::new(height.max(1), width.max(1)));
    }
}

pub(super) fn diagnostics_lines(app: &App) -> Option<&[String]> {
    match &app.day_pane {
        DayPane::View => None,
        DayPane::Edit(state) => Some(state.diagnostics.as_slice()),
    }
}

pub(super) fn diagnostics_scroll(app: &App) -> Option<usize> {
    match &app.day_pane {
        DayPane::View => None,
        DayPane::Edit(state) => Some(state.diagnostics_scroll.offset),
    }
}

pub(super) fn set_diagnostics_page_size(app: &mut App, page_size: usize) {
    let line_count = diagnostics_line_count(app);
    if let DayPane::Edit(state) = &mut app.day_pane {
        state
            .diagnostics_scroll
            .set_page_size(page_size, line_count);
    }
}

pub(super) fn diagnostics_line_count(app: &App) -> usize {
    match &app.day_pane {
        DayPane::View => 0,
        DayPane::Edit(state) => state.diagnostics.len().max(1),
    }
}

fn apply_editor_action(app: &mut App, action: Action) {
    if let DayPane::Edit(state) = &mut app.day_pane {
        state.editor.apply(action);
        let content = state.editor.buffer().as_text();
        let (_parsed_day, diagnostics) = parse_and_diagnostics(&content);
        state.diagnostics = diagnostics;
        state
            .diagnostics_scroll
            .clamp(state.diagnostics.len().max(1));
    }
}

fn save_current_edit(app: &mut App) -> Result<(), Box<dyn Error>> {
    let (text, viewport) = match &app.day_pane {
        DayPane::View => return Ok(()),
        DayPane::Edit(state) => (state.editor.buffer().as_text(), state.editor.viewport()),
    };

    let (parsed_day, mut diagnostics) = parse_and_diagnostics(&text);
    let day = match parsed_day {
        Some(day) => day,
        None => {
            set_diagnostics(app, diagnostics);
            app.status = "Fix diagnostics before saving".to_string();
            return Ok(());
        }
    };

    let date = app.selected_date();
    app.week.days.insert(date, day);

    if let Err(err) =
        apply_computed_times(&mut app.week).and_then(|_| save_week(&app.file_path, &app.week))
    {
        diagnostics.push(format!("Save failed: {err}"));
        set_diagnostics(app, diagnostics);
        app.status = "Save failed".to_string();
        return Ok(());
    }

    app.refresh();
    app.status = "Saved".to_string();

    let canonical_text = selected_day_text(app);
    let (_day, diagnostics_after_save) = parse_and_diagnostics(&canonical_text);
    if let DayPane::Edit(state) = &mut app.day_pane {
        state.editor = Editor::new(Buffer::from_text(&canonical_text), viewport);
        state.diagnostics = diagnostics_after_save;
    }
    clamp_diagnostics_scroll(app);

    Ok(())
}

fn set_diagnostics(app: &mut App, diagnostics: Vec<String>) {
    if let DayPane::Edit(state) = &mut app.day_pane {
        state.diagnostics = diagnostics;
    }
    clamp_diagnostics_scroll(app);
}

pub(super) fn clamp_diagnostics_scroll(app: &mut App) {
    let line_count = diagnostics_line_count(app);
    if let DayPane::Edit(state) = &mut app.day_pane {
        state.diagnostics_scroll.clamp(line_count);
    }
}

fn selected_day_text(app: &App) -> String {
    match app.week.days.get(&app.selected_date()) {
        Some(day) => render_day(day),
        None => String::new(),
    }
}

fn parse_and_diagnostics(text: &str) -> (Option<Day>, Vec<String>) {
    let (day, mut diagnostics) = parse_day(text);
    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }

    for entry in &day.entries {
        let resolved = resolve_entry(entry);
        if !resolved.mismatch {
            continue;
        }

        if let (Some(parent), Some(sub_total)) = (entry.time, resolved.sub_total_minutes) {
            diagnostics.push(format!(
                "Mismatch: '{}' parent @{} vs sub-items @{}",
                entry.name.trim(),
                parent.format(),
                format_minutes(sub_total)
            ));
        } else {
            diagnostics.push(format!("Mismatch: '{}'", entry.name.trim()));
        }
    }

    (Some(day), diagnostics)
}

fn matches_ctrl_s(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers,
            ..
        } if modifiers == KeyModifiers::CONTROL
    )
}

fn handle_diagnostics_scroll_key(app: &mut App, key: KeyEvent) -> bool {
    let line_count = diagnostics_line_count(app);
    if let DayPane::Edit(state) = &mut app.day_pane {
        match key.code {
            KeyCode::PageUp => {
                state.diagnostics_scroll.page_up(line_count);
                return true;
            }
            KeyCode::PageDown => {
                state.diagnostics_scroll.page_down(line_count);
                return true;
            }
            KeyCode::Home => {
                state.diagnostics_scroll.home();
                return true;
            }
            KeyCode::End => {
                state.diagnostics_scroll.end(line_count);
                return true;
            }
            _ => {}
        }
    }

    false
}

fn key_to_editor_action(key: KeyEvent) -> Option<Action> {
    match key {
        KeyEvent {
            code: KeyCode::Left,
            ..
        } => Some(Action::MoveLeft),
        KeyEvent {
            code: KeyCode::Right,
            ..
        } => Some(Action::MoveRight),
        KeyEvent {
            code: KeyCode::Up, ..
        } => Some(Action::MoveUp),
        KeyEvent {
            code: KeyCode::Down,
            ..
        } => Some(Action::MoveDown),
        KeyEvent {
            code: KeyCode::Backspace,
            ..
        } => Some(Action::DeleteBackward),
        KeyEvent {
            code: KeyCode::Delete,
            ..
        } => Some(Action::DeleteForward),
        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => Some(Action::Newline),
        KeyEvent {
            code: KeyCode::Tab, ..
        } => Some(Action::Insert('\t')),
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
            Some(Action::Insert(ch))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::NaiveDate;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use edit_core::{Buffer, Editor, Viewport};

    use crate::app::App;
    use crate::app::day_pane::{
        DayEditState, DayPane, DayPaneKind, handle_edit_key, key_to_editor_action,
    };
    use crate::ledger::WeekData;

    #[test]
    fn char_keys_map_to_insert_actions() {
        let event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let action = key_to_editor_action(event);
        assert!(action.is_some());
    }

    #[test]
    fn ctrl_s_does_not_insert_text() {
        let event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        let action = key_to_editor_action(event);
        assert!(action.is_none());
    }

    #[test]
    fn escape_exits_edit_mode() {
        let week_start = NaiveDate::from_ymd_opt(2026, 2, 9).expect("valid date");
        let week = WeekData::new(week_start);
        let mut app = App::new(week, "./data/test.ledger".into(), "./data".into());

        app.enter_day_edit_mode();
        assert_eq!(app.day_pane_kind(), DayPaneKind::Edit);

        let result = handle_edit_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(result.is_ok());
        assert_eq!(app.day_pane_kind(), DayPaneKind::View);
    }

    #[test]
    fn ctrl_s_with_parse_errors_keeps_edit_mode() {
        let week_start = NaiveDate::from_ymd_opt(2026, 2, 9).expect("valid date");
        let week = WeekData::new(week_start);
        let mut app = App::new(
            week,
            "./data/test-parse-error.ledger".into(),
            "./data".into(),
        );
        app.day_pane = DayPane::Edit(DayEditState {
            editor: Editor::new(Buffer::from_text("bad line"), Viewport::new(10, 40)),
            diagnostics: Vec::new(),
            diagnostics_scroll: crate::app::scroll_state::ScrollState {
                offset: 0,
                page_size: 5,
            },
        });

        let result = handle_edit_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        assert!(result.is_ok());
        assert_eq!(app.day_pane_kind(), DayPaneKind::Edit);
        assert_eq!(app.status, "Fix diagnostics before saving");
        let diagnostics = app.day_diagnostics_lines().unwrap_or(&[]);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn ctrl_s_with_valid_input_keeps_edit_mode() {
        let week_start = NaiveDate::from_ymd_opt(2026, 2, 9).expect("valid date");
        let week = WeekData::new(week_start);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let file_path = std::env::temp_dir().join(format!("time-ledger-day-pane-{stamp}.ledger"));

        let mut app = App::new(week, file_path.clone(), std::env::temp_dir());
        app.day_pane = DayPane::Edit(DayEditState {
            editor: Editor::new(Buffer::from_text("- Build @1h"), Viewport::new(10, 40)),
            diagnostics: Vec::new(),
            diagnostics_scroll: crate::app::scroll_state::ScrollState {
                offset: 0,
                page_size: 5,
            },
        });

        let result = handle_edit_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        assert!(result.is_ok());
        assert_eq!(app.day_pane_kind(), DayPaneKind::Edit);
        assert_eq!(app.status, "Saved");

        let saved = fs::read_to_string(&file_path).expect("saved ledger should exist");
        assert!(saved.contains("- Build @1h"));
        let _ = fs::remove_file(&file_path);
    }
}
