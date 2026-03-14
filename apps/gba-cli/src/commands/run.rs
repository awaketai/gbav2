//! Handler for the `gba run` command.
//!
//! This command executes the implementation for a planned feature,
//! consuming the event stream from `GbaEngine::run()` and displaying
//! progress for each phase.

use std::io::{self, Write};

use anyhow::Result;
use futures::StreamExt;
use gba_core::{GbaConfig, GbaEngine, PromptManager};
use tracing::debug;

use super::display::display_event;

/// Handles the `gba run` command.
///
/// This function:
/// 1. Creates a `GbaEngine` instance
/// 2. Consumes the `GbaEvent` stream from `engine.run()`
/// 3. Displays progress for each phase
/// 4. Shows review and verification results
///
/// # Errors
///
/// Returns an error if execution fails or if terminal output fails.
pub async fn handle_run(feature_id: &str, verbose: bool) -> Result<()> {
    debug!(feature_id = %feature_id, "Starting run command");

    let current_dir = std::env::current_dir()?;
    let config = GbaConfig::load(&current_dir)?;
    let prompt_manager = PromptManager::new(None)?;
    let engine = GbaEngine::new(config, prompt_manager)?;

    run_execute(&engine, feature_id, verbose).await
}

/// Runs the execution process asynchronously.
async fn run_execute(engine: &GbaEngine, feature_id: &str, verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Executing feature: {feature_id}")?;
    writeln!(handle)?;

    let mut stream = engine.run(feature_id);
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
        "\x1b[32mExecution complete! ({event_count} events)\x1b[0m"
    )?;

    Ok(())
}
