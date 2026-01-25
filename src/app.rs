use std::error::Error;
use std::path::PathBuf;

use chrono::{Duration, Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ledger::{
    Totals, WeekData, apply_computed_times, compute_totals, load_week, load_week_if_exists,
    save_week, week_dates, week_file_name, week_start_for,
};

#[derive(Debug, Clone)]
pub struct TaskDisplay {
    pub key: String,
    pub name: String,
}

enum Overlay {
    Warnings(WarningsOverlayState),
}

pub struct App {
    pub week: WeekData,
    pub file_path: PathBuf,
    pub tasks: Vec<TaskDisplay>,
    pub selected_day: usize,
    pub selected_task: usize,
    pub totals: Totals,
    pub status: String,
    overlay_type: Option<Overlay>,
}

struct WarningsOverlayState {
    scroll: usize,
    page_size: usize,
}

impl App {
    pub fn new(week: WeekData, file_path: PathBuf) -> Self {
        let totals = compute_totals(&week);
        let tasks = build_tasks(&totals);
        let status = format!("Warnings: {}", week.warnings.len());
        Self {
            week,
            file_path,
            tasks,
            selected_day: 0,
            selected_task: 0,
            totals,
            status,
            overlay_type: None,
        }
    }

    pub fn refresh(&mut self) {
        self.totals = compute_totals(&self.week);
        self.tasks = build_tasks(&self.totals);
        if self.selected_task >= self.tasks.len() {
            self.selected_task = self.tasks.len().saturating_sub(1);
        }
        self.status = format!("Warnings: {}", self.week.warnings.len());
        let line_count = self.warnings_line_count();
        if let Some(state) = self.warnings_overlay_state_mut() {
            state.clamp_scroll(line_count);
        }
    }

    pub fn selected_date(&self) -> NaiveDate {
        let dates = week_dates(self.week.week_start);
        dates
            .get(self.selected_day)
            .copied()
            .unwrap_or(self.week.week_start)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
        if self.showing_warnings() {
            let line_count = self.warnings_line_count();
            // Modal overlay: consume navigation keys so the main UI doesn't move underneath.
            match key {
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::NONE,
                    ..
                } => {
                    self.overlay_type = None;
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }
                | KeyEvent {
                    code: KeyCode::Esc, ..
                } => {
                    self.overlay_type = None;
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        state.scroll_by(-1, line_count);
                    }
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        state.scroll_by(1, line_count);
                    }
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        let delta = state.page_size.max(1) as i32;
                        state.scroll_by(-delta, line_count);
                    }
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        let delta = state.page_size.max(1) as i32;
                        state.scroll_by(delta, line_count);
                    }
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::Home,
                    ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        state.scroll = 0;
                    }
                    return Ok(false);
                }
                KeyEvent {
                    code: KeyCode::End, ..
                } => {
                    if let Some(state) = self.warnings_overlay_state_mut() {
                        state.scroll = state.max_scroll(line_count);
                    }
                    return Ok(false);
                }
                // Ignore all other keys while the overlay is open.
                _ => return Ok(false),
            }
        }

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
                apply_computed_times(&mut self.week)?;
                save_week(&self.file_path, &self.week)?;
                self.refresh();
                self.status = "Saved".to_string();
            }
            KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.overlay_type = match self.overlay_type.take() {
                    None => Some(Overlay::Warnings(WarningsOverlayState::new())),
                    Some(Overlay::Warnings(_)) => None,
                }
            }
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => {
                if self.selected_day == 0 {
                    self.shift_week(-1)?;
                } else {
                    self.selected_day = self.selected_day.saturating_sub(1);
                }
            }
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => {
                if self.selected_day == 6 {
                    self.shift_week(1)?;
                } else {
                    self.selected_day = (self.selected_day + 1).min(6);
                }
            }
            KeyEvent {
                code: KeyCode::Up, ..
            } => {
                self.selected_task = self.selected_task.saturating_sub(1);
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                if !self.tasks.is_empty() {
                    self.selected_task = (self.selected_task + 1).min(self.tasks.len() - 1);
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn shift_week(&mut self, direction: i64) -> Result<(), Box<dyn Error>> {
        let week_start = week_start_for(Local::now().date_naive());
        let candidate_week = self.week.week_start + Duration::days(7 * direction);
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

        self.week = week;
        self.file_path = file_path;
        self.refresh();
        self.selected_day = if direction < 0 { 6 } else { 0 };

        Ok(())
    }

    pub fn showing_warnings(&self) -> bool {
        matches!(self.overlay_type, Some(Overlay::Warnings(_)))
    }

    pub fn set_warnings_page_size(&mut self, page_size: usize) {
        let line_count = self.warnings_line_count();
        if let Some(state) = self.warnings_overlay_state_mut() {
            state.set_page_size(page_size, line_count);
        }
    }

    pub fn warnings_scroll(&self) -> usize {
        self.warnings_overlay_state()
            .map(|state| state.scroll)
            .unwrap_or(0)
    }

    fn warnings_line_count(&self) -> usize {
        if self.week.warnings.is_empty() {
            1
        } else {
            self.week.warnings.len()
        }
    }

    fn warnings_overlay_state(&self) -> Option<&WarningsOverlayState> {
        match &self.overlay_type {
            Some(Overlay::Warnings(state)) => Some(state),
            None => None,
        }
    }

    fn warnings_overlay_state_mut(&mut self) -> Option<&mut WarningsOverlayState> {
        match &mut self.overlay_type {
            Some(Overlay::Warnings(state)) => Some(state),
            None => None,
        }
    }
}

impl WarningsOverlayState {
    fn new() -> Self {
        Self {
            scroll: 0,
            page_size: 5,
        }
    }

    fn set_page_size(&mut self, page_size: usize, total_lines: usize) {
        self.page_size = page_size.max(1);
        self.clamp_scroll(total_lines);
    }

    fn clamp_scroll(&mut self, total_lines: usize) {
        let max_scroll = self.max_scroll(total_lines);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }

    fn max_scroll(&self, total_lines: usize) -> usize {
        total_lines.saturating_sub(self.page_size.max(1))
    }

    fn scroll_by(&mut self, delta: i32, total_lines: usize) {
        if delta < 0 {
            let amount = delta.unsigned_abs() as usize;
            self.scroll = self.scroll.saturating_sub(amount);
        } else {
            self.scroll = self.scroll.saturating_add(delta as usize);
        }
        self.clamp_scroll(total_lines);
    }
}

fn build_tasks(totals: &Totals) -> Vec<TaskDisplay> {
    totals
        .display_names
        .iter()
        .map(|(key, name)| TaskDisplay {
            key: key.clone(),
            name: name.clone(),
        })
        .collect()
}
