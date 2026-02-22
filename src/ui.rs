use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap};

use crate::app::App;
use crate::ledger::{Day, format_minutes, resolve_entry, week_dates};

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(frame.area());

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(root[0]);

    draw_week_table(frame, app, main[0]);
    if app.day_pane_is_editing() {
        draw_day_edit(frame, app, main[1]);
    } else {
        draw_day_detail(frame, app, main[1]);
    }

    let footer = build_footer(app);
    frame.render_widget(footer, root[1]);

    if app.showing_warnings() {
        draw_warnings_overlay(frame, app);
    }
}

fn build_footer(app: &App) -> Paragraph<'_> {
    let line = if app.showing_warnings() {
        warnings_footer_line(app)
    } else if app.day_pane_is_editing() {
        edit_footer_line(app)
    } else {
        main_footer_line(app)
    };
    Paragraph::new(line).block(Block::default().borders(Borders::TOP))
}

fn main_footer_line(app: &App) -> Line<'_> {
    Line::from(vec![
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("/"),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" quit  "),
        Span::styled("←/→", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" day  "),
        Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" task  "),
        Span::styled("s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" save  "),
        Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" edit day  "),
        Span::styled("w", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" show warnings  "),
        status_span(app),
    ])
}

fn edit_footer_line(app: &App) -> Line<'_> {
    Line::from(vec![
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" leave edit  "),
        Span::styled("Ctrl+s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" save  "),
        Span::styled("←/→/↑/↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" move cursor  "),
        Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" diagnostics scroll  "),
        Span::styled("Home/End", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" diagnostics jump  "),
        status_span(app),
    ])
}

fn warnings_footer_line(app: &App) -> Line<'_> {
    Line::from(vec![
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("/"),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" close  "),
        Span::styled("↑/↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" scroll  "),
        Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" page  "),
        Span::styled("Home/End", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" jump  "),
        Span::styled("w", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" close warnings  "),
        status_span(app),
    ])
}

fn status_span(app: &App) -> Span<'_> {
    if app.status.starts_with("Warnings: ") && !app.week.warnings.is_empty() {
        return Span::styled(app.status.as_str(), Style::default().fg(Color::Red));
    }

    Span::raw(app.status.as_str())
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

fn draw_day_edit(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let date = app.selected_date();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    let editor_area = layout[0];
    let diagnostics_area = layout[1];

    let editor_height = editor_area.height.saturating_sub(2) as usize;
    let editor_width = editor_area.width.saturating_sub(2) as usize;
    app.set_day_editor_viewport(editor_height.max(1), editor_width.max(1));

    let editor_lines = app
        .day_editor_visible_lines()
        .unwrap_or_default()
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();
    let editor_title = format!("Edit {} {}", date.format("%A"), date.format("%Y-%m-%d"));
    let editor_paragraph = Paragraph::new(editor_lines)
        .block(Block::default().borders(Borders::ALL).title(editor_title))
        .alignment(Alignment::Left);
    frame.render_widget(editor_paragraph, editor_area);

    if editor_area.width > 2
        && editor_area.height > 2
        && let Some((row, col)) = app.day_editor_cursor_screen_pos()
    {
        let visible_row = row
            .min(editor_height.saturating_sub(1))
            .min(u16::MAX as usize) as u16;
        let visible_col = col
            .min(editor_width.saturating_sub(1))
            .min(u16::MAX as usize) as u16;
        let x = editor_area.x.saturating_add(1).saturating_add(visible_col);
        let y = editor_area.y.saturating_add(1).saturating_add(visible_row);
        frame.set_cursor_position((x, y));
    }

    let diagnostics_lines = app.day_diagnostics_lines().unwrap_or(&[]);
    let mut diagnostic_lines: Vec<Line> =
        diagnostics_lines.iter().cloned().map(Line::from).collect();
    if diagnostic_lines.is_empty() {
        diagnostic_lines.push(Line::from("No issues"));
    }

    let diagnostics_height = diagnostics_area.height.saturating_sub(2) as usize;
    app.set_day_diagnostics_page_size(diagnostics_height.max(1));
    let diagnostics_scroll = app.day_diagnostics_scroll().min(u16::MAX as usize) as u16;

    let diagnostics_paragraph = Paragraph::new(diagnostic_lines)
        .block(Block::default().borders(Borders::ALL).title("Diagnostics"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .scroll((diagnostics_scroll, 0));
    frame.render_widget(diagnostics_paragraph, diagnostics_area);
}

fn build_day_lines(day: &Day, lines: &mut Vec<Line>) {
    if day.entries.is_empty() {
        lines.push(Line::from("No entries"));
        return;
    }

    for entry in &day.entries {
        let resolved = resolve_entry(entry);
        let mut line = format!("- {}", entry.name.trim());
        if let Some(time) = entry.time {
            line.push_str(" @");
            line.push_str(&time.format());
        }
        if entry.checked {
            line.push_str(" [x]");
        }
        lines.push(Line::from(line));

        if resolved.mismatch {
            if let (Some(parent), Some(sub_total)) = (entry.time, resolved.sub_total_minutes) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("mismatch:", Style::default().fg(Color::Red)),
                    Span::raw(format!(
                        " parent @{} vs sub-items @{}",
                        parent.format(),
                        format_minutes(sub_total)
                    )),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("mismatch", Style::default().fg(Color::Red)),
                ]));
            }
        }

        for sub in &entry.sub_items {
            let mut sub_line = format!("  - {}", sub.name.trim());
            if let Some(time) = sub.time {
                sub_line.push_str(" @");
                sub_line.push_str(&time.format());
            }
            lines.push(Line::from(sub_line));
        }
    }
}

fn draw_warnings_overlay(frame: &mut Frame<'_>, app: &mut App) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line> = Vec::new();
    if app.week.warnings.is_empty() {
        lines.push(Line::from("No warnings"));
    } else {
        for warning in &app.week.warnings {
            lines.push(Line::from(format!("* {warning}")));
        }
    }

    let content_height = area.height.saturating_sub(2) as usize;
    app.set_warnings_page_size(content_height.max(1));
    let scroll = app.warnings_scroll().min(u16::MAX as usize) as u16;

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Warnings"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);

    horizontal[1]
}
