//! UI rendering for Fortify TUI

#![allow(dead_code)]

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Dialog, View};

mod dialogs;
mod home;
mod logs;
mod running;
mod settings;
mod wizard;

/// Main draw function
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Split into left panel and right log panel
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let left_panel = main_layout[0];
    let right_panel = main_layout[1];

    // Draw left panel based on current view
    match &app.view {
        View::Home => home::draw(frame, app, left_panel),
        View::DeployWizard { step } => wizard::draw(frame, app, left_panel, *step),
        View::ViewSettings { tab, field_index } => {
            settings::draw(frame, app, left_panel, *tab, *field_index, true)
        }
        View::Settings { tab, field_index } => {
            settings::draw(frame, app, left_panel, *tab, *field_index, false)
        }
        View::Running => running::draw(frame, app, left_panel),
        View::ResumeSelect => draw_resume_select(frame, app, left_panel),
        View::JoinNetwork => draw_join_network(frame, app, left_panel),
    }

    // Draw right panel (logs) - always visible
    logs::draw(frame, app, right_panel);

    // Draw dialog overlay if active
    if !matches!(app.dialog, Dialog::None) {
        dialogs::draw(frame, app);
    }
}

/// Draw resume selection screen
fn draw_resume_select(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Resume Deployment ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.existing_deployments.is_empty() {
        let text = Paragraph::new("No existing deployments found.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(text, inner);
        return;
    }

    let items: Vec<ListItem> = app
        .existing_deployments
        .iter()
        .enumerate()
        .map(|(i, (id, path))| {
            let style = if i == app.resume_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("  {} - {}", id, path.display())).style(style)
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, inner);
}

/// Draw join network screen
fn draw_join_network(frame: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .title(" Join Community Network ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Community Network (Phase 5)",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Join a distributed network of Fortify nodes"),
        Line::from("  for enhanced protection and load balancing."),
        Line::from(""),
        Line::from(Span::styled(
            "  Coming Soon",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from("  Press ESC to go back"),
    ];

    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}

/// Draw system status screen
#[allow(dead_code)]
fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" System Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Deployment: "),
            if app.deployment.is_running() {
                Span::styled("● Running", Style::default().fg(Color::Green))
            } else {
                Span::styled("○ Stopped", Style::default().fg(Color::DarkGray))
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Log Buffer: "),
            Span::styled(
                format!("{} entries", app.logs.len()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Log Filter: "),
            Span::styled(
                format!("{:?}+", app.log_filter),
                Style::default().fg(app.log_filter.color()),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Logs Paused: "),
            Span::raw(if app.logs_paused { "Yes" } else { "No" }),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ESC to go back",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let para = Paragraph::new(status_lines);
    frame.render_widget(para, inner);
}

/// Draw header bar
pub fn draw_header(title: &str) -> Paragraph<'static> {
    Paragraph::new(title.to_string())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
}

/// Draw a labeled field
pub fn draw_field(label: &str, value: &str, selected: bool) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Line::from(vec![
        Span::styled(format!("  {}: ", label), style),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

/// Draw keyboard hints
pub fn draw_hints(hints: &[(&str, &str)]) -> Paragraph<'static> {
    let spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!("[{}]", key), Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {} ", desc)),
            ]
        })
        .collect();

    Paragraph::new(Line::from(spans))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
}
