//! Home screen UI

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Focus, MenuItem};

/// Draw home screen
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    // Main container
    let version = env!("FORTIFY_VERSION");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.focus == Focus::Menu {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(format!(" 🏰 FORTIFY CONTROL CENTER v{} ", version));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into header, menu, and footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header/logo
            Constraint::Min(10),   // Menu
            Constraint::Length(3), // Footer hints
        ])
        .split(inner);

    // Draw ASCII logo/header
    draw_header(frame, layout[0]);

    // Draw menu
    draw_menu(frame, app, layout[1]);

    // Draw footer hints
    draw_footer(frame, app, layout[2]);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let logo = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ███████╗ ██████╗ ██████╗ ████████╗██╗███████╗██╗   ██╗",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  ██╔════╝██╔═══██╗██╔══██╗╚══██╔══╝██║██╔════╝╚██╗ ██╔╝",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "  █████╗  ██║   ██║██████╔╝   ██║   ██║█████╗   ╚████╔╝ ",
            Style::default().fg(Color::LightCyan),
        )),
        Line::from(Span::styled(
            "  ██╔══╝  ██║   ██║██╔══██╗   ██║   ██║██╔══╝    ╚██╔╝  ",
            Style::default().fg(Color::LightCyan),
        )),
        Line::from(Span::styled(
            "  ██║     ╚██████╔╝██║  ██║   ██║   ██║██║        ██║   ",
            Style::default().fg(Color::White),
        )),
    ];

    let para = Paragraph::new(logo);
    frame.render_widget(para, area);
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<Line> = MenuItem::all()
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.menu_index;
            let prefix = if is_selected { " ▶ " } else { "   " };

            // Use item's color (Destroy is red)
            let base_color = item.color();

            let style = if is_selected {
                Style::default()
                    .fg(if base_color == Color::Red {
                        Color::LightRed
                    } else {
                        Color::Yellow
                    })
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(base_color)
            };

            let hotkey_style = if base_color == Color::Red {
                Style::default().fg(Color::Red).add_modifier(Modifier::DIM)
            } else {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            };

            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("[{}] ", item.hotkey()), hotkey_style),
                Span::styled(item.label(), style),
            ])
        })
        .collect();

    // Add some padding
    let mut lines = vec![Line::from("")];
    lines.extend(items);

    let menu = Paragraph::new(lines);
    frame.render_widget(menu, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints = if app.deployment.is_running() {
        "[Tab] Switch Panel  [S] Settings  [Ctrl+C] Stop"
    } else {
        "[↑↓] Navigate  [Enter] Select  [Tab] Logs  [Q] Quit"
    };

    let footer = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
