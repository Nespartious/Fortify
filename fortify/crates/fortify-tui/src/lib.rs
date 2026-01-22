//! Fortify TUI - Terminal User Interface for Deployment and Management
//!
//! A full-screen terminal application providing:
//! - Deployment wizard (New / Resume / Join)
//! - Live log streaming
//! - Configuration management with hot-reload
//! - Branding and settings customization

mod app;
mod config;
mod deployment;
mod events;
mod logging;
mod mirror_health;
mod settings;
mod ui;
mod widgets;

pub use app::App;
pub use config::{BrandingConfig, CaptchaConfig, FortifyConfig, ThresholdConfig};
pub use deployment::{DeploymentManager, DeploymentState};
pub use logging::LogEntry;
pub use mirror_health::{
    MirrorHealth, MirrorHealthChecker, MirrorHealthResult, MirrorHealthSummary, MirrorHealthTracker,
};
