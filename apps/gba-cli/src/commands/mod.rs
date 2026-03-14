//! Command handlers for GBA CLI.
//!
//! This module contains the implementation of each CLI subcommand:
//! - `init`: Initialize a repository for GBA
//! - `plan`: Start interactive planning session
//! - `run`: Execute feature implementation

mod display;
mod init;
mod plan;
mod run;

pub use init::handle_init;
pub use plan::handle_plan;
pub use run::handle_run;
