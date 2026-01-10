use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use chrono::{Datelike, Duration, NaiveDate};

use crate::time_amount::{TimeAmount, TimeError};

#[derive(Debug, Clone)]
pub struct SubItem {
    pub name: String,
    pub time: Option<TimeAmount>,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub time: Option<TimeAmount>,
    pub checked: bool,
    pub sub_items: Vec<SubItem>,
}

#[derive(Debug, Clone)]
pub struct Day {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct WeekData {
    pub week_start: NaiveDate,
    pub days: BTreeMap<NaiveDate, Day>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum LedgerError {
    Io(std::io::Error),
    Time(TimeError),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::Io(err) => write!(f, "io error: {err}"),
            LedgerError::Time(err) => write!(f, "time error: {err}"),
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<std::io::Error> for LedgerError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<TimeError> for LedgerError {
    fn from(err: TimeError) -> Self {
        Self::Time(err)
    }
}

#[derive(Debug, Clone)]
pub struct Totals {
    pub per_day_item: BTreeMap<NaiveDate, BTreeMap<String, i64>>,
    pub per_day_total: BTreeMap<NaiveDate, i64>,
    pub per_week_item: BTreeMap<String, i64>,
    pub week_total: i64,
    pub display_names: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct EntryResolved {
    pub effective_minutes: Option<i64>,
    pub sub_total_minutes: Option<i64>,
    pub sub_complete: bool,
    pub mismatch: bool,
}

pub fn load_week(path: &Path, default_week_start: NaiveDate) -> Result<WeekData, LedgerError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(parse_ledger(&content, default_week_start)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(WeekData::new(default_week_start))
        }
        Err(err) => Err(LedgerError::Io(err)),
    }
}

pub fn save_week(path: &Path, week: &WeekData) -> Result<(), LedgerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let output = render_week(week)?;
    fs::write(path, output)?;
    Ok(())
}

impl WeekData {
    pub fn new(week_start: NaiveDate) -> Self {
        Self {
            week_start,
            days: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn day_mut(&mut self, date: NaiveDate) -> &mut Day {
        self.days.entry(date).or_insert_with(|| Day {
            entries: Vec::new(),
        })
    }
}

pub fn week_start_for(date: NaiveDate) -> NaiveDate {
    let days_from_monday = date.weekday().num_days_from_monday() as i64;
    date - Duration::days(days_from_monday)
}

pub fn week_file_name(date: NaiveDate) -> String {
    let iso = date.iso_week();
    format!("{:04}-W{:02}.ledger", iso.year(), iso.week())
}

pub fn parse_ledger(content: &str, default_week_start: NaiveDate) -> WeekData {
    let mut week = WeekData::new(default_week_start);
    let mut current_day: Option<NaiveDate> = None;
    let mut last_entry_index: Option<usize> = None;

    for (line_idx, raw_line) in content.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = raw_line.trim_end();
        if line.is_empty() {
            continue;
        }

        if let Some(date) = parse_week_header(line) {
            week.week_start = date;
            continue;
        }

        if let Some(date) = parse_day_header(line) {
            current_day = Some(date);
            week.day_mut(date);
            last_entry_index = None;
            continue;
        }

        if line.starts_with(";;") {
            continue;
        }

        let Some(parsed) = parse_entry_line(line) else {
            week.warnings
                .push(format!("Unrecognized line {line_no}: {line}"));
            continue;
        };

        if let Some(error) = parsed.time_error {
            week.warnings.push(format!("Line {line_no}: {error}"));
        }

        let Some(day_date) = current_day else {
            week.warnings
                .push(format!("Line {line_no}: entry before any day header"));
            continue;
        };

        let day = week.day_mut(day_date);
        if parsed.is_sub_item {
            let Some(parent_index) = last_entry_index else {
                week.warnings
                    .push(format!("Line {line_no}: sub-item without parent"));
                continue;
            };
            if let Some(parent) = day.entries.get_mut(parent_index) {
                parent.sub_items.push(SubItem {
                    name: parsed.name,
                    time: parsed.time,
                });
            }
        } else {
            day.entries.push(Entry {
                name: parsed.name,
                time: parsed.time,
                checked: parsed.checked,
                sub_items: Vec::new(),
            });
            last_entry_index = Some(day.entries.len().saturating_sub(1));
        }
    }

    week
}

pub fn apply_computed_times(week: &mut WeekData) -> Result<(), LedgerError> {
    for day in week.days.values_mut() {
        for entry in &mut day.entries {
            let resolved = resolve_entry(entry);
            if entry.time.is_none()
                && resolved.sub_complete
                && let Some(minutes) = resolved.sub_total_minutes
            {
                entry.time = Some(TimeAmount::from_minutes(minutes)?);
            }
        }
    }
    Ok(())
}

pub fn compute_totals(week: &WeekData) -> Totals {
    let mut per_day_item: BTreeMap<NaiveDate, BTreeMap<String, i64>> = BTreeMap::new();
    let mut per_day_total: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    let mut per_week_item: BTreeMap<String, i64> = BTreeMap::new();
    let mut display_names: BTreeMap<String, String> = BTreeMap::new();
    let mut week_total = 0_i64;

    for (date, day) in &week.days {
        let mut day_items: BTreeMap<String, i64> = BTreeMap::new();
        let mut day_total = 0_i64;

        for entry in &day.entries {
            let key = task_key(&entry.name);
            display_names
                .entry(key.clone())
                .or_insert(entry.name.clone());
            let resolved = resolve_entry(entry);

            if let Some(minutes) = resolved.effective_minutes {
                *day_items.entry(key.clone()).or_insert(0) += minutes;
                *per_week_item.entry(key).or_insert(0) += minutes;
                day_total += minutes;
            }
        }

        per_day_item.insert(*date, day_items);
        per_day_total.insert(*date, day_total);
        week_total += day_total;
    }

    Totals {
        per_day_item,
        per_day_total,
        per_week_item,
        week_total,
        display_names,
    }
}

pub fn render_week(week: &WeekData) -> Result<String, LedgerError> {
    let mut week = week.clone();
    apply_computed_times(&mut week)?;
    let totals = compute_totals(&week);
    let mut lines: Vec<String> = Vec::new();

    lines.push(format!("# Week {}", week.week_start.format("%Y-%m-%d")));
    lines.push(String::new());

    for day_index in 0..7 {
        let date = week.week_start + Duration::days(day_index as i64);
        let weekday = date.format("%a");
        lines.push(format!("## {} {}", date.format("%Y-%m-%d"), weekday));

        if let Some(day) = week.days.get(&date) {
            for entry in &day.entries {
                lines.push(render_entry_line(entry));
                for sub in &entry.sub_items {
                    lines.push(render_sub_item_line(sub));
                }
            }
        }

        if let Some(day_items) = totals.per_day_item.get(&date) {
            for (key, minutes) in day_items {
                let display_name = totals
                    .display_names
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| key.clone());
                let time = TimeAmount::from_minutes(*minutes)?.format();
                lines.push(format!(";; item-total {} @{}", display_name, time));
            }
        }

        let day_total = totals.per_day_total.get(&date).copied().unwrap_or(0);
        let total_time = TimeAmount::from_minutes(day_total)?.format();
        lines.push(format!(";; day-total @{}", total_time));
        lines.push(String::new());
    }

    lines.push(format!(
        ";; week-total @{}",
        TimeAmount::from_minutes(totals.week_total)?.format()
    ));

    for (key, minutes) in &totals.per_week_item {
        let display_name = totals
            .display_names
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.clone());
        let time = TimeAmount::from_minutes(*minutes)?.format();
        lines.push(format!(";; week-item-total {} @{}", display_name, time));
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn render_entry_line(entry: &Entry) -> String {
    let mut line = format!("- {}", entry.name.trim());
    if let Some(time) = entry.time {
        line.push_str(" @");
        line.push_str(&time.format());
    }
    if entry.checked {
        line.push_str(" [x]");
    }
    line
}

fn render_sub_item_line(sub: &SubItem) -> String {
    let mut line = format!("  - {}", sub.name.trim());
    if let Some(time) = sub.time {
        line.push_str(" @");
        line.push_str(&time.format());
    }
    line
}

fn parse_week_header(line: &str) -> Option<NaiveDate> {
    let rest = line.strip_prefix("# Week ")?;
    let date_str = rest.split_whitespace().next()?;
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

fn parse_day_header(line: &str) -> Option<NaiveDate> {
    let rest = line.strip_prefix("## ")?;
    let date_str = rest.split_whitespace().next()?;
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

struct ParsedLine {
    name: String,
    time: Option<TimeAmount>,
    checked: bool,
    is_sub_item: bool,
    time_error: Option<String>,
}

fn parse_entry_line(line: &str) -> Option<ParsedLine> {
    let (is_sub_item, content) = if let Some(rest) = line.strip_prefix("  - ") {
        (true, rest)
    } else if let Some(rest) = line.strip_prefix("- ") {
        (false, rest)
    } else {
        return None;
    };

    let mut working = content.trim_end();
    let mut checked = false;
    if working.ends_with("[x]") {
        checked = true;
        working = working[..working.len().saturating_sub(3)].trim_end();
    }

    let mut time = None;
    let mut time_error = None;
    let mut name = working.trim();

    if let Some(at_pos) = working.rfind('@') {
        let before = working[..at_pos].trim_end();
        let after = working[at_pos + 1..].trim();
        name = before.trim();
        if !after.is_empty() {
            match TimeAmount::parse(after) {
                Ok(parsed) => time = Some(parsed),
                Err(err) => {
                    time_error = Some(format!("Invalid time '{after}': {err:?}"));
                }
            }
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(ParsedLine {
        name: name.to_string(),
        time,
        checked,
        is_sub_item,
        time_error,
    })
}

fn task_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

pub fn resolve_entry(entry: &Entry) -> EntryResolved {
    let mut sub_complete = !entry.sub_items.is_empty();
    let mut sub_total = 0_i64;

    for sub in &entry.sub_items {
        if let Some(time) = sub.time {
            sub_total += time.minutes();
        } else {
            sub_complete = false;
        }
    }

    let sub_total_minutes = if sub_complete && !entry.sub_items.is_empty() {
        Some(sub_total)
    } else {
        None
    };

    let parent_minutes = entry.time.map(|t| t.minutes());
    let mismatch = parent_minutes.is_some()
        && sub_total_minutes.is_some()
        && parent_minutes != sub_total_minutes;

    let effective_minutes = if let Some(parent) = parent_minutes {
        Some(parent)
    } else {
        sub_total_minutes
    };

    EntryResolved {
        effective_minutes,
        sub_total_minutes,
        sub_complete: sub_total_minutes.is_some(),
        mismatch,
    }
}

pub fn format_minutes(minutes: i64) -> String {
    match TimeAmount::from_minutes(minutes) {
        Ok(amount) => amount.format(),
        Err(_) => "0m".to_string(),
    }
}

pub fn week_dates(week_start: NaiveDate) -> Vec<NaiveDate> {
    (0..7)
        .map(|offset| week_start + Duration::days(offset as i64))
        .collect()
}

pub fn empty_week(week_start: NaiveDate) -> WeekData {
    let mut week = WeekData::new(week_start);
    for date in week_dates(week_start) {
        week.day_mut(date);
    }
    week
}
