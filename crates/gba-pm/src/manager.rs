//! Prompt manager implementation.

use crate::context::PromptContext;
use crate::error::GbaPmError;
use crate::prompt_id::PromptId;
use minijinja::Environment;
use std::fs;
use std::path::Path;
use tracing::debug;

// Embedded default templates
const TEMPLATE_INIT_SYSTEM: &str = include_str!("templates/init_system.jinja");
const TEMPLATE_INIT_ANALYZE: &str = include_str!("templates/init_analyze.jinja");
const TEMPLATE_INIT_GBA_MD: &str = include_str!("templates/init_gba_md.jinja");
const TEMPLATE_INIT_CLAUDE_MD: &str = include_str!("templates/init_claude_md.jinja");
const TEMPLATE_PLAN_SYSTEM: &str = include_str!("templates/plan_system.jinja");
const TEMPLATE_PLAN_DESIGN_SPEC: &str = include_str!("templates/plan_design_spec.jinja");
const TEMPLATE_PLAN_VERIFICATION: &str = include_str!("templates/plan_verification.jinja");
const TEMPLATE_RUN_SYSTEM: &str = include_str!("templates/run_system.jinja");
const TEMPLATE_RUN_PHASE: &str = include_str!("templates/run_phase.jinja");
const TEMPLATE_RUN_REVIEW: &str = include_str!("templates/run_review.jinja");
const TEMPLATE_RUN_VERIFY: &str = include_str!("templates/run_verify.jinja");

/// Manages prompt templates for the GBA system.
///
/// The `PromptManager` loads default templates embedded at compile time and
/// optionally allows users to override them from a custom directory.
///
/// # Examples
///
/// ```
/// use gba_pm::{PromptManager, PromptId, PromptContext};
///
/// # fn main() -> Result<(), gba_pm::GbaPmError> {
/// let manager = PromptManager::new(None)?;
///
/// let ctx = PromptContext::new()
///     .with_working_dir("/path/to/repo")
///     .with_repo_tree("src/\n  main.rs");
///
/// let prompt = manager.render(PromptId::InitSystem, &ctx)?;
/// assert!(prompt.contains("Repository Analyst"));
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PromptManager {
    /// MiniJinja environment with loaded templates.
    env: Environment<'static>,
}

impl PromptManager {
    /// Creates a new `PromptManager` with default templates and optional user overrides.
    ///
    /// Default templates are embedded at compile time. If `override_dir` is provided,
    /// templates from that directory will override the defaults with matching names.
    ///
    /// # Errors
    ///
    /// Returns `GbaPmError::LoadError` if the override directory exists but a template
    /// file cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use gba_pm::PromptManager;
    /// use std::path::Path;
    ///
    /// // Without overrides
    /// let manager = PromptManager::new(None)?;
    ///
    /// // With custom template directory
    /// let manager = PromptManager::new(Some(Path::new(".gba/templates")))?;
    /// # Ok::<(), gba_pm::GbaPmError>(())
    /// ```
    pub fn new(override_dir: Option<&Path>) -> Result<Self, GbaPmError> {
        let mut env = Environment::new();

        // Define all template names and their embedded content
        let default_templates: [(&str, &str); 11] = [
            ("init_system.jinja", TEMPLATE_INIT_SYSTEM),
            ("init_analyze.jinja", TEMPLATE_INIT_ANALYZE),
            ("init_gba_md.jinja", TEMPLATE_INIT_GBA_MD),
            ("init_claude_md.jinja", TEMPLATE_INIT_CLAUDE_MD),
            ("plan_system.jinja", TEMPLATE_PLAN_SYSTEM),
            ("plan_design_spec.jinja", TEMPLATE_PLAN_DESIGN_SPEC),
            ("plan_verification.jinja", TEMPLATE_PLAN_VERIFICATION),
            ("run_system.jinja", TEMPLATE_RUN_SYSTEM),
            ("run_phase.jinja", TEMPLATE_RUN_PHASE),
            ("run_review.jinja", TEMPLATE_RUN_REVIEW),
            ("run_verify.jinja", TEMPLATE_RUN_VERIFY),
        ];

        // Check if override directory exists and load overrides
        let has_overrides = override_dir.is_some_and(|dir| dir.exists());

        if let Some(dir) = override_dir
            && has_overrides
        {
            debug!("Loading template overrides from: {:?}", dir);
        }

        // Load templates: use override if available, otherwise use default
        for (name, default_content) in default_templates {
            let template_content = if has_overrides {
                if let Some(dir) = override_dir {
                    let override_path = dir.join(name);
                    if override_path.exists() {
                        let content = fs::read_to_string(&override_path).map_err(|e| {
                            GbaPmError::LoadError {
                                path: override_path.clone(),
                                source: e,
                            }
                        })?;
                        debug!("Loaded override for template: {}", name);
                        content
                    } else {
                        default_content.to_string()
                    }
                } else {
                    default_content.to_string()
                }
            } else {
                default_content.to_string()
            };

            env.add_template_owned(name.to_string(), template_content)
                .map_err(|e| GbaPmError::RenderError {
                    template: name.to_string(),
                    source: e,
                })?;
        }

        Ok(Self { env })
    }

    /// Renders a prompt template with the given context.
    ///
    /// # Errors
    ///
    /// Returns `GbaPmError::TemplateNotFound` if the template doesn't exist.
    /// Returns `GbaPmError::RenderError` if template rendering fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use gba_pm::{PromptManager, PromptId, PromptContext};
    ///
    /// # fn main() -> Result<(), gba_pm::GbaPmError> {
    /// let manager = PromptManager::new(None)?;
    ///
    /// let ctx = PromptContext::new()
    ///     .with_feature_slug("add-auth")
    ///     .with_phase_name("Implementation")
    ///     .with_phase_description("Add authentication module")
    ///     .with_phase_index(1)
    ///     .with_phase_total(3);
    ///
    /// let prompt = manager.render(PromptId::RunPhase, &ctx)?;
    /// assert!(prompt.contains("Implementation"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn render(&self, id: PromptId, ctx: &PromptContext) -> Result<String, GbaPmError> {
        let template_name = id.template_name();

        let template = self
            .env
            .get_template(template_name)
            .map_err(|e| GbaPmError::TemplateNotFound(format!("{}: {}", template_name, e)))?;

        let json_ctx = serde_json::to_value(ctx).map_err(|e| GbaPmError::RenderError {
            template: template_name.to_string(),
            source: minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, e.to_string()),
        })?;

        template
            .render(&json_ctx)
            .map_err(|e| GbaPmError::RenderError {
                template: template_name.to_string(),
                source: e,
            })
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new(None).expect("Failed to create default PromptManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GbaMdEntry;

    #[test]
    fn test_new_without_overrides() {
        let manager = PromptManager::new(None);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_new_with_nonexistent_override_dir() {
        let manager = PromptManager::new(Some(Path::new("/nonexistent/path")));
        assert!(manager.is_ok());
    }

    #[test]
    fn test_render_init_system() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_working_dir("/path/to/repo")
            .with_repo_tree("src/\n  main.rs");

        let result = manager.render(PromptId::InitSystem, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("Repository Analyst"));
    }

    #[test]
    fn test_render_init_analyze() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_working_dir("/path/to/repo")
            .with_repo_tree("src/\n  main.rs\nCargo.toml");

        let result = manager.render(PromptId::InitAnalyze, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("/path/to/repo"));
        assert!(prompt.contains("src/"));
    }

    #[test]
    fn test_render_init_gba_md() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_directory_path("src/auth")
            .with_directory_analysis("Contains authentication modules");

        let result = manager.render(PromptId::InitGbaMd, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("src/auth"));
        assert!(prompt.contains("authentication modules"));
    }

    #[test]
    fn test_render_init_claude_md() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_working_dir("/path/to/repo")
            .with_gba_md_files(vec![
                GbaMdEntry::new("src/auth/gba.md", "Auth module"),
                GbaMdEntry::new("src/db/gba.md", "Database layer"),
            ]);

        let result = manager.render(PromptId::InitClaudeMd, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("GBA Context"));
        assert!(prompt.contains("src/auth/gba.md"));
        assert!(prompt.contains("Auth module"));
    }

    #[test]
    fn test_render_plan_system() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_working_dir("/path/to/repo")
            .with_repo_tree("src/\n  main.rs");

        let result = manager.render(PromptId::PlanSystem, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("Planning Architect"));
    }

    #[test]
    fn test_render_plan_design_spec() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_feature_id("0001")
            .with_working_dir("/path/to/repo");

        let result = manager.render(PromptId::PlanDesignSpec, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("add-auth"));
        assert!(prompt.contains("0001"));
    }

    #[test]
    fn test_render_plan_verification() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_feature_id("0001");

        let result = manager.render(PromptId::PlanVerification, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("verification plan"));
    }

    #[test]
    fn test_render_run_system() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_working_dir("/path/to/repo")
            .with_design_spec("# Design\n\nAdd authentication")
            .with_verification_plan("# Verification\n\nTest auth flow")
            .with_repo_tree("src/\n  auth.rs");

        let result = manager.render(PromptId::RunSystem, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("Coding Agent"));
        assert!(prompt.contains("add-auth"));
        assert!(prompt.contains("Add authentication"));
    }

    #[test]
    fn test_render_run_phase() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_working_dir("/path/to/repo")
            .with_design_spec("# Design\n\nAdd auth")
            .with_phase_name("Core Implementation")
            .with_phase_description("Implement authentication logic")
            .with_phase_index(1)
            .with_phase_total(3);

        let result = manager.render(PromptId::RunPhase, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("Phase 1 of 3"));
        assert!(prompt.contains("Core Implementation"));
        assert!(prompt.contains("authentication logic"));
    }

    #[test]
    fn test_render_run_review() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_working_dir("/path/to/repo");

        let result = manager.render(PromptId::RunReview, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("Review"));
        assert!(prompt.contains("add-auth"));
    }

    #[test]
    fn test_render_run_verify() {
        let manager = PromptManager::new(None).expect("Failed to create manager");

        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_feature_id("0001")
            .with_working_dir("/path/to/repo")
            .with_verification_plan("# Verification\n\n1. Test login\n2. Test logout")
            .with_review_issues(vec!["Missing error handling".to_string()]);

        let result = manager.render(PromptId::RunVerify, &ctx);
        assert!(result.is_ok());
        let prompt = result.expect("Failed to render");
        assert!(prompt.contains("verification plan"));
        assert!(prompt.contains("add-auth"));
        assert!(prompt.contains("0001"));
        assert!(prompt.contains("Missing error handling"));
    }

    #[test]
    fn test_default_implementation() {
        let manager = PromptManager::default();
        let ctx = PromptContext::new().with_working_dir("/test");

        let result = manager.render(PromptId::InitSystem, &ctx);
        assert!(result.is_ok());
    }
}
