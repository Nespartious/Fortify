//! Log panel UI - Split view: Status Bar + Verified Traffic + Threat Traffic

use chrono::{Timelike, Utc};
use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Focus};
use crate::logging::{ComponentStatus, NetworkEvent, NetworkEventBuffer};

/// Draw the log panel (right side) - 3 sections: status bar, verified traffic, threat traffic
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Logs;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(" ▶ SYSTEM MONITOR ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into three panels: status (compact) + verified traffic + threat traffic
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Compact status bar (2 lines + border)
            Constraint::Min(5),     // Verified traffic (fill)
            Constraint::Length(12), // Threat traffic (50% taller for attack visibility)
        ])
        .split(inner);

    // Draw compact status bar
    draw_status_bar(frame, app, layout[0]);

    // Draw verified traffic stream
    draw_traffic_panel(frame, app, layout[1], true);

    // Draw threat traffic stream
    draw_traffic_panel(frame, app, layout[2], false);
}

/// Draw the compact status bar (top) - horizontal layout
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status = &app.system_status;
    let backend = app.backend_health;

    // Row 1: Core services (Tor, Gate, Controller, Backend)
    let row1 = Line::from(vec![
        Span::raw(" "),
        status_compact("Tor", status.tor_daemon),
        Span::raw("  "),
        status_compact("Gate", status.gate),
        Span::raw("  "),
        status_compact("Ctrl", status.controller),
        Span::raw("  "),
        Span::styled(
            format!("{} Backend", backend.symbol()),
            Style::default().fg(backend.color()),
        ),
    ]);

    // Row 2: Resources + TWO STATUS LIGHTS (System + Security)
    let (orch_cur, _orch_tgt) = status.orchestrators;
    let (live, standby, _) = status.mirrors;
    let (captcha_cur, captcha_tgt) = status.captcha_pool;

    // Compute system health (internal services only)
    let (system_color, system_label) = compute_system_health(app);

    // Get security status (attack detection)
    let security = &app.security_status;
    let security_color = security.level.color();
    let security_label = security.level.label();

    // Current time
    let now = Utc::now();
    let clock_str = format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second());

    let row2 = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("{} Orch:{}", status.orchestrator_status.symbol(), orch_cur),
            Style::default().fg(status.orchestrator_status.color()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} Mir:{}+{}", status.mirror_status.symbol(), live, standby),
            Style::default().fg(status.mirror_status.color()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("CAP:{}/{}", captcha_cur, captcha_tgt),
            Style::default().fg(status.captcha_status.color()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("🕐{} ", clock_str),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" │ "),
        // System status light - BOLD and prominent
        Span::styled(
            "SYS:",
            Style::default()
                .fg(system_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            system_label.to_string(),
            Style::default()
                .fg(system_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        // Security status light - BOLD and prominent
        Span::styled(
            "SEC:",
            Style::default()
                .fg(security_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            security_label,
            Style::default()
                .fg(security_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let para = Paragraph::new(vec![row1, row2]);
    frame.render_widget(para, inner);
}

/// Draw a traffic panel (verified or threat)
fn draw_traffic_panel(frame: &mut Frame, app: &App, area: Rect, is_verified: bool) {
    let (title, title_color, events) =
        if is_verified {
            (" ✓ Verified Traffic ", Color::Green, &app.network_events)
        } else {
            // Title changes based on security level
            let security_level = &app.security_status.level;
            let title = match security_level {
                crate::logging::SecurityLevel::Attack | crate::logging::SecurityLevel::Warning => {
                    " 🔴 Active Threats "
                }
                crate::logging::SecurityLevel::Suspicious
                | crate::logging::SecurityLevel::Elevated => " ⚠ Suspicious Activity ",
                _ => " 👁 Pending Verification ",
            };
            let color = match security_level {
                crate::logging::SecurityLevel::Attack | crate::logging::SecurityLevel::Warning => {
                    Color::Red
                }
                crate::logging::SecurityLevel::Suspicious
                | crate::logging::SecurityLevel::Elevated => Color::Yellow,
                _ => Color::DarkGray,
            };
            (title, color, &app.threat_events)
        };

    let mode_indicator = if app.logs_paused { "[PAUSED]" } else { "" };

    // Count activity in last 5 minutes
    let recent_count = count_recent_events(events, 300);
    let activity_indicator = format!(" ⚡{}req/5m ", recent_count);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!("{}{}", title, mode_indicator),
            Style::default().fg(title_color),
        ))
        .title_bottom(
            Line::from(vec![Span::styled(
                activity_indicator,
                Style::default().fg(if recent_count > 50 {
                    Color::Red
                } else if recent_count > 20 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }),
            )])
            .right_aligned(),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get events to display
    let traffic_height = inner.height.saturating_sub(1) as usize; // -1 for header
    let events_list = events.recent(traffic_height);

    let mut lines = Vec::new();

    // Compact header with timestamp column
    lines.push(Line::from(vec![
        Span::styled(
            "TIME     ",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "SID      ",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "MTD ",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "PATH",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if events_list.is_empty() {
        let waiting_msg = if is_verified {
            "  Waiting for verified traffic..."
        } else {
            "  No threat traffic detected"
        };
        lines.push(Line::from(Span::styled(
            waiting_msg,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Calculate path max width (leave room for timestamp + other columns)
        let path_max = inner.width.saturating_sub(32) as usize; // Adjusted for timestamp

        for event in events_list {
            lines.push(format_traffic_line(event, path_max, is_verified));
        }
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

/// Format a single traffic line with timestamp
fn format_traffic_line(event: &NetworkEvent, path_max: usize, is_verified: bool) -> Line<'static> {
    let status_color = event.status.color();

    // Format timestamp as HH:MM:SS
    let time_str = format!(
        "{:02}:{:02}:{:02} ",
        event.timestamp.hour(),
        event.timestamp.minute(),
        event.timestamp.second()
    );

    // Asset bundles get special styling
    let (method_text, method_color, path_color) = if event.is_asset_bundle {
        (
            "📦".to_string(),
            Color::Rgb(150, 150, 150),
            Color::Rgb(150, 150, 150),
        )
    } else {
        let base_color = if is_verified {
            Color::White
        } else {
            Color::Yellow
        };
        (
            format!("{:<3}", event.method.label()),
            event.method.color(),
            base_color,
        )
    };

    let status_text = match event.status_code {
        Some(code) => format!("{:3}", code),
        None => "---".to_string(),
    };

    Line::from(vec![
        Span::styled(time_str, Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:<8} ", event.display_session()),
            Style::default().fg(if is_verified {
                Color::Cyan
            } else {
                Color::Rgb(255, 165, 0)
            }),
        ),
        Span::styled(
            format!("{} ", method_text),
            Style::default().fg(method_color),
        ),
        Span::styled(
            format!("{:<width$}", event.display_path(path_max), width = path_max),
            Style::default().fg(path_color),
        ),
        Span::styled(
            format!(" {}", status_text),
            Style::default().fg(status_color),
        ),
    ])
}

/// Create a compact status indicator
fn status_compact(label: &str, status: ComponentStatus) -> Span<'static> {
    Span::styled(
        format!("{} {}", status.symbol(), label),
        Style::default().fg(status.color()),
    )
}

/// Compute system health (internal services only) and return (color, label)
/// This only checks service status, NOT attack detection (that's SecurityStatus)
fn compute_system_health(app: &App) -> (Color, String) {
    use crate::app::BackendHealthState;
    use crate::logging::ComponentStatus;

    let status = &app.system_status;
    let backend = app.backend_health;

    // Check for failures
    let has_failure = backend == BackendHealthState::Disconnected
        || status.tor_daemon == ComponentStatus::Error
        || status.gate == ComponentStatus::Error
        || status.controller == ComponentStatus::Error
        || status.orchestrator_status == ComponentStatus::Error
        || status.mirror_status == ComponentStatus::Error;

    // Check for degraded state
    let is_degraded = backend == BackendHealthState::Degraded1of3
        || backend == BackendHealthState::Degraded2of3
        || status.tor_daemon == ComponentStatus::Warning
        || status.gate == ComponentStatus::Warning
        || status.controller == ComponentStatus::Warning
        || status.orchestrator_status == ComponentStatus::Warning
        || status.mirror_status == ComponentStatus::Warning
        || status.captcha_status == ComponentStatus::Warning;

    // System health is purely about internal services
    if has_failure {
        (Color::Red, "FAILURE".to_string())
    } else if is_degraded {
        (Color::Yellow, "Degraded".to_string())
    } else {
        (Color::Green, "Healthy".to_string())
    }
}

/// Count events in the last N seconds
fn count_recent_events(buffer: &NetworkEventBuffer, seconds: i64) -> usize {
    let cutoff = Utc::now() - chrono::Duration::seconds(seconds);
    buffer.all().filter(|e| e.timestamp > cutoff).count()
}
