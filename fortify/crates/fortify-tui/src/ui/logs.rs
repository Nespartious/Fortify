//! Log panel UI

use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::app::{App, Focus};

/// Draw the log panel (right side)
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Logs;
    
    let mode_indicator = if app.log_select_mode {
        "[SELECT]"
    } else if app.logs_paused {
        "[PAUSED]"
    } else {
        ""
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.log_select_mode {
            Style::default().fg(Color::Magenta)
        } else if is_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(format!(
            " ▶ LIVE LOGS {} {} ",
            mode_indicator,
            format!("[{:?}+]", app.log_filter)
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate visible area
    let log_height = inner.height as usize;
    
    // Get filtered logs
    let filtered_logs = app.logs.scroll(app.log_scroll, log_height, app.log_filter);

    // Convert to styled lines with selection highlight
    let lines: Vec<Line> = filtered_logs
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = app.log_select_mode && i == app.log_selected_line;
            
            let time_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let level_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(entry.level.color())
            };
            let source_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Blue)
            };
            let msg_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            Line::from(vec![
                Span::styled(
                    entry.timestamp.format("%H:%M:%S ").to_string(),
                    time_style
                ),
                Span::styled(
                    format!("{} ", entry.level.symbol()),
                    level_style
                ),
                Span::styled(
                    format!("[{}] ", truncate_source(&entry.source, 15)),
                    source_style
                ),
                Span::styled(&entry.message, msg_style),
            ])
        })
        .collect();

    // If no logs, show placeholder
    if lines.is_empty() {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Waiting for log entries...",
                Style::default().fg(Color::DarkGray)
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Logs will appear here when",
                Style::default().fg(Color::DarkGray)
            )),
            Line::from(Span::styled(
                "  deployment is running.",
                Style::default().fg(Color::DarkGray)
            )),
        ]);
        frame.render_widget(placeholder, inner);
    } else {
        let log_widget = Paragraph::new(lines)
            .wrap(Wrap { trim: false });
        frame.render_widget(log_widget, inner);
    }

    // Draw scroll indicator if there are more logs
    if app.log_scroll > 0 {
        let indicator = Paragraph::new(format!("↓ {} more", app.log_scroll))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Right);
        
        let indicator_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        frame.render_widget(indicator, indicator_area);
    }

    // Draw hints at bottom if focused
    if is_focused {
        let hints = if app.log_select_mode {
            " [↑↓] Select  [Y/Enter] Copy  [Esc] Exit Select "
        } else {
            " [S] Select  [P] Pause  [C] Clear  [↑↓/PgUp/Dn] Scroll "
        };
        let hint_line = Paragraph::new(hints)
            .style(if app.log_select_mode {
                Style::default().fg(Color::Magenta).bg(Color::Black)
            } else {
                Style::default().fg(Color::DarkGray).bg(Color::Black)
            })
            .alignment(Alignment::Center);
        
        // Overlay at bottom of log area
        let hint_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - 1,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        frame.render_widget(hint_line, hint_area);
    }
}

/// Truncate source name for display
fn truncate_source(source: &str, max_len: usize) -> String {
    // Remove common prefixes
    let short = source
        .strip_prefix("fortify_")
        .or_else(|| source.strip_prefix("fortify-"))
        .unwrap_or(source);

    if short.len() <= max_len {
        short.to_string()
    } else {
        format!("{}…", &short[..max_len - 1])
    }
}
