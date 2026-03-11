//! # GBA Prompt Manager
//!
//! This crate provides prompt management capabilities for the GBA (Git-Based Agent) system.
//! It handles loading, managing, and rendering Jinja templates for various GBA operations.
//!
//! ## Overview
//!
//! The GBA system uses structured prompts to guide AI agents through different operations:
//!
//! - **`gba init`**: Initialize a repository with GBA documentation
//! - **`gba plan`**: Plan a feature through multi-round dialogue
//! - **`gba run`**: Execute the implementation phases
//!
//! Each operation uses a combination of System prompts (defining agent identity and rules)
//! and User prompts (containing specific task instructions).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │  PromptManager  │────▶│  MiniJinja Env   │────▶│    Templates    │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//!         │                                                │
//!         │                                                │
//!         ▼                                                ▼
//! ┌─────────────────┐                           ┌──────────────────┐
//! │  PromptContext  │                           │ Embedded/Override│
//! └─────────────────┘                           └──────────────────┘
//! ```
//!
//! ## Usage
//!
//! ### Basic Usage
//!
//! ```rust
//! use gba_pm::{PromptManager, PromptId, PromptContext};
//!
//! # fn main() -> Result<(), gba_pm::GbaPmError> {
//! // Create a prompt manager with default embedded templates
//! let manager = PromptManager::new(None)?;
//!
//! // Create context with relevant data
//! let ctx = PromptContext::new()
//!     .with_working_dir("/path/to/repo")
//!     .with_repo_tree("src/\n  main.rs\nCargo.toml");
//!
//! // Render a prompt
//! let system_prompt = manager.render(PromptId::InitSystem, &ctx)?;
//! let analyze_prompt = manager.render(PromptId::InitAnalyze, &ctx)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### With Custom Templates
//!
//! ```rust
//! use gba_pm::{PromptManager, PromptId, PromptContext};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), gba_pm::GbaPmError> {
//! // Create manager with custom template overrides
//! let manager = PromptManager::new(Some(Path::new(".gba/templates")))?;
//!
//! // Templates in .gba/templates/ will override embedded defaults
//! let ctx = PromptContext::new().with_working_dir("/repo");
//! let prompt = manager.render(PromptId::InitSystem, &ctx)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Prompt Roles
//!
//! Each prompt has a role that determines how it's used:
//!
//! - **System prompts** (`PromptRole::System`): Define agent identity and rules
//! - **User prompts** (`PromptRole::User`): Contain specific task instructions
//!
//! ```rust
//! use gba_pm::{PromptId, PromptRole};
//!
//! assert_eq!(PromptId::InitSystem.role(), PromptRole::System);
//! assert_eq!(PromptId::InitAnalyze.role(), PromptRole::User);
//! ```
//!
//! ## Feature Flags
//!
//! This crate has no optional feature flags.

mod context;
mod error;
mod manager;
mod prompt_id;

// Re-export public API
pub use context::{GbaMdEntry, PromptContext};
pub use error::GbaPmError;
pub use manager::PromptManager;
pub use prompt_id::{PromptId, PromptRole};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_exports() {
        // Verify all public types are accessible
        let _ctx = PromptContext::default();
        let _entry = GbaMdEntry::default();
        let _role = PromptRole::System;
        let _id = PromptId::InitSystem;

        // PromptManager creation
        let manager = PromptManager::new(None);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_all_prompt_ids_have_templates() {
        let manager = PromptManager::new(None).expect("Failed to create manager");
        let ctx = PromptContext::default();

        // Verify all PromptId variants can be rendered
        let ids = [
            PromptId::InitSystem,
            PromptId::InitAnalyze,
            PromptId::InitGbaMd,
            PromptId::InitClaudeMd,
            PromptId::PlanSystem,
            PromptId::PlanDesignSpec,
            PromptId::PlanVerification,
            PromptId::RunSystem,
            PromptId::RunPhase,
            PromptId::RunReview,
            PromptId::RunVerify,
        ];

        for id in ids {
            let result = manager.render(id, &ctx);
            assert!(result.is_ok(), "Failed to render {:?}: {:?}", id, result);
        }
    }

    #[test]
    fn test_prompt_roles() {
        // System prompts
        assert_eq!(PromptId::InitSystem.role(), PromptRole::System);
        assert_eq!(PromptId::PlanSystem.role(), PromptRole::System);
        assert_eq!(PromptId::RunSystem.role(), PromptRole::System);

        // User prompts
        assert_eq!(PromptId::InitAnalyze.role(), PromptRole::User);
        assert_eq!(PromptId::InitGbaMd.role(), PromptRole::User);
        assert_eq!(PromptId::InitClaudeMd.role(), PromptRole::User);
        assert_eq!(PromptId::PlanDesignSpec.role(), PromptRole::User);
        assert_eq!(PromptId::PlanVerification.role(), PromptRole::User);
        assert_eq!(PromptId::RunPhase.role(), PromptRole::User);
        assert_eq!(PromptId::RunReview.role(), PromptRole::User);
        assert_eq!(PromptId::RunVerify.role(), PromptRole::User);
    }

    #[test]
    fn test_context_serialization() {
        let ctx = PromptContext::new()
            .with_feature_slug("test-feature")
            .with_feature_id("0042")
            .with_phase_index(2)
            .with_phase_total(5)
            .with_gba_md_files(vec![
                GbaMdEntry::new("src/gba.md", "Source"),
                GbaMdEntry::new("tests/gba.md", "Tests"),
            ]);

        // Serialize to JSON
        let json = serde_json::to_string(&ctx).expect("Failed to serialize");

        // Verify JSON contains expected fields
        assert!(json.contains("test-feature"));
        assert!(json.contains("0042"));
        assert!(json.contains("phase_index"));
        assert!(json.contains("phase_total"));

        // Deserialize back
        let deserialized: PromptContext =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.feature_slug, "test-feature");
        assert_eq!(deserialized.feature_id, "0042");
        assert_eq!(deserialized.phase_index, 2);
        assert_eq!(deserialized.phase_total, 5);
        assert_eq!(deserialized.gba_md_files.len(), 2);
    }

    #[test]
    fn test_error_types() {
        // Verify error types are accessible and display correctly
        let err = GbaPmError::TemplateNotFound("test.jinja".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test.jinja"));

        let err = GbaPmError::RenderError {
            template: "test.jinja".to_string(),
            source: minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, "test error"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("test.jinja"));
    }
}
