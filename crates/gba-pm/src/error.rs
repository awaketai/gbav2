//! Error types for the gba-pm crate.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during prompt management operations.
#[derive(Debug, Error)]
pub enum GbaPmError {
    /// The requested template was not found.
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// An error occurred during template rendering.
    #[error("Failed to render template '{template}': {source}")]
    RenderError {
        /// The name of the template that failed to render.
        template: String,
        /// The underlying minijinja error.
        #[source]
        source: minijinja::Error,
    },

    /// An error occurred while loading a template file.
    #[error("Failed to load template from '{path}': {source}")]
    LoadError {
        /// The path to the file that failed to load.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}
