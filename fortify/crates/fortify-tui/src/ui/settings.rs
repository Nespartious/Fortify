//! Settings panel UI

use ratatui::{prelude::*, widgets::*};

use crate::app::{App, Focus, SettingsTab};

/// Draw settings screen
pub fn draw(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    current_tab: SettingsTab,
    field_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.focus == Focus::Settings {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(" ⚙ Settings ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: tabs, content, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Content
            Constraint::Length(2), // Footer
        ])
        .split(inner);

    // Draw tabs
    draw_tabs(frame, current_tab, layout[0]);

    // Draw content based on tab
    match current_tab {
        SettingsTab::Branding => draw_branding(frame, app, layout[1], field_index),
        SettingsTab::Captcha => draw_captcha(frame, app, layout[1], field_index),
        SettingsTab::Thresholds => draw_thresholds(frame, app, layout[1], field_index),
        SettingsTab::Network => draw_network(frame, app, layout[1], field_index),
        SettingsTab::Mirrors => draw_mirrors(frame, app, layout[1], field_index),
        SettingsTab::Vanity => draw_vanity(frame, app, layout[1], field_index),
    }

    // Draw footer
    draw_footer(frame, app, layout[2]);
}

fn draw_tabs(frame: &mut Frame, current: SettingsTab, area: Rect) {
    let available_width = area.width as usize;
    let tabs = SettingsTab::all();

    // Calculate full width needed for verbose tabs
    let full_width: usize = tabs
        .iter()
        .map(|t| t.label().len() + 5) // " Label │ "
        .sum();

    // Use compact dot mode if tabs would overflow
    let use_compact = full_width > available_width.saturating_sub(4);

    if use_compact {
        // Compact: ● ○ ○ ○ ○ ○  [Branding]
        let mut progress: Vec<Span> = Vec::new();

        for tab in tabs.iter() {
            let is_current = *tab == current;
            let (symbol, color) = if is_current {
                ("◉", Color::Yellow)
            } else {
                ("○", Color::DarkGray)
            };
            progress.push(Span::styled(
                format!("{} ", symbol),
                Style::default().fg(color),
            ));
        }

        // Add current tab name
        progress.push(Span::raw(" "));
        progress.push(Span::styled(
            format!("[{}]", current.label()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

        let line = Line::from(progress);
        let para = Paragraph::new(line)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(para, area);
    } else {
        // Full format with labels
        let titles: Vec<Line> = tabs
            .iter()
            .map(|tab| {
                let style = if *tab == current {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::styled(format!(" {} ", tab.label()), style)
            })
            .collect();

        let tabs_widget = Tabs::new(titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(Span::raw(" │ "));

        frame.render_widget(tabs_widget, area);
    }
}

fn draw_branding(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let logo_path = app
        .config
        .branding
        .logo_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(none)".to_string());

    let custom_css = app
        .config
        .branding
        .custom_css
        .as_ref()
        .map(|s| {
            if s.len() > 30 {
                format!("{}...", &s[..30])
            } else {
                s.clone()
            }
        })
        .unwrap_or_else(|| "(none)".to_string());

    let fields = [
        ("Service Name", app.config.branding.service_name.as_str()),
        ("Description", app.config.branding.description.as_str()),
        (
            "Welcome Message",
            app.config.branding.welcome_message.as_str(),
        ),
        ("Primary Color", app.config.branding.primary_color.as_str()),
        (
            "Secondary Color",
            app.config.branding.secondary_color.as_str(),
        ),
        (
            "Tertiary Color",
            app.config.branding.tertiary_color.as_str(),
        ),
        ("Logo Path", logo_path.as_str()),
        ("Custom CSS", custom_css.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);
}

fn draw_captcha(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let enabled = if app.config.captcha.enabled {
        "Yes"
    } else {
        "No"
    };
    let pool = app.config.captcha.pool_size.to_string();
    let min_pool = app.config.captcha.min_pool_size.to_string();
    let max_pool = app.config.captcha.max_pool_size.to_string();
    let diff = app.config.captcha.difficulty.to_string();
    let timeout = app.config.captcha.timeout_seconds.to_string();
    let attempts = app.config.captcha.max_attempts.to_string();
    let audio = if app.config.captcha.audio_enabled {
        "Yes"
    } else {
        "No"
    };
    let rotation_pct = app.config.captcha.rotation_percent.to_string();
    let rotation_days = app.config.captcha.rotation_interval_days.to_string();

    let fields = [
        ("Enabled", enabled),
        ("Pool Size", pool.as_str()),
        ("Min Pool", min_pool.as_str()),
        ("Max Pool", max_pool.as_str()),
        ("Difficulty (1-10)", diff.as_str()),
        ("Timeout (seconds)", timeout.as_str()),
        ("Max Attempts", attempts.as_str()),
        ("Audio Enabled", audio),
        ("Rotation %", rotation_pct.as_str()),
        ("Rotation Days", rotation_days.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);
}

fn draw_thresholds(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let rate = app.config.thresholds.rate_limit_rpm.to_string();
    let fail = app.config.thresholds.captcha_fail_limit.to_string();
    let temp = app.config.thresholds.temp_ban_minutes.to_string();
    let perm = app.config.thresholds.perm_ban_threshold.to_string();
    let suspicion = format!("{:.1}", app.config.thresholds.suspicion_threshold);
    let threat = format!("{:.1}", app.config.thresholds.threat_threshold);
    let burn = app.config.thresholds.burn_threshold.to_string();
    let auto_ban = if app.config.thresholds.auto_ban_enabled {
        "Yes"
    } else {
        "No"
    };
    let ddos = app.config.thresholds.ddos_rps_threshold.to_string();
    let probe = app.config.thresholds.probe_sensitivity.to_string();

    let fields = [
        ("Rate Limit (req/min)", rate.as_str()),
        ("CAPTCHA Fail Limit", fail.as_str()),
        ("Temp Ban Duration (min)", temp.as_str()),
        ("Perm Ban Threshold", perm.as_str()),
        ("Suspicion Threshold", suspicion.as_str()),
        ("Threat Threshold", threat.as_str()),
        ("Burn Threshold", burn.as_str()),
        ("Auto Ban Enabled", auto_ban),
        ("DDoS RPS Threshold", ddos.as_str()),
        ("Probe Sensitivity (1-10)", probe.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);
}

fn draw_network(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let socks = app.config.network.socks_port.to_string();
    let ctrl = app.config.network.control_port.to_string();
    let vg = if app.config.network.vanguards_enabled {
        "Yes"
    } else {
        "No"
    };
    let vg_layers = format!(
        "L2: {}, L3: {}",
        app.config.network.vanguards_layer2, app.config.network.vanguards_layer3
    );
    let data_dir = app.config.network.data_dir.display().to_string();

    let fields = [
        (
            "Backend Address",
            app.config.network.backend_address.as_str(),
        ),
        ("HTTP Bind", app.config.network.http_bind.as_str()),
        ("Gate Bind", app.config.network.gate_bind.as_str()),
        ("SOCKS Port", socks.as_str()),
        ("Control Port", ctrl.as_str()),
        ("Vanguards Enabled", vg),
        ("Vanguards Layers", vg_layers.as_str()),
        ("Data Directory", data_dir.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);
}

fn draw_mirrors(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let min = app.config.mirrors.min_mirrors.to_string();
    let max = app.config.mirrors.max_mirrors.to_string();
    let standby = app.config.mirrors.standby_mirrors.to_string();
    let rotation = app.config.mirrors.rotation_interval_seconds.to_string();
    let proactive = if app.config.mirrors.proactive_burn_enabled {
        "Yes"
    } else {
        "No"
    };
    let burn_range = format!(
        "{}-{} days",
        app.config.mirrors.burn_interval_days_min, app.config.mirrors.burn_interval_days_max
    );
    let retire = app.config.mirrors.retirement_page_hours.to_string();

    let fields = [
        ("Min Active Mirrors", min.as_str()),
        ("Max Mirrors", max.as_str()),
        ("Standby Mirrors", standby.as_str()),
        ("Rotation Interval (sec)", rotation.as_str()),
        ("Proactive Burn Enabled", proactive),
        ("Burn Interval Range", burn_range.as_str()),
        ("Retirement Page (hours)", retire.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);
}

fn draw_vanity(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let enabled = if app.config.vanity.enabled {
        "Yes"
    } else {
        "No"
    };
    let prefix = if app.config.vanity.prefix.is_empty() {
        "(not set)".to_string()
    } else {
        app.config.vanity.prefix.clone()
    };
    let prefix_len = format!("{}/10 characters", app.config.vanity.prefix.len());
    let safety_net = if app.config.vanity.safety_net_enabled {
        "Yes"
    } else {
        "No"
    };
    let timeout = format!("{} seconds", app.config.vanity.safety_net_timeout_seconds);
    let min_len = app.config.vanity.min_prefix_length.to_string();
    let warn = format!("{} chars", app.config.vanity.warn_threshold);

    let fields = [
        ("Vanity Enabled", enabled),
        ("Prefix", prefix.as_str()),
        ("Prefix Length", prefix_len.as_str()),
        ("Safety Net Enabled", safety_net),
        ("Vanity Timeout (sec)", timeout.as_str()),
        ("Min Prefix Length", min_len.as_str()),
        ("Warn Threshold", warn.as_str()),
    ];

    draw_field_list(frame, area, &fields, selected);

    // Show warning if prefix is too long
    if app.config.vanity.prefix.len() > app.config.vanity.warn_threshold {
        let warning_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 2,
        };
        let warning = Paragraph::new(Line::from(vec![
            Span::styled(
                "  ⚠ WARNING: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "Prefix > {} chars may take hours/days to generate!",
                    app.config.vanity.warn_threshold
                ),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        frame.render_widget(warning, warning_area);
    }
}

fn draw_field_list(frame: &mut Frame, area: Rect, fields: &[(&str, &str)], selected: usize) {
    // Calculate available width for content (minus borders and padding)
    let content_width = area.width.saturating_sub(4) as usize;
    let label_width = 22.min(content_width / 2); // Max 22 chars for label
    let value_width = content_width.saturating_sub(label_width + 6); // Rest for value

    let lines: Vec<Line> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let is_selected = i == selected;

            // Colored dot indicator instead of arrow
            let dot = if is_selected { "●" } else { "○" };
            let dot_style = if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let value_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Truncate label if too long
            let truncated_label: String = if label.len() > label_width {
                format!("{}…", &label[..label_width.saturating_sub(1)])
            } else {
                (*label).to_string()
            };

            // Truncate value if too long
            let truncated_value: String = if value.len() > value_width {
                format!("{}…", &value[..value_width.saturating_sub(1)])
            } else {
                (*value).to_string()
            };

            Line::from(vec![
                Span::styled(format!(" {} ", dot), dot_style),
                Span::styled(format!("{}: ", truncated_label), label_style),
                Span::styled(truncated_value, value_style),
            ])
        })
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let dirty_indicator = if app.config.is_dirty() {
        Span::styled(" [Modified] ", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("")
    };

    let hints = Line::from(vec![
        Span::styled("[←→]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Tab "),
        Span::styled("[↑↓]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Select "),
        Span::styled("[Enter]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Edit "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Back "),
        dirty_indicator,
    ]);

    let footer = Paragraph::new(hints).alignment(Alignment::Center);
    frame.render_widget(footer, area);
}
