//! Running deployment view UI

use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::app::{App, MirrorStatusState};

/// Mask backend address for security - show first 12 and last 5 characters
/// Example: "http://abc123def456.onion/path" -> "http://abc12•••456.onion/path"
fn mask_backend_address(addr: &str) -> String {
    // Parse the address to find the host part
    if let Some(scheme_end) = addr.find("://") {
        let after_scheme = &addr[scheme_end + 3..];
        // Find the end of the host (either '/' or end of string)
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];
        let path = &after_scheme[host_end..];
        
        // Mask the host if it's long enough
        if host.len() > 17 {
            let first = &host[..12];
            let last = &host[host.len()-5..];
            format!("{}://{}•••{}{}", &addr[..scheme_end], first, last, path)
        } else {
            addr.to_string()
        }
    } else if addr.len() > 17 {
        // No scheme, just mask the whole thing
        let first = &addr[..12];
        let last = &addr[addr.len()-5..];
        format!("{}•••{}", first, last)
    } else {
        addr.to_string()
    }
}

/// Draw running deployment view
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" 🏰 FORTIFY - RUNNING ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: status, main area, controls
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Status
            Constraint::Min(10),    // Main area (split left/right)
            Constraint::Length(3),  // Controls
        ])
        .split(inner);

    // Draw status
    draw_status(frame, app, layout[0]);

    // Split main area into left (mirrors + health) and right (for future logs/info)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(100),  // Full width for now
        ])
        .split(layout[1]);

    // Split left column vertically: mirrors on top, mirror health in middle, backend health below
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),  // Mirrors
            Constraint::Percentage(30),  // Mirror health checks
            Constraint::Percentage(30),  // Backend health
        ])
        .split(main_layout[0]);

    // Draw mirror addresses on top
    draw_mirror_addresses(frame, app, left_layout[0]);

    // Draw mirror health checks in middle
    draw_mirror_health(frame, app, left_layout[1]);

    // Draw backend health below
    draw_backend_health(frame, app, left_layout[2]);

    // Draw controls
    draw_controls(frame, layout[2]);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let status_block = Block::default()
        .borders(Borders::BOTTOM)
        .title(" Status ");

    let inner = status_block.inner(area);
    frame.render_widget(status_block, area);

    // Mask backend address for security - show first 12 and last 5 chars only
    let backend_display = mask_backend_address(&app.config.network.backend_address);

    let content = vec![
        Line::from(vec![
            Span::raw("  State: "),
            Span::styled("● RUNNING", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("  │  Service: "),
            Span::styled(&app.config.branding.service_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(Span::raw("  Backend:")),
        Line::from(Span::styled(
            format!("    {}", backend_display),
            Style::default().fg(Color::White)
        )),
    ];

    let para = Paragraph::new(content).wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn draw_mirror_health(frame: &mut Frame, app: &App, area: Rect) {
    let health_block = Block::default()
        .borders(Borders::ALL)
        .title(" Mirror Health Checks ");

    let inner = health_block.inner(area);
    frame.render_widget(health_block, area);

    let mut lines = vec![];
    
    if app.mirror_health_checks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No mirror health checks yet...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Show health status for each mirror
        for (mirror_addr, checks) in &app.mirror_health_checks {
            // Shorten mirror address
            let short_addr = if mirror_addr.len() > 40 {
                format!("{}...{}", &mirror_addr[..20], &mirror_addr[mirror_addr.len()-15..])
            } else {
                mirror_addr.clone()
            };
            
            // Get latest check
            if let Some(latest) = checks.last() {
                let elapsed = latest.timestamp.elapsed();
                let status_symbol = if latest.success { "🟢" } else { "🔴" };
                let status_text = if latest.success { "REACHABLE" } else { "UNREACHABLE" };
                let status_color = if latest.success { Color::Green } else { Color::Red };
                
                let time_str = if elapsed.as_secs() < 60 {
                    format!("{}s ago", elapsed.as_secs())
                } else {
                    format!("{}m ago", elapsed.as_secs() / 60)
                };
                
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", status_symbol), Style::default().fg(status_color)),
                    Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" ({})", time_str), Style::default().fg(Color::DarkGray)),
                ]));
                
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(short_addr, Style::default().fg(Color::White)),
                ]));
                
                if latest.success {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("Response: {}ms", latest.duration.as_millis()),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                }
                
                lines.push(Line::from(""));
            }
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn draw_backend_health(frame: &mut Frame, app: &App, area: Rect) {
    let health_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.backend_health.color()))
        .title(" Backend Health ");

    let inner = health_block.inner(area);
    frame.render_widget(health_block, area);

    let mut lines = vec![];
    
    // Current status
    lines.push(Line::from(vec![
        Span::raw("  Status: "),
        Span::styled(
            format!("{} {}", app.backend_health.symbol(), app.backend_health.label()),
            Style::default().fg(app.backend_health.color()).add_modifier(Modifier::BOLD),
        ),
    ]));
    
    lines.push(Line::from(""));
    
    // Last check time
    if let Some(last_check) = app.backend_last_check {
        let elapsed = last_check.elapsed();
        lines.push(Line::from(vec![
            Span::raw("  Last Check: "),
            Span::styled(
                format!("{}s ago", elapsed.as_secs()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  Last Check: "),
            Span::styled("Never", Style::default().fg(Color::DarkGray)),
        ]));
    }
    
    // Next check interval
    lines.push(Line::from(vec![
        Span::raw("  Check Interval: "),
        Span::styled(
            format!("{}s", app.backend_check_interval),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Recent Checks:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));
    
    // Show last 10 health checks
    let history_to_show = app.backend_check_history.iter().rev().take(10);
    for check in history_to_show {
        let elapsed = check.timestamp.elapsed();
        let status_symbol = if check.success { "✓" } else { "✗" };
        let status_color = if check.success { Color::Green } else { Color::Red };
        
        let time_str = if elapsed.as_secs() < 60 {
            format!("{}s", elapsed.as_secs())
        } else {
            format!("{}m", elapsed.as_secs() / 60)
        };
        
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                status_symbol,
                Style::default().fg(status_color),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:>4} ago", time_str),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::styled(
                format!("({}ms)", check.duration.as_millis()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        
        // Show error message if check failed
        if !check.success {
            if let Some(ref error) = check.error {
                let error_short = if error.len() > 35 {
                    format!("{}...", &error[..32])
                } else {
                    error.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        error_short,
                        Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }
    }
    
    if app.backend_check_history.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No checks yet...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn draw_mirror_addresses(frame: &mut Frame, app: &App, area: Rect) {
    let mirrors_block = Block::default()
        .borders(Borders::ALL)
        .title(" Active Mirrors ");

    let inner = mirrors_block.inner(area);
    frame.render_widget(mirrors_block, area);

    let mut lines = vec![Line::from("")];

    if app.mirror_statuses.is_empty() {
        // Show placeholder while mirrors are initializing
        lines.push(Line::from(Span::styled(
            "  Initializing mirrors...",
            Style::default().fg(Color::Yellow),
        )));
        
        // Show vanity generation status if applicable
        if app.config.vanity.enabled {
            if let Some(ref prefix) = app.vanity_current_prefix {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  ◐ ", Style::default().fg(Color::Magenta)),
                    Span::raw("Generating vanity address: "),
                    Span::styled(format!("{}...", prefix), Style::default().fg(Color::Cyan)),
                ]));
            }
        }
    } else {
        // Show each mirror with status dot
        for mirror in &app.mirror_statuses {
            let status_color = mirror.state.color();
            let status_symbol = mirror.state.symbol();
            let status_label = mirror.state.label();
            
            let addr_display = if mirror.address.len() > 16 {
                format!("{}...{}.onion", 
                    &mirror.address[..8], 
                    &mirror.address[mirror.address.len()-8..])
            } else {
                format!("{}.onion", mirror.address)
            };

            let standby_marker = if mirror.is_standby { " [STANDBY]" } else { "" };
            
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status_symbol), Style::default().fg(status_color)),
                Span::styled(addr_display, Style::default().fg(Color::White)),
                Span::styled(
                    format!("  [{}]", status_label),
                    Style::default().fg(status_color),
                ),
                Span::styled(standby_marker.to_string(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    // Show summary
    let live_count = app.mirror_statuses.iter()
        .filter(|m| m.state == MirrorStatusState::Live && !m.is_standby)
        .count();
    let standby_count = app.mirror_statuses.iter()
        .filter(|m| m.is_standby)
        .count();
    
    if !app.mirror_statuses.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  Active: "),
            Span::styled(format!("{}", live_count), Style::default().fg(Color::Green)),
            Span::raw(" / "),
            Span::styled(format!("{}", app.config.mirrors.max_mirrors), Style::default().fg(Color::DarkGray)),
            Span::raw("  │  Standby: "),
            Span::styled(format!("{}", standby_count), Style::default().fg(Color::Yellow)),
        ]));
    }

    // Show vanity config if enabled
    if app.config.vanity.enabled && !app.config.vanity.prefix.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  Vanity Prefix: "),
            Span::styled(&app.config.vanity.prefix, Style::default().fg(Color::Magenta)),
        ]));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn draw_controls(frame: &mut Frame, area: Rect) {
    let controls = vec![
        Span::styled("[S]", Style::default().fg(Color::Yellow)),
        Span::raw(" Settings  "),
        Span::styled("[E]", Style::default().fg(Color::Cyan)),
        Span::raw(" Export  "),
        Span::styled("[P]", Style::default().fg(Color::Yellow)),
        Span::raw(" Pause  "),
        Span::styled("[F]", Style::default().fg(Color::Yellow)),
        Span::raw(" Filter  "),
        Span::styled("[C]", Style::default().fg(Color::Yellow)),
        Span::raw(" Clear  "),
        Span::styled("[Esc]", Style::default().fg(Color::Red)),
        Span::raw(" Stop"),
    ];

    let para = Paragraph::new(Line::from(controls))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(para, area);
}
