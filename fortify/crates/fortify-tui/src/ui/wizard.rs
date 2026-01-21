//! Deployment wizard UI

use ratatui::{
    prelude::*,
    widgets::*,
};

use crate::app::App;
use crate::deployment::check_dependencies;

/// Mask backend address for security in summary views
/// Shows first 12 and last 5 characters of the host
fn mask_backend_address(addr: &str) -> String {
    if let Some(scheme_end) = addr.find("://") {
        let after_scheme = &addr[scheme_end + 3..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let host = &after_scheme[..host_end];
        let path = &after_scheme[host_end..];
        
        if host.len() > 17 {
            let first = &host[..12];
            let last = &host[host.len()-5..];
            format!("{}://{}•••{}{}", &addr[..scheme_end], first, last, path)
        } else {
            addr.to_string()
        }
    } else if addr.len() > 17 {
        let first = &addr[..12];
        let last = &addr[addr.len()-5..];
        format!("{}•••{}", first, last)
    } else {
        addr.to_string()
    }
}

const WIZARD_STEPS: &[(&str, &str)] = &[
    ("Deps", "Check system dependencies"),
    ("Branding", "Configure your service identity"),
    ("CAPTCHA", "Set up challenge verification"),
    ("Thresholds", "Define security limits"),
    ("Network", "Configure addresses and ports"),
    ("Mirrors", "Configure mirrors and vanity addresses"),
    ("Review", "Review and deploy"),
];

/// Draw deployment wizard
pub fn draw(frame: &mut Frame, app: &App, area: Rect, step: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 🚀 New Deployment Wizard ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: progress, content, navigation
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Progress
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Navigation
        ])
        .split(inner);

    // Draw progress indicator
    draw_progress(frame, step, layout[0]);

    // Draw step content
    match step {
        0 => draw_step_dependencies(frame, app, layout[1]),
        1 => draw_step_branding(frame, app, layout[1]),
        2 => draw_step_captcha(frame, app, layout[1]),
        3 => draw_step_thresholds(frame, app, layout[1]),
        4 => draw_step_network(frame, app, layout[1]),
        5 => draw_step_mirrors(frame, app, layout[1]),
        6 => draw_step_review(frame, app, layout[1]),
        _ => {}
    }

    // Draw navigation
    draw_navigation(frame, step, layout[2]);
}

fn draw_progress(frame: &mut Frame, current: usize, area: Rect) {
    // Use compact format: show step numbers with dots for completed/pending
    // and only show current step name to avoid overflow
    let available_width = area.width as usize;
    
    // Calculate approximate length of full progress bar
    let full_length: usize = WIZARD_STEPS.iter()
        .map(|(name, _)| 2 + name.len() + 3) // "N.Name → "
        .sum();
    
    // Use compact mode if we don't have enough space
    let use_compact = full_length > available_width.saturating_sub(4);
    
    if use_compact {
        // Compact: ● ● ● ○ ○ ○ ○  [Step 4: Network]
        let mut progress: Vec<Span> = Vec::new();
        
        for i in 0..WIZARD_STEPS.len() {
            let (symbol, color) = if i < current {
                ("●", Color::Green)
            } else if i == current {
                ("◉", Color::Yellow)
            } else {
                ("○", Color::DarkGray)
            };
            progress.push(Span::styled(format!("{} ", symbol), Style::default().fg(color)));
        }
        
        // Add current step name
        let (step_name, _step_desc) = WIZARD_STEPS[current];
        progress.push(Span::raw(" "));
        progress.push(Span::styled(
            format!("[{}/{}] {}", current + 1, WIZARD_STEPS.len(), step_name),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        ));
        
        let line = Line::from(progress);
        let para = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(para, area);
    } else {
        // Full format: 1.Deps → 2.Branding → ...
        let progress: Vec<Span> = WIZARD_STEPS
            .iter()
            .enumerate()
            .flat_map(|(i, (name, _))| {
                let style = if i < current {
                    Style::default().fg(Color::Green)
                } else if i == current {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let connector = if i < WIZARD_STEPS.len() - 1 {
                    Span::styled(" → ", Style::default().fg(Color::DarkGray))
                } else {
                    Span::raw("")
                };

                vec![
                    Span::styled(format!("{}", i + 1), style),
                    Span::styled(format!(".{}", name), style),
                    connector,
                ]
            })
            .collect();

        let line = Line::from(progress);
        let para = Paragraph::new(line).alignment(Alignment::Center);
        frame.render_widget(para, area);
    }
}

fn draw_step_dependencies(frame: &mut Frame, _app: &App, area: Rect) {
    // Check dependencies synchronously for UI
    let results = check_dependencies();
    
    let required_ok = results.iter().filter(|r| r.required && r.available).count();
    let required_total = results.iter().filter(|r| r.required).count();
    let optional_ok = results.iter().filter(|r| !r.required && r.available).count();
    let optional_total = results.iter().filter(|r| !r.required).count();
    
    let all_required_met = results.iter().filter(|r| r.required).all(|r| r.available);
    
    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  System Dependencies",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Required: "),
            Span::styled(
                format!("{}/{} installed", required_ok, required_total),
                if all_required_met { 
                    Style::default().fg(Color::Green) 
                } else { 
                    Style::default().fg(Color::Red) 
                }
            ),
        ]),
        Line::from(vec![
            Span::raw("  Optional: "),
            Span::styled(
                format!("{}/{} installed", optional_ok, optional_total),
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from(""),
    ];
    
    // List each dependency
    for result in &results {
        let (icon, color) = if result.available {
            ("✓", Color::Green)
        } else if result.required {
            ("✗", Color::Red)
        } else {
            ("○", Color::Yellow)
        };
        
        let req_marker = if result.required { " (required)" } else { " (optional)" };
        
        content.push(Line::from(vec![
            Span::styled(format!("  {} ", icon), Style::default().fg(color)),
            Span::styled(result.name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(req_marker, Style::default().fg(Color::DarkGray)),
            Span::raw(" - "),
            Span::styled(result.description.clone(), Style::default().fg(Color::Gray)),
        ]));
        
        if !result.available {
            content.push(Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    format!("Install: {}", result.install_hint),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
                ),
            ]));
        }
    }
    
    content.push(Line::from(""));
    
    if !all_required_met {
        content.push(Line::from(Span::styled(
            "  ⚠ Missing required dependencies! Press [I] to install",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        )));
    } else {
        content.push(Line::from(Span::styled(
            "  ✓ All required dependencies met. Press [→] to continue",
            Style::default().fg(Color::Green)
        )));
        if optional_ok < optional_total {
            content.push(Line::from(Span::styled(
                "  Optional: Press [I] to install missing optional dependencies",
                Style::default().fg(Color::DarkGray)
            )));
        }
    }
    
    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_branding(frame: &mut Frame, app: &App, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Service Identity",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Name: "),
            Span::styled(&app.config.branding.service_name, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Description: "),
            Span::styled(&app.config.branding.description, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Color: "),
            Span::styled(&app.config.branding.primary_color, Style::default().fg(Color::Magenta)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Logo: Max 256x256 PNG/JPG",
            Style::default().fg(Color::DarkGray)
        )),
        Line::from(vec![
            Span::raw("  Path: "),
            Span::styled(
                app.config.branding.logo_path.as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(none)".to_string()),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [S] to open Settings and configure these values",
            Style::default().fg(Color::DarkGray)
        )),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_captcha(frame: &mut Frame, app: &App, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  CAPTCHA Configuration",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Enabled: "),
            Span::styled(
                if app.config.captcha.enabled { "Yes" } else { "No" },
                Style::default().fg(if app.config.captcha.enabled { Color::Green } else { Color::Red })
            ),
        ]),
        Line::from(vec![
            Span::raw("  Pool Size: "),
            Span::styled(
                app.config.captcha.pool_size.to_string(),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Difficulty: "),
            Span::styled(
                format!("{}/10", app.config.captcha.difficulty),
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Timeout: "),
            Span::styled(
                format!("{} seconds", app.config.captcha.timeout_seconds),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Max Attempts: "),
            Span::styled(
                app.config.captcha.max_attempts.to_string(),
                Style::default().fg(Color::White)
            ),
        ]),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_thresholds(frame: &mut Frame, app: &App, area: Rect) {
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Security Thresholds",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Rate Limit: "),
            Span::styled(
                format!("{} req/min", app.config.thresholds.rate_limit_rpm),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(vec![
            Span::raw("  CAPTCHA Fail Limit: "),
            Span::styled(
                app.config.thresholds.captcha_fail_limit.to_string(),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Temp Ban: "),
            Span::styled(
                format!("{} minutes", app.config.thresholds.temp_ban_minutes),
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Burn Threshold: "),
            Span::styled(
                format!("{:.0}%", app.config.thresholds.burn_threshold * 100.0),
                Style::default().fg(Color::Red)
            ),
        ]),
        Line::from(vec![
            Span::raw("  DDoS Detection: "),
            Span::styled(
                format!("{} req/sec", app.config.thresholds.ddos_rps_threshold),
                Style::default().fg(Color::White)
            ),
        ]),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_network(frame: &mut Frame, app: &App, area: Rect) {
    // Mask backend address in summary view for security
    let masked_backend = mask_backend_address(&app.config.network.backend_address);
    
    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Network Configuration",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(Span::raw("  Backend (protected service):")),
        Line::from(Span::styled(
            format!("    {}", masked_backend),
            Style::default().fg(Color::Green)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  HTTP Proxy: "),
            Span::styled(&app.config.network.http_bind, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Gate (CAPTCHA): "),
            Span::styled(&app.config.network.gate_bind, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::raw("  Tor SOCKS: "),
            Span::styled(format!("127.0.0.1:{}", app.config.network.socks_port), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Vanguards: "),
            Span::styled(
                if app.config.network.vanguards_enabled { "Enabled" } else { "Disabled" },
                Style::default().fg(if app.config.network.vanguards_enabled { Color::Green } else { Color::DarkGray })
            ),
        ]),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_mirrors(frame: &mut Frame, app: &App, area: Rect) {
    let vanity_status = if app.config.vanity.enabled {
        if app.config.vanity.prefix.is_empty() {
            ("Enabled (no prefix set)", Color::Yellow)
        } else {
            ("Enabled", Color::Green)
        }
    } else {
        ("Disabled", Color::DarkGray)
    };

    let prefix_display = if app.config.vanity.prefix.is_empty() {
        "(not set)".to_string()
    } else {
        app.config.vanity.prefix.clone()
    };

    // Warning for long prefixes with time estimates
    let prefix_warning = if app.config.vanity.prefix.len() > app.config.vanity.warn_threshold {
        let estimate = match app.config.vanity.prefix.len() {
            6 => "~30s per mirror",
            7 => "~15-20 min per mirror",
            _ => "hours+ (will timeout)",
        };
        Some(format!(
            "⚠ {} chars → {} - Consider 4-5 chars!",
            app.config.vanity.prefix.len(),
            estimate
        ))
    } else {
        None
    };

    let mut content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Mirror Configuration",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Active Mirrors: "),
            Span::styled(
                app.config.mirrors.min_mirrors.to_string(),
                Style::default().fg(Color::Green)
            ),
            Span::raw(" (max: "),
            Span::styled(
                app.config.mirrors.max_mirrors.to_string(),
                Style::default().fg(Color::White)
            ),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::raw("  Standby Mirrors: "),
            Span::styled(
                app.config.mirrors.standby_mirrors.to_string(),
                Style::default().fg(Color::Yellow)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Rotation: Every "),
            Span::styled(
                format!("{} seconds", app.config.mirrors.rotation_interval_seconds),
                Style::default().fg(Color::White)
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Vanity Address Generation",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Vanity Addresses: "),
            Span::styled(vanity_status.0, Style::default().fg(vanity_status.1)),
        ]),
        Line::from(vec![
            Span::raw("  Prefix: "),
            Span::styled(
                &prefix_display,
                Style::default().fg(if app.config.vanity.prefix.is_empty() { Color::DarkGray } else { Color::Cyan })
            ),
            Span::styled(
                format!(" ({}/10 chars)", app.config.vanity.prefix.len()),
                Style::default().fg(Color::DarkGray)
            ),
        ]),
    ];

    if let Some(warning) = prefix_warning {
        content.push(Line::from(Span::styled(
            format!("  {}", warning),
            Style::default().fg(Color::Yellow)
        )));
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::raw("  Safety Net: "),
        Span::styled(
            if app.config.vanity.safety_net_enabled { "Enabled" } else { "Disabled" },
            Style::default().fg(if app.config.vanity.safety_net_enabled { Color::Green } else { Color::DarkGray })
        ),
        Span::raw(" ("),
        Span::styled(
            format!("{}s timeout", app.config.vanity.safety_net_timeout_seconds),
            Style::default().fg(Color::White)
        ),
        Span::raw(")"),
    ]));
    content.push(Line::from(Span::styled(
        "  └ Auto-shortens prefix if generation takes too long",
        Style::default().fg(Color::DarkGray)
    )));

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_step_review(frame: &mut Frame, app: &App, area: Rect) {
    let vanity_info = if app.config.vanity.enabled && !app.config.vanity.prefix.is_empty() {
        format!("{}...", app.config.vanity.prefix)
    } else if app.config.vanity.enabled {
        "Enabled (no prefix)".to_string()
    } else {
        "Disabled".to_string()
    };

    // Mask backend address in summary view for security
    let masked_backend = mask_backend_address(&app.config.network.backend_address);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  📋 Deployment Summary",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Service: "),
            Span::styled(&app.config.branding.service_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(Span::raw("  Backend:")),
        Line::from(Span::styled(
            format!("    {}", masked_backend),
            Style::default().fg(Color::Green)
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Mirrors: "),
            Span::styled(
                format!("{} active, {} standby", app.config.mirrors.min_mirrors, app.config.mirrors.standby_mirrors),
                Style::default().fg(Color::Cyan)
            ),
        ]),
        Line::from(vec![
            Span::raw("  Vanity: "),
            Span::styled(
                &vanity_info,
                Style::default().fg(if app.config.vanity.enabled { Color::Magenta } else { Color::DarkGray })
            ),
        ]),
        Line::from(vec![
            Span::raw("  CAPTCHA: "),
            Span::styled(
                if app.config.captcha.enabled { "Enabled" } else { "Disabled" },
                Style::default().fg(if app.config.captcha.enabled { Color::Green } else { Color::Red })
            ),
        ]),
        Line::from(vec![
            Span::raw("  Vanguards: "),
            Span::styled(
                if app.config.network.vanguards_enabled { "Enabled" } else { "Disabled" },
                Style::default().fg(if app.config.network.vanguards_enabled { Color::Green } else { Color::DarkGray })
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ✓ Ready to deploy!",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [Enter] to start deployment",
            Style::default().fg(Color::Yellow)
        )),
    ];

    let para = Paragraph::new(content);
    frame.render_widget(para, area);
}

fn draw_navigation(frame: &mut Frame, step: usize, area: Rect) {
    let can_back = step > 0;
    let is_final = step >= WIZARD_STEPS.len() - 1;

    let left = if can_back {
        Span::styled("[← Back]", Style::default().fg(Color::White))
    } else {
        Span::styled("[← Back]", Style::default().fg(Color::DarkGray))
    };

    let right = if is_final {
        Span::styled("[Deploy →]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("[Next →]", Style::default().fg(Color::White))
    };

    let line = Line::from(vec![
        Span::raw("  "),
        left,
        Span::raw("          "),
        Span::styled("[S] Settings", Style::default().fg(Color::DarkGray)),
        Span::raw("          "),
        right,
        Span::raw("  "),
    ]);

    let nav = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(nav, area);
}
