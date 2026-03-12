//! TUI components for GBA CLI.
//!
//! This module provides the ratatui-based terminal user interface for
//! the interactive planning session.

mod app;
mod event;
mod ui;

pub use app::App;
pub use ui::run_app;
