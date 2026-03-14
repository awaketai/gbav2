//! GBA CLI - Command line interface for GBA.
//!
//! A CLI tool that wraps the GBA Core engine, allowing users to
//! conveniently initialize repositories, plan features, and run implementations.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod tui;

/// GBA - Geektime Bootcamp Agent CLI
#[derive(Debug, Parser)]
#[command(name = "gba", version, about, long_about = None)]
struct Cli {
    /// Turn on verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize the current repository for GBA usage.
    ///
    /// This command:
    /// - Creates the .gba/ directory structure
    /// - Analyzes the repository structure
    /// - Generates gba.md files for important directories
    /// - Updates CLAUDE.md with GBA context
    Init,

    /// Start an interactive planning session for a feature.
    ///
    /// This command:
    /// - Starts a multi-round dialogue with the agent
    /// - Helps design the feature specification
    /// - Generates design.md and verification.md files
    ///
    /// Use the feature slug as a short identifier (e.g., "add-auth", "fix-login-bug").
    Plan {
        /// Feature slug identifier (e.g., "add-auth")
        feature_slug: String,
    },

    /// Execute the implementation for a planned feature.
    ///
    /// This command:
    /// - Runs all implementation phases
    /// - Commits after each phase
    /// - Reviews generated code
    /// - Fixes any issues found
    /// - Verifies implementation
    /// - Creates a pull request
    Run {
        /// Feature ID (e.g., "0001")
        feature_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Dispatch to command handler
    match cli.command {
        Commands::Init => commands::handle_init(cli.verbose).await,
        Commands::Plan { feature_slug } => commands::handle_plan(&feature_slug, cli.verbose).await,
        Commands::Run { feature_id } => commands::handle_run(&feature_id, cli.verbose).await,
    }
}
