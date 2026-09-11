use crate::app::{App, Mode};
use crate::status::relative_time;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Wrap};
use std::collections::HashMap;

const ORANGE: Color = Color::Rgb(239, 119, 62);
const PINK: Color = Color::Rgb(218, 3, 65);
const RED: Color = Color::Rgb(239, 68, 68);
const GREEN: Color = Color::Rgb(74, 222, 128);
const TEAL: Color = Color::Rgb(45, 212, 191);
const MUTED: Color = Color::Rgb(120, 120, 140);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_table(frame, app, chunks[0]);
    draw_status(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);

    match app.mode {
        Mode::AddRoot => draw_input_popup(
            frame,
            area,
            "Add root directory (e.g. ~/code)",
            &app.input_buffer,
        ),
        Mode::Detail => draw_detail_popup(frame, area, app),
        Mode::Help => draw_help_popup(frame, area),
        _ => {}
    }
}

fn draw_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        "",
        "Repo",
        "Branch",
        "Δ",
        "Dirty",
        "Stash",
        "Last commit",
    ])
    .style(Style::default().fg(ORANGE).add_modifier(Modifier::BOLD));

    // Two different checkouts can share a directory name (e.g. one root's
    // `cyberterm` and another's) — disambiguate those with a parent-dir
    // prefix instead of showing identical, unclickable-looking rows.
    let mut name_counts: HashMap<&str, usize> = HashMap::new();
    for r in &app.repos {
        *name_counts.entry(r.name.as_str()).or_insert(0) += 1;
    }

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .map(|&i| {
            let r = &app.repos[i];
            let display_name = if name_counts.get(r.name.as_str()).copied().unwrap_or(0) > 1 {
                let parent = r
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                format!("{parent}/{}", r.name)
            } else {
                r.name.clone()
            };
            let flag = if r.state.label() != "" {
                Span::styled(
                    r.state.label(),
                    Style::default().fg(RED).add_modifier(Modifier::BOLD),
                )
            } else if r.needs_attention() {
                Span::styled("●", Style::default().fg(RED))
            } else {
                Span::styled("●", Style::default().fg(GREEN))
            };

            let branch = match (&r.branch, r.detached) {
                (Some(b), true) => format!("@{b}"),
                (Some(b), false) => b.clone(),
                (None, _) => "(no commits)".to_string(),
            };

            let delta = if !r.has_upstream {
                Span::styled("no upstream", Style::default().fg(MUTED))
            } else if r.ahead == 0 && r.behind == 0 {
                Span::styled("up to date", Style::default().fg(GREEN))
            } else {
                Span::styled(
                    format!("↑{} ↓{}", r.ahead, r.behind),
                    Style::default().fg(if r.behind > 0 { RED } else { ORANGE }),
                )
            };

            let dirty = if r.is_dirty() {
                Span::styled(
                    format!("+{} ~{} ?{}", r.staged, r.unstaged, r.untracked),
                    Style::default().fg(RED),
                )
            } else {
                Span::styled("clean", Style::default().fg(GREEN))
            };

            let stash = if r.stashes > 0 {
                Span::styled(r.stashes.to_string(), Style::default().fg(ORANGE))
            } else {
                Span::styled("-", Style::default().fg(MUTED))
            };

            let last_commit = r
                .last_commit_time
                .map(relative_time)
                .unwrap_or_else(|| "-".to_string());

            Row::new(vec![
                Line::from(flag),
                Line::from(Span::styled(
                    display_name,
                    Style::default().fg(Color::White),
                )),
                Line::from(Span::styled(branch, Style::default().fg(TEAL))),
                Line::from(delta),
                Line::from(dirty),
                Line::from(stash),
                Line::from(Span::styled(last_commit, Style::default().fg(MUTED))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(22),
        Constraint::Percentage(16),
        Constraint::Percentage(14),
        Constraint::Percentage(16),
        Constraint::Length(6),
        Constraint::Min(12),
    ];

    let title = format!(" cyberfleet — {} repos ", app.repos.len());
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(PINK)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ORANGE))
                .title(Span::styled(title, Style::default().fg(ORANGE))),
        );

    let mut state = TableState::default();
    state.select(if app.filtered.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Filter => format!("filter: {}_", app.filter_text),
        _ => app.status.clone().unwrap_or_default(),
    };
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(TEAL)), area);
}

fn draw_footer(frame: &mut Frame, _app: &App, area: Rect) {
    let text = "j/k nav  o open term  e editor  f fetch  F fetch all  d status  a add root  i ignore  / filter  r rescan  ? help  q quit";
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(MUTED)), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_input_popup(frame: &mut Frame, area: Rect, title: &str, buffer: &str) {
    let popup = centered_rect(60, 15, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(ORANGE),
        ));
    frame.render_widget(Paragraph::new(format!("{buffer}_")).block(block), popup);
}

fn draw_detail_popup(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup);
    let title = app
        .selected_repo()
        .map(|r| format!(" git status — {} (Esc to close) ", r.name))
        .unwrap_or_else(|| " git status ".to_string());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(title, Style::default().fg(ORANGE)));
    frame.render_widget(
        Paragraph::new(app.detail_text.clone())
            .wrap(Wrap { trim: false })
            .block(block),
        popup,
    );
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 60, area);
    frame.render_widget(Clear, popup);
    let lines = [
        "j/k, ↑/↓   move",
        "/          filter by name",
        "o, Enter   open a terminal at the repo",
        "e          open $EDITOR (or $VISUAL) there",
        "f          fetch the selected repo",
        "F          fetch every repo",
        "d          show `git status` for the selected repo",
        "a          add a root directory to scan",
        "i          ignore the selected repo (by name)",
        "r          rescan all configured roots",
        "?          toggle this help",
        "q, Esc     quit (Esc closes a popup first)",
    ]
    .join("\n");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            " cyberfleet — keys ",
            Style::default().fg(ORANGE),
        ));
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}
