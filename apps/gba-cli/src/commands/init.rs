//! Handler for the `gba init` command.
//!
//! This command initializes the current repository for GBA usage,
//! consuming the event stream from `GbaEngine::init()` and displaying
//! progress to the terminal.

use std::io::{self, Write};

use anyhow::Result;
use futures::StreamExt;
use gba_core::{GbaConfig, GbaEngine, GbaEvent, PromptManager};
use tracing::debug;

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
pub fn handle_init(verbose: bool) -> Result<()> {
    debug!("Starting init command");

    let current_dir = std::env::current_dir()?;
    let config = GbaConfig::new(&current_dir);
    let prompt_manager = PromptManager::new(None)?;
    let engine = GbaEngine::new(config, prompt_manager)?;

    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { run_init(&engine, verbose).await })
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
                "  \x1b[36m[{index}/{total}] Starting: {name}\x1b[0m"
            )?;
        }
        GbaEvent::PhaseCommitted { name } => {
            writeln!(handle, "  \x1b[32mCompleted: {name}\x1b[0m")?;
        }
        GbaEvent::ReviewStarted => {
            writeln!(handle, "  \x1b[35mStarting code review...\x1b[0m")?;
        }
        GbaEvent::IssuesFound(issues) => {
            writeln!(handle, "  \x1b[33mFound {} issue(s):\x1b[0m", issues.len())?;
            for issue in issues {
                writeln!(handle, "    - {issue}")?;
            }
        }
        GbaEvent::FixingIssues => {
            writeln!(handle, "  \x1b[33mFixing issues...\x1b[0m")?;
        }
        GbaEvent::VerificationResult { passed, details } => {
            if *passed {
                writeln!(handle, "  \x1b[32mVerification passed: {details}\x1b[0m")?;
            } else {
                writeln!(handle, "  \x1b[31mVerification failed: {details}\x1b[0m")?;
            }
        }
        GbaEvent::PrCreated { url } => {
            writeln!(handle, "  \x1b[32mPull request created: {url}\x1b[0m")?;
        }
        GbaEvent::Error(msg) => {
            writeln!(handle, "  \x1b[31mError: {msg}\x1b[0m")?;
        }
        // Handle any future event variants
        _ => {
            writeln!(handle, "  {}", event.description())?;
        }
    }

    handle.flush()?;
    Ok(())
}
