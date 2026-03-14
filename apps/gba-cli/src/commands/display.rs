//! Display utilities for GBA CLI.
//!
//! This module provides common display functions for rendering events
//! to the terminal, used by both `init` and `run` commands.

use std::io::{self, Write};

use anyhow::Result;
use gba_core::GbaEvent;

/// Displays a single event to the terminal.
///
/// # Arguments
///
/// * `handle` - The stdout handle to write to
/// * `event` - The event to display
/// * `verbose` - Whether to show full details (vs truncated output)
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
pub fn display_event(
    handle: &mut io::StdoutLock<'_>,
    event: &GbaEvent,
    verbose: bool,
) -> Result<()> {
    match event {
        GbaEvent::AssistantMessage(msg) => {
            display_assistant_message(handle, msg, verbose)?;
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
            writeln!(handle, "  \x1b[32m[OK] Committed: {name}\x1b[0m")?;
        }
        GbaEvent::ReviewStarted => {
            writeln!(handle, "\n\x1b[35m--- Code Review ---\x1b[0m")?;
        }
        GbaEvent::IssuesFound(issues) => {
            writeln!(
                handle,
                "  \x1b[33m[!] Found {} issue(s):\x1b[0m",
                issues.len()
            )?;
            for issue in issues {
                writeln!(handle, "    - {issue}")?;
            }
        }
        GbaEvent::FixingIssues => {
            writeln!(handle, "  \x1b[33m[*] Fixing issues...\x1b[0m")?;
        }
        GbaEvent::VerificationResult { passed, details } => {
            writeln!(handle, "\n\x1b[34m--- Verification ---\x1b[0m")?;
            if *passed {
                writeln!(handle, "  \x1b[32m[OK] Passed: {details}\x1b[0m")?;
            } else {
                writeln!(handle, "  \x1b[31m[X] Failed: {details}\x1b[0m")?;
            }
        }
        GbaEvent::PrCreated { url } => {
            writeln!(handle, "\n\x1b[32m--- Pull Request ---\x1b[0m")?;
            writeln!(handle, "  \x1b[32m[OK] Created: {url}\x1b[0m")?;
        }
        GbaEvent::Error(msg) => {
            writeln!(handle, "  \x1b[31m[X] Error: {msg}\x1b[0m")?;
        }
        // Handle any future event variants
        _ => {
            writeln!(handle, "  {}", event.description())?;
        }
    }

    handle.flush()?;
    Ok(())
}

/// Displays an assistant message, truncating if not in verbose mode.
fn display_assistant_message(
    handle: &mut io::StdoutLock<'_>,
    msg: &str,
    verbose: bool,
) -> Result<()> {
    if verbose {
        writeln!(handle, "  {msg}")?;
    } else {
        let truncated = if msg.len() > 100 {
            format!("{}...", &msg[..97])
        } else {
            msg.to_string()
        };
        writeln!(handle, "  {truncated}")?;
    }
    Ok(())
}
