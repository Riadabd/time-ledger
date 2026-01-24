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

pub struct App {
    pub week: WeekData,
    pub file_path: PathBuf,
    pub tasks: Vec<TaskDisplay>,
    pub selected_day: usize,
    pub selected_task: usize,
    pub totals: Totals,
    pub status: String,
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
        }
    }

    pub fn refresh(&mut self) {
        self.totals = compute_totals(&self.week);
        self.tasks = build_tasks(&self.totals);
        if self.selected_task >= self.tasks.len() {
            self.selected_task = self.tasks.len().saturating_sub(1);
        }
        self.status = format!("Warnings: {}", self.week.warnings.len());
    }

    pub fn selected_date(&self) -> NaiveDate {
        let dates = week_dates(self.week.week_start);
        dates
            .get(self.selected_day)
            .copied()
            .unwrap_or(self.week.week_start)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool, Box<dyn Error>> {
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
