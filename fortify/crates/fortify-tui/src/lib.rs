//! Fortify TUI - Terminal User Interface for Deployment and Management
//!
//! A full-screen terminal application providing:
//! - Deployment wizard (New / Resume / Join)
//! - Live log streaming
//! - Configuration management with hot-reload
//! - Branding and settings customization

mod app;
mod config;
mod controller;
mod deployment;
mod events;
mod logging;
mod mirror_health;
mod settings;
mod status;
mod ui;
mod verification;
mod widgets;

pub use app::App;
pub use config::{BrandingConfig, CaptchaConfig, FortifyConfig, ThresholdConfig};
pub use controller::{
    ControllerClient, ControllerConfig, ControllerHealth, ServiceSnapshot, ServiceStatus,
    ServiceType,
};
pub use deployment::{DeploymentManager, DeploymentState};
pub use logging::LogEntry;
pub use mirror_health::{
    MirrorHealth, MirrorHealthChecker, MirrorHealthResult, MirrorHealthSummary, MirrorHealthTracker,
};
pub use status::{
    MirrorStatus, NodeStatus, OrchestratorStatusResponse, StatusMessage, StatusPoller,
    StatusPollerConfig, StatusPollerHandle, SystemStatus, start_status_polling,
};
pub use verification::{OnionVerifier, VerificationConfig, VerificationResult};
