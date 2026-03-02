//! GBA CLI - Command line interface for GBA
//!
//! A CLI tool that wraps the Claude Agent SDK, allowing users to
//! conveniently add new features around a repository.

use anyhow::Result;
use clap::Parser;

/// GBA - Geektime Bootcamp Agent CLI
#[derive(Debug, Parser)]
#[command(name = "gba", version, about, long_about = None)]
struct Cli {
    /// Turn on verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("GBA CLI starting...");

    Ok(())
}
