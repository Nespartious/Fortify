//! Main entry point for Fortify TUI

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use fortify_tui::App;

/// Get the logs directory path (persistent)
fn get_logs_dir() -> std::path::PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let mut path = std::path::PathBuf::from(home);
        path.push(".local");
        path.push("share");
        path.push("fortify");
        path.push("logs");
        path
    } else {
        std::path::PathBuf::from("/tmp/fortify/logs")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file-based logging (TUI takes over stdout)
    let logs_dir = get_logs_dir();
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_file = std::fs::File::create(logs_dir.join("tui.log")).ok();
    if let Some(file) = log_file {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::sync::Mutex::new(file)))
            .with(EnvFilter::from_default_env().add_directive("fortify=debug".parse()?))
            .init();
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new().await?;
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print any error after terminal restore
    if let Err(ref e) = result {
        eprintln!("Error: {e:?}");
    }

    result
}
