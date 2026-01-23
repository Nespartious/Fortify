//! Dialog rendering

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, DependencyCheckPhase, DependencyState, Dialog};

/// Draw dialog overlay
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Calculate centered dialog area - wider for input dialogs (URLs can be long)
    let base_width = match &app.dialog {
        Dialog::Input { .. } => 90, // Wider for URL input
        _ => 70,
    };
    let dialog_width = base_width.min(area.width.saturating_sub(4));
    let dialog_height = match &app.dialog {
        Dialog::Confirm { .. } => 8,
        Dialog::ApplyChanges {
            hot_reload,
            restart_required,
        } => {
            let total_changes = hot_reload.len() + restart_required.len();
            // Extra space for section headers if both types present
            let extra = if !hot_reload.is_empty() && !restart_required.is_empty() {
                4
            } else {
                2
            };
            (10 + total_changes + extra).min(24) as u16
        }
        Dialog::Input { .. } => 7,
        Dialog::Error { .. } => 8,
        Dialog::Info { .. } => 8,
        Dialog::DependencyCheck { statuses, .. } => (6 + statuses.len() * 2).min(24) as u16,
        Dialog::None => return,
    };

    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect {
        x,
        y,
        width: dialog_width,
        height: dialog_height,
    };

    // Draw semi-transparent overlay
    let _overlay = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, dialog_area);

    // Draw dialog content
    match &app.dialog {
        Dialog::Confirm { title, message, .. } => {
            draw_confirm(frame, dialog_area, title, message);
        }
        Dialog::ApplyChanges {
            hot_reload,
            restart_required,
        } => {
            draw_apply_changes(frame, dialog_area, hot_reload, restart_required);
        }
        Dialog::Input { title, value, .. } => {
            draw_input(frame, dialog_area, title, value);
        }
        Dialog::Error { message } => {
            draw_error(frame, dialog_area, message);
        }
        Dialog::Info { title, message } => {
            draw_info(frame, dialog_area, title, message);
        }
        Dialog::DependencyCheck {
            statuses,
            phase,
            completed_at,
        } => {
            draw_dependency_check(frame, dialog_area, statuses, phase, completed_at);
        }
        Dialog::None => {}
    }
}

fn draw_confirm(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(Color::Green)),
            Span::raw(" Yes    "),
            Span::styled("[N]", Style::default().fg(Color::Red)),
            Span::raw(" No"),
        ]),
    ];

    let para = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn draw_apply_changes(
    frame: &mut Frame,
    area: Rect,
    hot_reload: &[String],
    restart_required: &[String],
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .border_type(BorderType::Rounded)
        .title(" Apply Changes? ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut content = vec![Line::from("")];

    // Hot-reload changes (can be applied immediately)
    if !hot_reload.is_empty() {
        content.push(Line::from(Span::styled(
            "✓ Can apply immediately:",
            Style::default().fg(Color::Green),
        )));
        for change in hot_reload.iter().take(3) {
            content.push(Line::from(Span::styled(
                format!("  • {}", change),
                Style::default().fg(Color::Green),
            )));
        }
        if hot_reload.len() > 3 {
            content.push(Line::from(Span::styled(
                format!("  ... and {} more", hot_reload.len() - 3),
                Style::default().fg(Color::DarkGray),
            )));
        }
        content.push(Line::from(""));
    }

    // Restart-required changes
    if !restart_required.is_empty() {
        content.push(Line::from(Span::styled(
            "⚠ Requires restart:",
            Style::default().fg(Color::Yellow),
        )));
        for change in restart_required.iter().take(3) {
            content.push(Line::from(Span::styled(
                format!("  • {}", change),
                Style::default().fg(Color::Yellow),
            )));
        }
        if restart_required.len() > 3 {
            content.push(Line::from(Span::styled(
                format!("  ... and {} more", restart_required.len() - 3),
                Style::default().fg(Color::DarkGray),
            )));
        }
        content.push(Line::from(""));
    }

    // Options depend on what types of changes we have
    content.push(Line::from(""));
    if !hot_reload.is_empty() && !restart_required.is_empty() {
        // Both types - offer all options
        content.push(Line::from(vec![
            Span::styled("[A]", Style::default().fg(Color::Green)),
            Span::raw(" Apply hot-reload only"),
        ]));
        content.push(Line::from(vec![
            Span::styled("[R]", Style::default().fg(Color::Yellow)),
            Span::raw(" Restart to apply all"),
        ]));
        content.push(Line::from(vec![
            Span::styled("[C]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel (discard changes)"),
        ]));
    } else if !hot_reload.is_empty() {
        // Only hot-reload changes
        content.push(Line::from(vec![
            Span::styled("[A]", Style::default().fg(Color::Green)),
            Span::raw(" Apply changes    "),
            Span::styled("[C]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel"),
        ]));
    } else {
        // Only restart-required changes
        content.push(Line::from(vec![
            Span::styled("[R]", Style::default().fg(Color::Yellow)),
            Span::raw(" Restart to apply    "),
            Span::styled("[C]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel"),
        ]));
    }

    let para = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn draw_input(frame: &mut Frame, area: Rect, title: &str, value: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Input field area
    let input_area = Rect {
        x: inner.x + 2,
        y: inner.y + 2,
        width: inner.width.saturating_sub(4),
        height: 1,
    };

    // Draw input background
    let input_bg = Block::default().style(Style::default().bg(Color::DarkGray));
    frame.render_widget(input_bg, input_area);

    // Draw input value with cursor
    let display_value = format!("{}█", value);
    let input =
        Paragraph::new(display_value).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(input, input_area);

    // Draw hints
    let hints = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Green)),
        Span::raw(" Save    "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Cancel"),
    ]);

    let hint_area = Rect {
        x: inner.x,
        y: inner.y + 4,
        width: inner.width,
        height: 1,
    };
    let hint_para = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(hint_para, hint_area);
}

fn draw_error(frame: &mut Frame, area: Rect, message: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .border_type(BorderType::Rounded)
        .title(" ⚠ Error ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::Red))),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let para = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn draw_info(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .border_type(BorderType::Rounded)
        .title(format!(" ℹ {} ", title));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to continue",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let para = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

fn draw_dependency_check(
    frame: &mut Frame,
    area: Rect,
    statuses: &[crate::app::DependencyStatus],
    phase: &DependencyCheckPhase,
    completed_at: &Option<std::time::Instant>,
) {
    let (title, border_color) = match phase {
        DependencyCheckPhase::Checking => (" 🔍 Checking Dependencies ", Color::Cyan),
        DependencyCheckPhase::Installing => (" 📦 Installing Dependencies ", Color::Yellow),
        DependencyCheckPhase::Complete => (" ✓ Dependencies Ready ", Color::Green),
        DependencyCheckPhase::Failed => (" ✗ Dependency Check Failed ", Color::Red),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .border_type(BorderType::Rounded)
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut content = vec![Line::from("")];

    for status in statuses {
        let (icon, color) = match &status.state {
            DependencyState::Pending => ("○", Color::DarkGray),
            DependencyState::Checking => ("◐", Color::Cyan),
            DependencyState::Installing => ("◐", Color::Yellow),
            DependencyState::Ok => ("✓", Color::Green),
            DependencyState::Failed(_) => ("✗", Color::Red),
            DependencyState::Skipped => ("○", Color::DarkGray),
        };

        let state_text = match &status.state {
            DependencyState::Pending => "pending".to_string(),
            DependencyState::Checking => "checking...".to_string(),
            DependencyState::Installing => "installing...".to_string(),
            DependencyState::Ok => "ready".to_string(),
            DependencyState::Failed(e) => format!("failed: {}", e),
            DependencyState::Skipped => "skipped".to_string(),
        };

        let req_marker = if status.required { " *" } else { "" };

        content.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{} ", icon), Style::default().fg(color)),
            Span::styled(
                format!("{}{}", status.name, req_marker),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(&status.description, Style::default().fg(Color::Gray)),
        ]));

        content.push(Line::from(vec![
            Span::raw("      "),
            Span::styled(state_text, Style::default().fg(color)),
        ]));
    }

    content.push(Line::from(""));

    // Status message at bottom
    let status_msg = match phase {
        DependencyCheckPhase::Checking => "Verifying system dependencies...",
        DependencyCheckPhase::Installing => "Installing missing packages (may require sudo)...",
        DependencyCheckPhase::Complete => {
            if completed_at.is_some() {
                "All dependencies ready. Starting deployment..."
            } else {
                "All dependencies ready."
            }
        }
        DependencyCheckPhase::Failed => "Press [Esc] to cancel or [R] to retry",
    };

    content.push(Line::from(Span::styled(
        status_msg,
        Style::default()
            .fg(match phase {
                DependencyCheckPhase::Complete => Color::Green,
                DependencyCheckPhase::Failed => Color::Red,
                _ => Color::Cyan,
            })
            .add_modifier(Modifier::ITALIC),
    )));

    // Show legend for required marker
    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  * = required dependency",
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(content);
    frame.render_widget(para, inner);
}
