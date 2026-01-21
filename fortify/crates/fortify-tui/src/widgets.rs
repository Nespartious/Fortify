//! Custom widgets for Fortify TUI

#![allow(dead_code)]
#![allow(unused_imports)]

use ratatui::{
    prelude::*,
    widgets::*,
};

/// A gauge widget for showing percentages
pub struct PercentGauge<'a> {
    label: &'a str,
    value: f32,
    style: Style,
}

impl<'a> PercentGauge<'a> {
    pub fn new(label: &'a str, value: f32) -> Self {
        Self {
            label,
            value: value.clamp(0.0, 100.0),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for PercentGauge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 1 {
            return;
        }

        let label_width = self.label.len() as u16 + 2;
        let gauge_width = area.width.saturating_sub(label_width + 8);
        let filled = (gauge_width as f32 * self.value / 100.0) as u16;

        // Render label
        buf.set_string(area.x, area.y, format!("{}: ", self.label), self.style);

        // Render gauge
        let gauge_x = area.x + label_width;
        for x in 0..gauge_width {
            let ch = if x < filled { '█' } else { '░' };
            let style = if x < filled {
                self.style.fg(Color::Green)
            } else {
                self.style.fg(Color::DarkGray)
            };
            buf.set_string(gauge_x + x, area.y, ch.to_string(), style);
        }

        // Render percentage
        let pct = format!(" {:.0}%", self.value);
        buf.set_string(gauge_x + gauge_width, area.y, pct, self.style);
    }
}

/// A status indicator (dot with label)
pub struct StatusIndicator<'a> {
    label: &'a str,
    active: bool,
    active_color: Color,
}

impl<'a> StatusIndicator<'a> {
    pub fn new(label: &'a str, active: bool) -> Self {
        Self {
            label,
            active,
            active_color: Color::Green,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.active_color = color;
        self
    }
}

impl Widget for StatusIndicator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (dot, color) = if self.active {
            ("●", self.active_color)
        } else {
            ("○", Color::DarkGray)
        };

        buf.set_string(area.x, area.y, dot, Style::default().fg(color));
        buf.set_string(area.x + 2, area.y, self.label, Style::default());
    }
}

/// ASCII box for containing content
pub fn ascii_box(title: &str, width: u16) -> Vec<String> {
    let inner_width = width.saturating_sub(4) as usize;
    let title_padded = if title.len() > inner_width {
        title[..inner_width].to_string()
    } else {
        format!(" {} ", title)
    };

    let top = format!("╔{}{}{}╗", 
        "═".repeat((inner_width - title_padded.len()) / 2),
        title_padded,
        "═".repeat((inner_width - title_padded.len() + 1) / 2)
    );

    vec![top]
}
