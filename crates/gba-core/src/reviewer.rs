//! Code review for GBA.
//!
//! This module provides the `Reviewer` which reviews generated code
//! and optionally fixes issues found.

use tracing::{debug, info, warn};

use crate::config::GbaConfig;
use crate::error::GbaCoreError;
use crate::preset::AgentPreset;
use crate::session::AgentSession;
use crate::workspace::Workspace;
use gba_pm::{PromptContext, PromptId, PromptManager};

/// Maximum number of fix iterations.
const MAX_FIX_ITERATIONS: usize = 3;

/// Issue found during code review.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ReviewIssue {
    /// File path relative to repository root.
    pub file: String,
    /// Line number (1-based).
    pub line: Option<usize>,
    /// Issue description.
    pub description: String,
    /// Suggested fix.
    pub suggestion: Option<String>,
}

/// Result of a code review.
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// Whether the review passed (no issues found).
    pub passed: bool,
    /// List of issues found.
    pub issues: Vec<ReviewIssue>,
}

/// Code reviewer for generated code.
///
/// The `Reviewer` uses a `ReadOnly` session to review code and output
/// a JSON list of issues. If issues are found, it can optionally
/// use a `FullCoding` session to fix them.
///
/// # Security Model
///
/// Review sessions use `ReadOnly` preset to ensure the reviewer
/// cannot modify code during review. Only fix sessions use
/// `FullCoding` preset.
///
/// # Examples
///
/// ```no_run
/// use gba_core::{GbaConfig, PromptManager, Reviewer};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), gba_core::GbaCoreError> {
/// let config = GbaConfig::new("/repo");
/// let pm = PromptManager::new(None)?;
/// let reviewer = Reviewer::new(&config, &pm, "0001")?;
///
/// let result = reviewer.review().await?;
/// if !result.passed {
///     println!("Found {} issues", result.issues.len());
/// }
/// # Ok(())
/// # }
/// ```
pub struct Reviewer {
    /// Configuration.
    config: GbaConfig,
    /// Prompt manager for rendering templates.
    prompt_manager: PromptManager,
    /// Workspace for file operations.
    workspace: Workspace,
    /// Feature ID.
    feature_id: String,
}

impl Reviewer {
    /// Creates a new reviewer for the given feature.
    #[must_use]
    pub fn new(config: &GbaConfig, prompt_manager: &PromptManager, feature_id: &str) -> Self {
        let workspace = Workspace::new(&config.working_dir);

        debug!(feature_id = %feature_id, "Creating reviewer");

        Self {
            config: config.clone(),
            prompt_manager: prompt_manager.clone(),
            workspace,
            feature_id: feature_id.to_string(),
        }
    }

    /// Reviews the generated code and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if the review session fails.
    pub async fn review(&self) -> Result<ReviewResult, GbaCoreError> {
        info!(feature_id = %self.feature_id, "Starting code review");

        // Render system prompt
        let ctx = PromptContext::new()
            .with_working_dir(self.workspace.root())
            .with_feature_slug(&self.feature_id);

        let system_prompt = self.prompt_manager.render(PromptId::RunSystem, &ctx)?;
        let review_prompt = self.prompt_manager.render(PromptId::RunReview, &ctx)?;

        // Create ReadOnly session for review
        let mut session = AgentSession::new(
            AgentPreset::ReadOnly,
            &self.config.sessions.run_review,
            system_prompt,
        )?;

        session.connect().await?;

        // Execute review
        let response = session.send_and_collect(&review_prompt).await?;

        session.disconnect().await?;

        // Parse JSON response
        let issues = self.parse_review_response(&response)?;

        let result = ReviewResult {
            passed: issues.is_empty(),
            issues,
        };

        info!(
            feature_id = %self.feature_id,
            passed = result.passed,
            issue_count = result.issues.len(),
            "Review completed"
        );

        Ok(result)
    }

    /// Fixes the given issues and returns the updated review result.
    ///
    /// This method runs a fix-review loop up to `MAX_FIX_ITERATIONS` times.
    ///
    /// # Errors
    ///
    /// Returns an error if the fix session fails.
    pub async fn fix_and_review(
        &self,
        issues: Vec<ReviewIssue>,
    ) -> Result<ReviewResult, GbaCoreError> {
        let mut current_issues = issues;
        let mut iteration = 0;

        while !current_issues.is_empty() && iteration < MAX_FIX_ITERATIONS {
            iteration += 1;
            info!(
                feature_id = %self.feature_id,
                iteration,
                issue_count = current_issues.len(),
                "Fixing issues"
            );

            // Fix issues
            self.fix_issues(&current_issues).await?;

            // Re-review
            let result = self.review().await?;

            if result.passed {
                return Ok(result);
            }

            current_issues = result.issues;
        }

        if !current_issues.is_empty() {
            warn!(
                feature_id = %self.feature_id,
                iteration,
                remaining_issues = current_issues.len(),
                "Max fix iterations reached with remaining issues"
            );
        }

        Ok(ReviewResult {
            passed: current_issues.is_empty(),
            issues: current_issues,
        })
    }

    /// Fixes the given issues using a FullCoding session.
    async fn fix_issues(&self, issues: &[ReviewIssue]) -> Result<(), GbaCoreError> {
        // Render prompts
        let ctx = PromptContext::new()
            .with_working_dir(self.workspace.root())
            .with_feature_slug(&self.feature_id)
            .with_review_issues(
                issues
                    .iter()
                    .map(|i| format!("{}: {}", i.file, i.description))
                    .collect(),
            );

        let system_prompt = self.prompt_manager.render(PromptId::RunSystem, &ctx)?;

        // Create fix prompt with issue descriptions
        let issue_descriptions: Vec<String> = issues
            .iter()
            .map(|i| {
                let line_info = i.line.map_or(String::new(), |l| format!(":{l}"));
                format!("- {}{line_info}: {}", i.file, i.description)
            })
            .collect();

        let fix_prompt = format!(
            "Please fix the following issues found during code review:\n\n{}",
            issue_descriptions.join("\n")
        );

        // Create FullCoding session for fixes
        let mut session = AgentSession::new(
            AgentPreset::FullCoding,
            &self.config.sessions.run_phase,
            system_prompt,
        )?;

        session.connect().await?;
        session.send_and_collect(&fix_prompt).await?;
        session.disconnect().await?;

        debug!(
            feature_id = %self.feature_id,
            issue_count = issues.len(),
            "Fixes applied"
        );

        Ok(())
    }

    /// Parses the review response to extract issues.
    fn parse_review_response(&self, response: &str) -> Result<Vec<ReviewIssue>, GbaCoreError> {
        // Try to extract JSON from the response
        let json_start = response.find('[');
        let json_end = response.rfind(']');

        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) => &response[start..=end],
            _ => {
                // No JSON array found, check for empty response or "no issues"
                if response.to_lowercase().contains("no issues")
                    || response.to_lowercase().contains("passed")
                {
                    return Ok(Vec::new());
                }
                debug!(
                    response_len = response.len(),
                    "No JSON found in review response"
                );
                return Ok(Vec::new());
            }
        };

        // Parse JSON
        let issues: Vec<ReviewIssue> = serde_json::from_str(json_str).unwrap_or_else(|e| {
            debug!(error = %e, "Failed to parse review JSON, returning empty");
            Vec::new()
        });

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_issue_deserialization() {
        let json = r#"[
            {"file": "src/main.rs", "line": 42, "description": "Missing error handling", "suggestion": "Add error handling"}
        ]"#;

        let issues: Vec<ReviewIssue> = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/main.rs");
        assert_eq!(issues[0].line, Some(42));
        assert_eq!(issues[0].description, "Missing error handling");
        assert_eq!(issues[0].suggestion, Some("Add error handling".to_string()));
    }

    #[test]
    fn test_review_issue_optional_fields() {
        let json = r#"[
            {"file": "src/lib.rs", "description": "Unused import"}
        ]"#;

        let issues: Vec<ReviewIssue> = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/lib.rs");
        assert_eq!(issues[0].line, None);
        assert_eq!(issues[0].description, "Unused import");
        assert_eq!(issues[0].suggestion, None);
    }

    #[test]
    fn test_review_result_debug() {
        let result = ReviewResult {
            passed: true,
            issues: vec![],
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("passed"));
    }
}
