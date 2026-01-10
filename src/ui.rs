use std::path::PathBuf;

use chrono::NaiveDate;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};

use crate::ledger::{
    Day, Totals, WeekData, compute_totals, format_minutes, resolve_entry, week_dates,
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
}

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(frame.area());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(root[0]);

    draw_week_table(frame, app, main[0]);
    draw_day_detail(frame, app, main[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  "),
        Span::styled("←/→", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" day  "),
        Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" task  "),
        Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" save  "),
        Span::raw(&app.status),
    ]))
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(footer, root[1]);
}

fn draw_week_table(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let week_dates = week_dates(app.week.week_start);
    let mut header_cells = Vec::with_capacity(9);
    header_cells.push(Cell::from("Task"));
    for date in &week_dates {
        let label = format!("{} {}", date.format("%a"), date.format("%m-%d"));
        header_cells.push(Cell::from(label));
    }
    header_cells.push(Cell::from("Week"));

    let header = Row::new(header_cells)
        .style(Style::default().fg(Color::Yellow))
        .height(1);

    let mut rows: Vec<Row> = Vec::new();
    for task in &app.tasks {
        let mut cells: Vec<Cell> = Vec::with_capacity(9);
        cells.push(Cell::from(task.name.clone()));
        for date in &week_dates {
            let minutes = app
                .totals
                .per_day_item
                .get(date)
                .and_then(|items| items.get(&task.key))
                .copied()
                .unwrap_or(0);
            cells.push(Cell::from(format_minutes(minutes)));
        }
        let week_minutes = app
            .totals
            .per_week_item
            .get(&task.key)
            .copied()
            .unwrap_or(0);
        cells.push(Cell::from(format_minutes(week_minutes)));
        rows.push(Row::new(cells));
    }

    let mut total_cells: Vec<Cell> = Vec::with_capacity(9);
    total_cells.push(Cell::from("TOTAL"));
    for date in &week_dates {
        let minutes = app.totals.per_day_total.get(date).copied().unwrap_or(0);
        total_cells.push(Cell::from(format_minutes(minutes)));
    }
    total_cells.push(Cell::from(format_minutes(app.totals.week_total)));
    rows.push(Row::new(total_cells).style(Style::default().add_modifier(Modifier::BOLD)));

    let widths = std::iter::once(Constraint::Length(20))
        .chain(std::iter::repeat_n(Constraint::Length(10), 7))
        .chain(std::iter::once(Constraint::Length(10)))
        .collect::<Vec<_>>();

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Weekly Overview"),
        )
        .column_spacing(1)
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = TableState::default();
    if !app.tasks.is_empty() {
        state.select(Some(app.selected_task));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_day_detail(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let date = app.selected_date();
    let title = format!("{} {}", date.format("%A"), date.format("%Y-%m-%d"));
    let mut lines: Vec<Line> = Vec::new();

    if let Some(day) = app.week.days.get(&date) {
        build_day_lines(day, &mut lines);
    } else {
        lines.push(Line::from("No entries"));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn build_day_lines(day: &Day, lines: &mut Vec<Line>) {
    if day.entries.is_empty() {
        lines.push(Line::from("No entries"));
        return;
    }

    for entry in &day.entries {
        let resolved = resolve_entry(entry);
        let time_text = match (
            entry.time,
            resolved.sub_total_minutes,
            resolved.sub_complete,
        ) {
            (Some(time), _, _) => time.format(),
            (None, Some(minutes), true) => format!("{} (computed)", format_minutes(minutes)),
            (None, _, _) => "—".to_string(),
        };

        let check = if entry.checked { "[x]" } else { "[ ]" };
        let mut spans = vec![
            Span::raw(check),
            Span::raw(" "),
            Span::raw(entry.name.clone()),
            Span::raw(" @"),
            Span::raw(time_text),
        ];

        if resolved.mismatch {
            spans.push(Span::raw(" "));
            spans.push(Span::styled("mismatch", Style::default().fg(Color::Red)));
        }

        lines.push(Line::from(spans));

        for sub in &entry.sub_items {
            let mut sub_line = format!("  - {}", sub.name);
            if let Some(time) = sub.time {
                sub_line.push_str(" @");
                sub_line.push_str(&time.format());
            }
            lines.push(Line::from(sub_line));
        }
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
