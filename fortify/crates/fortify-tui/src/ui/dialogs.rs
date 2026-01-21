//! Dialog rendering

use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::app::{App, Dialog};

/// Draw dialog overlay
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    
    // Calculate centered dialog area
    let dialog_width = 60.min(area.width.saturating_sub(4));
    let dialog_height = match &app.dialog {
        Dialog::Confirm { .. } => 8,
        Dialog::ApplyChanges { changes } => (8 + changes.len()).min(20) as u16,
        Dialog::Input { .. } => 7,
        Dialog::Error { .. } => 8,
        Dialog::Info { .. } => 8,
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
    let _overlay = Block::default()
        .style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, dialog_area);

    // Draw dialog content
    match &app.dialog {
        Dialog::Confirm { title, message, .. } => {
            draw_confirm(frame, dialog_area, title, message);
        }
        Dialog::ApplyChanges { changes } => {
            draw_apply_changes(frame, dialog_area, changes);
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

fn draw_apply_changes(frame: &mut Frame, area: Rect, changes: &[String]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .border_type(BorderType::Rounded)
        .title(" Apply Changes? ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Configuration has been modified:",
            Style::default().fg(Color::White)
        )),
        Line::from(""),
    ];

    // Add changes (limited)
    for change in changes.iter().take(5) {
        content.push(Line::from(Span::styled(
            format!("  • {}", change),
            Style::default().fg(Color::Yellow)
        )));
    }

    if changes.len() > 5 {
        content.push(Line::from(Span::styled(
            format!("  ... and {} more", changes.len() - 5),
            Style::default().fg(Color::DarkGray)
        )));
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled("[A]", Style::default().fg(Color::Green)),
        Span::raw(" Apply Now    "),
        Span::styled("[L]", Style::default().fg(Color::Yellow)),
        Span::raw(" Later    "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Cancel"),
    ]));

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
    let input_bg = Block::default()
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(input_bg, input_area);

    // Draw input value with cursor
    let display_value = format!("{}█", value);
    let input = Paragraph::new(display_value)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
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
            Style::default().fg(Color::DarkGray)
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
            Style::default().fg(Color::DarkGray)
        )),
    ];

    let para = Paragraph::new(content).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}
