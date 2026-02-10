use std::error::Error;
use std::path::PathBuf;

use chrono::{Duration, Local, NaiveDate};
use crossterm::event::KeyEvent;

mod main_screen;
mod warnings_screen;

use crate::ledger::{
    Totals, WeekData, compute_totals, load_week, load_week_if_exists, week_dates, week_file_path,
    week_start_for,
};

#[derive(Debug, Clone)]
pub struct TaskDisplay {
    pub key: String,
    pub name: String,
}

enum Screen {
    Main,
    Warnings(WarningsOverlayState),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScreenKind {
    Main,
    Warnings,
}

pub struct App {
    pub week: WeekData,
    pub file_path: PathBuf,
    ledger_dir: PathBuf,
    pub tasks: Vec<TaskDisplay>,
    pub selected_day: usize,
    pub selected_task: usize,
    pub totals: Totals,
    pub status: String,
    screen: Screen,
}

struct WarningsOverlayState {
    scroll: usize,
    page_size: usize,
}

impl App {
    pub fn new(week: WeekData, file_path: PathBuf, ledger_dir: PathBuf) -> Self {
        let totals = compute_totals(&week);
        let tasks = build_tasks(&totals);
        let status = format!("Warnings: {}", week.warnings.len());
        Self {
            week,
            file_path,
            ledger_dir,
            tasks,
            selected_day: 0,
            selected_task: 0,
            totals,
            status,
            screen: Screen::Main,
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
        match self.active_screen() {
            ScreenKind::Main => main_screen::handle_key(self, key),
            ScreenKind::Warnings => warnings_screen::handle_key(self, key),
        }
    }

    fn active_screen(&self) -> ScreenKind {
        match &self.screen {
            Screen::Main => ScreenKind::Main,
            Screen::Warnings(_) => ScreenKind::Warnings,
        }
    }

    fn shift_week(&mut self, direction: i64) -> Result<(), Box<dyn Error>> {
        let week_start = week_start_for(Local::now().date_naive());
        let candidate_week = self.week.week_start + Duration::days(7 * direction);
        let file_path = week_file_path(&self.ledger_dir, candidate_week);

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
        matches!(self.screen, Screen::Warnings(_))
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
        match &self.screen {
            Screen::Warnings(state) => Some(state),
            Screen::Main => None,
        }
    }

    fn warnings_overlay_state_mut(&mut self) -> Option<&mut WarningsOverlayState> {
        match &mut self.screen {
            Screen::Warnings(state) => Some(state),
            Screen::Main => None,
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
