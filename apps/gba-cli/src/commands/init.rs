//! Handler for the `gba init` command.
//!
//! This command initializes the current repository for GBA usage,
//! consuming the event stream from `GbaEngine::init()` and displaying
//! progress to the terminal.

use std::io::{self, Write};

use anyhow::Result;
use futures::StreamExt;
use gba_core::{GbaConfig, GbaEngine, PromptManager};
use tracing::debug;

use super::display::display_event;

/// Handles the `gba init` command.
///
/// This function:
/// 1. Creates a `GbaEngine` instance
/// 2. Consumes the `GbaEvent` stream from `engine.init()`
/// 3. Outputs events to the terminal with progress display
///
/// # Errors
///
/// Returns an error if initialization fails or if terminal output fails.
pub async fn handle_init(verbose: bool) -> Result<()> {
    debug!("Starting init command");

    let current_dir = std::env::current_dir()?;
    let config = GbaConfig::load(&current_dir)?;
    let prompt_manager = PromptManager::new(None)?;
    let engine = GbaEngine::new(config, prompt_manager)?;

    run_init(&engine, verbose).await
}

/// Runs the initialization process asynchronously.
async fn run_init(engine: &GbaEngine, verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Initializing GBA repository...")?;
    writeln!(handle)?;

    let mut stream = engine.init();
    let mut event_count = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                event_count += 1;
                display_event(&mut handle, &event, verbose)?;
            }
            Err(e) => {
                writeln!(handle)?;
                writeln!(handle, "\x1b[31mError: {e}\x1b[0m")?;
                return Err(e.into());
            }
        }
    }

    writeln!(handle)?;
    writeln!(
        handle,
        "\x1b[32mInitialization complete! ({event_count} events)\x1b[0m"
    )?;

    Ok(())
}
