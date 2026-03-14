//! # GBA Core - Core Execution Engine
//!
//! This crate provides the core execution engine for the GBA (Git-Based Agent) CLI,
//! built on top of the Claude Agent SDK. It orchestrates all GBA operations,
//! combining prompt rendering (gba-pm) with Claude Agent SDK calls.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                          gba-core                               │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌──────────────┐   │
//! │  │ GbaConfig│  │ AgentPreset│ │ AgentSession│ │   Workspace  │   │
//! │  └──────────┘  └──────────┘  └─────────────┘  └──────────────┘   │
//! │                                                                  │
//! │  ┌──────────┐  ┌───────────┐  ┌────────────────────────────┐    │
//! │  │ GbaEvent │  │ GbaCoreError│ │       (Future: GbaEngine)   │    │
//! │  └──────────┘  └───────────┘  └────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────┘
//!           │                    │                    │
//!           ▼                    ▼                    ▼
//!     ┌──────────┐      ┌───────────────┐     ┌─────────────────┐
//!     │  gba-pm  │      │ claude-agent  │     │   filesystem    │
//!     │ (prompts)│      │    -sdk-rs    │     │    (.gba/)      │
//!     └──────────┘      └───────────────┘     └─────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! ### AgentPreset (Security Boundary)
//!
//! Agent tool permissions are hardcoded in the engine as `AgentPreset` variants.
//! This is a security boundary - users cannot configure tool permissions.
//!
//! - **ReadOnly**: Read, Glob, Grep - for analysis and review
//! - **WriteSpec**: Read, Glob, Grep, Write - for spec file generation
//! - **FullCoding**: All tools - for implementation phases
//! - **Verify**: Read, Glob, Grep, Bash - for verification without file modification
//!
//! ### SessionConfig (User Tunable)
//!
//! Session parameters like model and max_turns can be configured via `.gba/config.yaml`.
//!
//! ### Workspace
//!
//! The workspace manages the `.gba/` directory structure including specs and trees.
//!
//! ## Usage
//!
//! ### Creating an Agent Session
//!
//! ```no_run
//! use gba_core::{AgentSession, AgentPreset, SessionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), gba_core::GbaCoreError> {
//! let config = SessionConfig::default();
//! let mut session = AgentSession::new(
//!     AgentPreset::ReadOnly,
//!     &config,
//!     "You are a code reviewer.".to_string(),
//! )?;
//!
//! session.connect().await?;
//! let response = session.send_and_collect("Review this code.").await?;
//! session.disconnect().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Managing Workspace
//!
//! ```no_run
//! use gba_core::Workspace;
//!
//! let workspace = Workspace::new("/path/to/repo");
//! workspace.initialize().expect("Failed to initialize");
//!
//! let feature_id = workspace.create_feature("add-auth").expect("Failed to create feature");
//! workspace.write_design_spec(&feature_id, "add-auth", "# Design\n...").expect("Failed to write");
//! ```

mod config;
mod engine;
mod error;
mod event;
mod git;
mod plan;
mod preset;
mod reviewer;
mod runner;
mod session;
mod verifier;
mod workspace;

// Re-export public API
pub use config::{GbaConfig, SessionConfig, SessionsConfig};
pub use engine::GbaEngine;
pub use error::GbaCoreError;
pub use event::GbaEvent;
pub use git::GitOps;
pub use plan::PlanSession;
pub use preset::AgentPreset;
pub use reviewer::{ReviewIssue, ReviewResult, Reviewer};
pub use runner::{Phase, Runner};
pub use session::{AgentMessage, AgentSession};
pub use verifier::{VerificationResult, Verifier};
pub use workspace::Workspace;

// Re-export gba-pm types that users commonly need
pub use gba_pm::{GbaMdEntry, PromptContext, PromptId, PromptManager, PromptRole};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_exports() {
        // Verify all public types are accessible
        let _config = GbaConfig::new("/test");
        let _session_config = SessionConfig::default();
        let _sessions_config = SessionsConfig::default();
        let _preset = AgentPreset::ReadOnly;
        let _event = GbaEvent::WaitingForInput;
        let _workspace = Workspace::new("/test");

        // Verify gba-pm re-exports
        let _ctx = PromptContext::default();
        let _entry = GbaMdEntry::default();
        let _role = PromptRole::System;
        let _id = PromptId::InitSystem;
    }

    #[test]
    fn test_agent_preset_allowed_tools() {
        assert_eq!(
            AgentPreset::ReadOnly.allowed_tools(),
            vec!["Read", "Glob", "Grep"]
        );
        assert_eq!(
            AgentPreset::WriteSpec.allowed_tools(),
            vec!["Read", "Glob", "Grep", "Write"]
        );
        assert_eq!(
            AgentPreset::FullCoding.allowed_tools(),
            vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep"]
        );
        assert_eq!(
            AgentPreset::Verify.allowed_tools(),
            vec!["Read", "Glob", "Grep", "Bash"]
        );
    }

    #[test]
    fn test_session_config_default_values() {
        let config = SessionConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_turns, 10);
    }

    #[test]
    fn test_sessions_config_default_values() {
        let config = SessionsConfig::default();
        assert_eq!(config.init.max_turns, 3);
        assert_eq!(config.plan.max_turns, 30);
        assert_eq!(config.run_phase.max_turns, 20);
        assert_eq!(config.run_review.max_turns, 5);
        assert_eq!(config.run_verify.max_turns, 10);
    }

    #[test]
    fn test_gba_config_paths() {
        let config = GbaConfig::new("/repo");
        assert_eq!(config.gba_dir(), std::path::PathBuf::from("/repo/.gba"));
        assert_eq!(
            config.specs_dir(),
            std::path::PathBuf::from("/repo/.gba/specs")
        );
    }

    #[test]
    fn test_gba_event_description() {
        let event = GbaEvent::PhaseStarted {
            name: String::from("Test"),
            index: 1,
            total: 3,
        };
        assert_eq!(event.description(), "Starting phase 1/3: Test");

        let event = GbaEvent::VerificationResult {
            passed: true,
            details: String::from("All tests passed"),
        };
        assert_eq!(event.description(), "Verification passed: All tests passed");
    }

    #[test]
    fn test_workspace_paths() {
        let ws = Workspace::new("/repo");
        assert_eq!(ws.root(), std::path::Path::new("/repo"));
        assert_eq!(ws.gba_dir(), std::path::PathBuf::from("/repo/.gba"));
    }
}
