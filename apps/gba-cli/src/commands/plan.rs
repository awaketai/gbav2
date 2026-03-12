//! Handler for the `gba plan` command.
//!
//! This command starts an interactive planning session using a ratatui TUI.
//! It handles user input and agent responses in a chat-style interface.

use std::io;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use gba_core::{GbaConfig, GbaEngine, PromptManager};
use ratatui::{Terminal, backend::CrosstermBackend};
use tracing::debug;

use crate::tui::{App, run_app};

/// Handles the `gba plan` command.
///
/// This function:
/// 1. Creates a `GbaEngine` instance
/// 2. Starts a planning session via `engine.plan()`
/// 3. Launches the ratatui TUI for interactive dialogue
/// 4. Finalizes and generates specs when complete
///
/// # Errors
///
/// Returns an error if planning fails or if terminal setup fails.
pub fn handle_plan(feature_slug: &str, verbose: bool) -> Result<()> {
    debug!(feature_slug = %feature_slug, "Starting plan command");

    let current_dir = std::env::current_dir()?;
    let config = GbaConfig::new(&current_dir);
    let prompt_manager = PromptManager::new(None)?;
    let engine = GbaEngine::new(config, prompt_manager)?;

    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { run_plan(&engine, feature_slug, verbose).await })
}

/// Runs the planning process asynchronously.
async fn run_plan(engine: &GbaEngine, feature_slug: &str, verbose: bool) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let app = App::new(feature_slug.to_string(), verbose);
    let result = run_app(&mut terminal, app, engine).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle result
    match result {
        Ok(Some(feature_id)) => {
            println!("\x1b[32mPlanning complete! Feature ID: {feature_id}\x1b[0m");
            println!("Run `gba run {feature_id}` to start implementation.");
        }
        Ok(None) => {
            println!("\x1b[33mPlanning cancelled.\x1b[0m");
        }
        Err(e) => {
            println!("\x1b[31mError: {e}\x1b[0m");
            return Err(e);
        }
    }

    Ok(())
}
