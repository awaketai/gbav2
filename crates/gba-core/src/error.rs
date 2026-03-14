//! Error types for the gba-core crate.

use thiserror::Error;

/// Errors that can occur during GBA core operations.
#[derive(Debug, Error)]
pub enum GbaCoreError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Workspace management error.
    #[error("Workspace error: {0}")]
    WorkspaceError(#[source] std::io::Error),

    /// Session management error.
    #[error("Session error: {0}")]
    SessionError(String),

    /// Planning error.
    #[error("Planning error: {0}")]
    PlanError(String),

    /// Execution error.
    #[error("Run error: {0}")]
    RunError(String),

    /// Review error.
    #[error("Review error: {0}")]
    ReviewError(String),

    /// Prompt manager error.
    #[error("Prompt error: {0}")]
    PromptError(#[source] gba_pm::GbaPmError),

    /// Claude Agent SDK error.
    #[error("Claude agent error: {0}")]
    ClaudeError(#[source] claude_agent_sdk_rs::ClaudeError),

    /// JSON serialization error.
    #[error("JSON serialization error: {0}")]
    SerializationError(#[source] serde_json::Error),

    /// YAML serialization error.
    #[error("YAML serialization error: {0}")]
    YamlError(String),
}

impl From<gba_pm::GbaPmError> for GbaCoreError {
    fn from(err: gba_pm::GbaPmError) -> Self {
        Self::PromptError(err)
    }
}

impl From<claude_agent_sdk_rs::ClaudeError> for GbaCoreError {
    fn from(err: claude_agent_sdk_rs::ClaudeError) -> Self {
        Self::ClaudeError(err)
    }
}

impl From<serde_json::Error> for GbaCoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err)
    }
}

impl From<std::io::Error> for GbaCoreError {
    fn from(err: std::io::Error) -> Self {
        Self::WorkspaceError(err)
    }
}

impl From<serde_yaml::Error> for GbaCoreError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::YamlError(err.to_string())
    }
}
