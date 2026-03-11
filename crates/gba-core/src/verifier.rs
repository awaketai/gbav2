//! Verification for GBA.
//!
//! This module provides the `Verifier` which runs verification plans
//! and returns pass/fail results.

use tracing::{debug, info, warn};

use crate::config::GbaConfig;
use crate::error::GbaCoreError;
use crate::preset::AgentPreset;
use crate::session::AgentSession;
use crate::workspace::Workspace;
use gba_pm::{PromptContext, PromptId, PromptManager};

/// Verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    /// Whether verification passed.
    pub passed: bool,
    /// Detailed output from verification.
    pub details: String,
    /// Summary of what was verified.
    pub summary: Option<String>,
}

/// Verifier for running verification plans.
///
/// The `Verifier` uses a `Verify` preset session to execute the
/// verification plan without modifying files.
///
/// # Security Model
///
/// Verifier sessions use `Verify` preset which allows running
/// commands (cargo build/test/clippy) but not modifying files.
/// This ensures verification cannot accidentally or maliciously
/// modify the codebase.
///
/// # Examples
///
/// ```no_run
/// use gba_core::{GbaConfig, PromptManager, Verifier};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), gba_core::GbaCoreError> {
/// let config = GbaConfig::new("/repo");
/// let pm = PromptManager::new(None)?;
/// let verifier = Verifier::new(&config, &pm, "0001")?;
///
/// let result = verifier.verify().await?;
/// if result.passed {
///     println!("Verification passed: {}", result.details);
/// } else {
///     println!("Verification failed: {}", result.details);
/// }
/// # Ok(())
/// # }
/// ```
pub struct Verifier {
    /// Configuration.
    config: GbaConfig,
    /// Prompt manager for rendering templates.
    prompt_manager: PromptManager,
    /// Workspace for file operations.
    workspace: Workspace,
    /// Feature ID.
    feature_id: String,
    /// Review issues (if any) to include in verification context.
    review_issues: Vec<String>,
}

impl Verifier {
    /// Creates a new verifier for the given feature.
    #[must_use]
    pub fn new(config: &GbaConfig, prompt_manager: &PromptManager, feature_id: &str) -> Self {
        let workspace = Workspace::new(&config.working_dir);

        debug!(feature_id = %feature_id, "Creating verifier");

        Self {
            config: config.clone(),
            prompt_manager: prompt_manager.clone(),
            workspace,
            feature_id: feature_id.to_string(),
            review_issues: Vec::new(),
        }
    }

    /// Sets the review issues to include in verification context.
    #[must_use]
    pub fn with_review_issues(mut self, issues: Vec<String>) -> Self {
        self.review_issues = issues;
        self
    }

    /// Runs the verification plan and returns the result.
    ///
    /// # Errors
    ///
    /// Returns an error if the verification session fails.
    pub async fn verify(&self) -> Result<VerificationResult, GbaCoreError> {
        info!(feature_id = %self.feature_id, "Starting verification");

        // Read verification plan
        let verification_plan = self.workspace.read_verification(&self.feature_id)?;

        // Render prompts
        let ctx = PromptContext::new()
            .with_working_dir(self.workspace.root())
            .with_feature_slug(&self.feature_id)
            .with_feature_id(&self.feature_id)
            .with_verification_plan(&verification_plan)
            .with_review_issues(self.review_issues.clone());

        let system_prompt = self.prompt_manager.render(PromptId::RunSystem, &ctx)?;
        let verify_prompt = self.prompt_manager.render(PromptId::RunVerify, &ctx)?;

        // Create Verify session
        let mut session = AgentSession::new(
            AgentPreset::Verify,
            &self.config.sessions.run_verify,
            system_prompt,
        )?;

        session.connect().await?;

        // Execute verification
        let response = session.send_and_collect(&verify_prompt).await?;

        session.disconnect().await?;

        // Parse response
        let result = self.parse_verification_response(&response)?;

        info!(
            feature_id = %self.feature_id,
            passed = result.passed,
            "Verification completed"
        );

        Ok(result)
    }

    /// Parses the verification response to extract the result.
    fn parse_verification_response(
        &self,
        response: &str,
    ) -> Result<VerificationResult, GbaCoreError> {
        // Try to extract JSON from the response
        let json_start = response.find('{');
        let json_end = response.rfind('}');

        let json_str = match (json_start, json_end) {
            (Some(start), Some(end)) => &response[start..=end],
            _ => {
                // No JSON found, parse as text
                return self.parse_text_response(response);
            }
        };

        // Try to parse as JSON
        #[derive(serde::Deserialize)]
        struct VerificationJson {
            passed: Option<bool>,
            status: Option<String>,
            details: Option<String>,
            summary: Option<String>,
        }

        if let Ok(json) = serde_json::from_str::<VerificationJson>(json_str) {
            let passed = json.passed.unwrap_or_else(|| {
                json.status.as_ref().is_some_and(|s| {
                    s.to_lowercase() == "passed" || s.to_lowercase() == "success"
                })
            });

            return Ok(VerificationResult {
                passed,
                details: json.details.unwrap_or_else(|| response.to_string()),
                summary: json.summary,
            });
        }

        // Fall back to text parsing
        self.parse_text_response(response)
    }

    /// Parses a text response to determine verification result.
    fn parse_text_response(&self, response: &str) -> Result<VerificationResult, GbaCoreError> {
        let lower = response.to_lowercase();

        // Check for pass indicators
        let passed = lower.contains("passed")
            || lower.contains("success")
            || lower.contains("all tests passed")
            || lower.contains("verification complete")
            || (lower.contains("build") && lower.contains("succeeded"));

        // Check for fail indicators
        let failed =
            lower.contains("failed") || lower.contains("error") || lower.contains("tests failed");

        // Determine final status
        let final_passed = passed && !failed;

        if !final_passed && !failed {
            // Ambiguous response, default to checking for common patterns
            warn!(
                response_len = response.len(),
                "Ambiguous verification response"
            );
        }

        Ok(VerificationResult {
            passed: final_passed,
            details: response.to_string(),
            summary: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_debug() {
        let result = VerificationResult {
            passed: true,
            details: "All tests passed".to_string(),
            summary: Some("OK".to_string()),
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("passed"));
    }

    #[test]
    fn test_verification_result_equality() {
        let result1 = VerificationResult {
            passed: true,
            details: "OK".to_string(),
            summary: None,
        };
        let result2 = VerificationResult {
            passed: true,
            details: "OK".to_string(),
            summary: None,
        };
        assert_eq!(result1, result2);
    }
}
