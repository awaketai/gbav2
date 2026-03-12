//! Handler for the `gba run` command.
//!
//! This command executes the implementation for a planned feature,
//! consuming the event stream from `GbaEngine::run()` and displaying
//! progress for each phase.

use std::io::{self, Write};

use anyhow::Result;
use futures::StreamExt;
use gba_core::{GbaConfig, GbaEngine, GbaEvent, PromptManager};
use tracing::debug;

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
pub fn handle_run(feature_id: &str, verbose: bool) -> Result<()> {
    debug!(feature_id = %feature_id, "Starting run command");

    let current_dir = std::env::current_dir()?;
    let config = GbaConfig::new(&current_dir);
    let prompt_manager = PromptManager::new(None)?;
    let engine = GbaEngine::new(config, prompt_manager)?;

    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { run_execute(&engine, feature_id, verbose).await })
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

/// Displays a single event to the terminal.
fn display_event(handle: &mut io::StdoutLock<'_>, event: &GbaEvent, verbose: bool) -> Result<()> {
    match event {
        GbaEvent::AssistantMessage(msg) => {
            // Truncate long messages unless verbose
            if verbose {
                writeln!(handle, "  {msg}")?;
            } else {
                let truncated = if msg.len() > 100 {
                    format!("{}...", &msg[..97])
                } else {
                    msg.clone()
                };
                writeln!(handle, "  {truncated}")?;
            }
        }
        GbaEvent::WaitingForInput => {
            writeln!(handle, "  \x1b[33mWaiting for input...\x1b[0m")?;
        }
        GbaEvent::PhaseStarted { name, index, total } => {
            writeln!(
                handle,
                "\n\x1b[36m--- Phase {index}/{total}: {name} ---\x1b[0m"
            )?;
        }
        GbaEvent::PhaseCommitted { name } => {
            writeln!(handle, "  [OK] Committed: {name}")?;
        }
        GbaEvent::ReviewStarted => {
            writeln!(handle, "\n\x1b[35m--- Code Review ---\x1b[0m")?;
        }
        GbaEvent::IssuesFound(issues) => {
            writeln!(handle, "  [!] Found {} issue(s):", issues.len())?;
            for issue in issues {
                writeln!(handle, "    - {issue}")?;
            }
        }
        GbaEvent::FixingIssues => {
            writeln!(handle, "  [*] Fixing issues...")?;
        }
        GbaEvent::VerificationResult { passed, details } => {
            writeln!(handle, "\n\x1b[34m--- Verification ---\x1b[0m")?;
            if *passed {
                writeln!(handle, "  [OK] Passed: {details}")?;
            } else {
                writeln!(handle, "  [X] Failed: {details}")?;
            }
        }
        GbaEvent::PrCreated { url } => {
            writeln!(handle, "\n\x1b[32m--- Pull Request ---\x1b[0m")?;
            writeln!(handle, "  [OK] Created: {url}")?;
        }
        GbaEvent::Error(msg) => {
            writeln!(handle, "  [X] Error: {msg}")?;
        }
        // Handle any future event variants
        _ => {
            writeln!(handle, "  {}", event.description())?;
        }
    }

    handle.flush()?;
    Ok(())
}
