//! Event handling for TUI

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

/// Application event
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Tick (for animations/updates)
    Tick,
    /// Log received
    Log(String),
    /// Deployment state changed
    DeploymentStateChanged,
}

/// Check if key is a quit key
pub fn is_quit_key(key: KeyEvent) -> bool {
    matches!(
        key,
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            ..
        } | KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
    )
}
