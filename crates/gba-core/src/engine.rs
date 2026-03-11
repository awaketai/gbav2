//! Main GBA engine orchestrating all operations.
//!
//! This module provides `GbaEngine` which orchestrates all GBA operations
//! including initialization, planning, and execution.

use std::pin::Pin;
use std::sync::Arc;

use futures::stream::Stream;
use tracing::{debug, info, instrument};

use crate::config::GbaConfig;
use crate::error::GbaCoreError;
use crate::event::GbaEvent;
use crate::git::GitOps;
use crate::plan::PlanSession;
use crate::preset::AgentPreset;
use crate::reviewer::Reviewer;
use crate::runner::Runner;
use crate::session::AgentSession;
use crate::verifier::Verifier;
use crate::workspace::Workspace;
use gba_pm::{GbaMdEntry, PromptContext, PromptId, PromptManager};

/// The main GBA engine orchestrating all operations.
///
/// `GbaEngine` is the primary entry point for all GBA operations. It combines
/// prompt management with agent sessions to implement the full GBA workflow.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │                          GbaEngine                               │
/// ├─────────────────────────────────────────────────────────────────┤
/// │  ┌──────────┐  ┌──────────────┐  ┌───────────┐  ┌───────────┐  │
/// │  │ GbaConfig│  │PromptManager │  │ Workspace │  │  GitOps   │  │
/// │  └──────────┘  └──────────────┘  └───────────┘  └───────────┘  │
/// │                                                                  │
/// │  ┌──────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────┐  │
/// │  │   init() │  │  plan()   │  │   run()   │  │ PlanSession   │  │
/// │  └──────────┘  └───────────┘  └───────────┘  └───────────────┘  │
/// └─────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Examples
///
/// ```no_run
/// use gba_core::{GbaConfig, GbaEngine, PromptManager};
/// use futures::StreamExt;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), gba_core::GbaCoreError> {
/// let config = GbaConfig::new("/path/to/repo");
/// let pm = PromptManager::new(None)?;
/// let engine = GbaEngine::new(config, pm)?;
///
/// // Initialize repository
/// let mut stream = engine.init();
/// while let Some(event) = stream.next().await {
///     println!("{:?}", event?);
/// }
///
/// // Plan a feature
/// let mut plan = engine.plan("add-auth").await?;
/// plan.send("I want OAuth support").await?;
/// plan.finalize().await?;
///
/// // Run implementation
/// let mut stream = engine.run("0001").await?;
/// while let Some(event) = stream.next().await {
///     println!("{:?}", event?);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct GbaEngine {
    /// Configuration.
    config: GbaConfig,
    /// Prompt manager for rendering templates.
    prompt_manager: Arc<PromptManager>,
    /// Workspace for file operations.
    workspace: Workspace,
    /// Git operations handler.
    git: GitOps,
}

impl GbaEngine {
    /// Creates a new `GbaEngine` with the given configuration and prompt manager.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace cannot be initialized.
    pub fn new(config: GbaConfig, prompt_manager: PromptManager) -> Result<Self, GbaCoreError> {
        let workspace = Workspace::new(&config.working_dir);
        let git = GitOps::new(&config.working_dir);
        let prompt_manager = Arc::new(prompt_manager);

        debug!(
            working_dir = %config.working_dir.display(),
            "Creating GbaEngine"
        );

        Ok(Self {
            config,
            prompt_manager,
            workspace,
            git,
        })
    }

    /// Creates a new `GbaEngine` with default prompt manager.
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt manager or workspace cannot be initialized.
    pub fn with_defaults(config: GbaConfig) -> Result<Self, GbaCoreError> {
        let prompt_manager = PromptManager::new(None)?;
        Self::new(config, prompt_manager)
    }

    /// Initializes the repository for GBA usage.
    ///
    /// This operation:
    /// 1. Creates the `.gba/` directory structure
    /// 2. Analyzes the repository structure (ReadOnly session)
    /// 3. Generates `gba.md` files for important directories (WriteSpec sessions)
    /// 4. Updates `CLAUDE.md` with GBA context (WriteSpec session)
    ///
    /// # Errors
    ///
    /// Errors are yielded through the stream rather than returned directly.
    #[instrument(skip(self))]
    pub fn init(&self) -> Pin<Box<dyn Stream<Item = Result<GbaEvent, GbaCoreError>> + Send + '_>> {
        Box::pin(async_stream::stream! {
            info!("Starting repository initialization");

            // Initialize workspace
            self.workspace.initialize()?;

            // Get repo tree
            let repo_tree = self.get_repo_tree()?;

            // Session 1: Analyze repository (ReadOnly)
            let analyze_ctx = PromptContext::new()
                .with_working_dir(&self.config.working_dir)
                .with_repo_tree(&repo_tree);

            let system_prompt = self.prompt_manager.render(PromptId::InitSystem, &analyze_ctx)?;
            let analyze_prompt = self.prompt_manager.render(PromptId::InitAnalyze, &analyze_ctx)?;

            let mut analyze_session = AgentSession::new(
                AgentPreset::ReadOnly,
                &self.config.sessions.init,
                system_prompt.clone(),
            )?;

            analyze_session.connect().await?;

            yield Ok(GbaEvent::AssistantMessage(
                "Analyzing repository structure...".to_string()
            ));

            let analysis_result = analyze_session.send_and_collect(&analyze_prompt).await?;
            analyze_session.disconnect().await?;

            // Parse analysis to get important directories
            let directories = self.parse_analysis_directories(&analysis_result);

            // Sessions 2..N: Generate gba.md for each directory (WriteSpec)
            let mut gba_md_files: Vec<GbaMdEntry> = Vec::new();

            for dir in directories {
                let dir_ctx = PromptContext::new()
                    .with_working_dir(&self.config.working_dir)
                    .with_directory_path(&dir.path)
                    .with_directory_analysis(&dir.analysis);

                let gba_md_prompt = self.prompt_manager.render(PromptId::InitGbaMd, &dir_ctx)?;

                let mut gba_session = AgentSession::new(
                    AgentPreset::WriteSpec,
                    &self.config.sessions.init,
                    system_prompt.clone(),
                )?;

                gba_session.connect().await?;

                yield Ok(GbaEvent::AssistantMessage(
                    format!("Generating gba.md for {}...", dir.path)
                ));

                gba_session.send_and_collect(&gba_md_prompt).await?;
                gba_session.disconnect().await?;

                gba_md_files.push(GbaMdEntry::new(
                    format!("{}/gba.md", dir.path),
                    dir.summary,
                ));
            }

            // Session N+1: Update CLAUDE.md (WriteSpec)
            let claude_ctx = PromptContext::new()
                .with_working_dir(&self.config.working_dir)
                .with_gba_md_files(gba_md_files);

            let claude_prompt = self.prompt_manager.render(PromptId::InitClaudeMd, &claude_ctx)?;

            let mut claude_session = AgentSession::new(
                AgentPreset::WriteSpec,
                &self.config.sessions.init,
                system_prompt,
            )?;

            claude_session.connect().await?;

            yield Ok(GbaEvent::AssistantMessage(
                "Updating CLAUDE.md with GBA context...".to_string()
            ));

            claude_session.send_and_collect(&claude_prompt).await?;
            claude_session.disconnect().await?;

            yield Ok(GbaEvent::AssistantMessage(
                "Repository initialized successfully!".to_string()
            ));

            info!("Repository initialization completed");
        })
    }

    /// Starts a planning session for a feature.
    ///
    /// Returns a `PlanSession` that supports multi-round dialogue
    /// and finalization to generate spec files.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be created.
    #[instrument(skip(self))]
    pub async fn plan(&self, feature_slug: &str) -> Result<PlanSession, GbaCoreError> {
        info!(feature_slug = %feature_slug, "Starting plan session");
        PlanSession::new(&self.config, Arc::clone(&self.prompt_manager), feature_slug).await
    }

    /// Executes the implementation for a feature.
    ///
    /// This operation:
    /// 1. Runs all implementation phases (FullCoding sessions)
    /// 2. Commits after each phase
    /// 3. Reviews generated code (ReadOnly session)
    /// 4. Fixes any issues found (FullCoding sessions)
    /// 5. Verifies implementation (Verify session)
    /// 6. Creates a pull request (FullCoding session)
    ///
    /// # Errors
    ///
    /// Errors are yielded through the stream rather than returned directly.
    #[instrument(skip(self))]
    pub fn run(
        &self,
        feature_id: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<GbaEvent, GbaCoreError>> + Send + '_>> {
        let feature_id = feature_id.to_string();

        Box::pin(async_stream::stream! {
            info!(feature_id = %feature_id, "Starting feature execution");

            // Step 1-5: Run phases
            let runner = Runner::new(&self.config, Arc::clone(&self.prompt_manager), &feature_id)?;

            let phase_stream = runner.run();
            use futures::StreamExt;
            let mut phase_stream = phase_stream;

            while let Some(event) = phase_stream.next().await {
                yield event;
            }

            // Step 6: Review
            yield Ok(GbaEvent::ReviewStarted);

            let reviewer = Reviewer::new(&self.config, &self.prompt_manager, &feature_id);
            let review_result = reviewer.review().await?;

            if !review_result.passed {
                let issue_descriptions: Vec<String> = review_result
                    .issues
                    .iter()
                    .map(|i| format!("{}: {}", i.file, i.description))
                    .collect();

                yield Ok(GbaEvent::IssuesFound(issue_descriptions.clone()));

                // Step 7: Fix loop
                yield Ok(GbaEvent::FixingIssues);

                let fix_result = reviewer.fix_and_review(review_result.issues).await?;

                if !fix_result.passed {
                    yield Ok(GbaEvent::Error(format!(
                        "Could not fix all issues: {:?}",
                        fix_result.issues
                    )));
                    return;
                }
            }

            // Step 8: Verify
            let verifier = Verifier::new(&self.config, &self.prompt_manager, &feature_id);
            let verify_result = verifier.verify().await?;

            yield Ok(GbaEvent::VerificationResult {
                passed: verify_result.passed,
                details: verify_result.details,
            });

            if !verify_result.passed {
                yield Ok(GbaEvent::Error(
                    "Verification failed. Please fix the issues and run again.".to_string()
                ));
                return;
            }

            // Step 9: Create PR
            let design_spec = self.workspace.read_design_spec(&feature_id)?;
            let pr_title = format!("feat: {} ({})", feature_id, feature_id);
            let pr_body = format!(
                "## Summary\n\nImplementation of feature {}.\n\n## Design\n\n{}",
                feature_id, design_spec
            );

            match self.git.create_pr(&pr_title, &pr_body) {
                Ok(url) => {
                    yield Ok(GbaEvent::PrCreated { url });
                }
                Err(e) => {
                    yield Ok(GbaEvent::Error(format!("Failed to create PR: {}", e)));
                }
            }

            info!(feature_id = %feature_id, "Feature execution completed");
        })
    }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &GbaConfig {
        &self.config
    }

    /// Returns the workspace.
    #[must_use]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Gets the repository tree as a string.
    fn get_repo_tree(&self) -> Result<String, GbaCoreError> {
        let output = std::process::Command::new("find")
            .args([".", "-type", "f", "-not", "-path", "./.git/*"])
            .current_dir(self.workspace.root())
            .output()
            .map_err(|e| GbaCoreError::RunError(format!("Failed to get repo tree: {e}")))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Parses the analysis result to extract important directories.
    fn parse_analysis_directories(&self, analysis: &str) -> Vec<DirectoryInfo> {
        // Try to parse as JSON first
        if let Some(json_start) = analysis.find('[')
            && let Some(json_end) = analysis.rfind(']')
        {
            let json_str = &analysis[json_start..=json_end];
            if let Ok(dirs) = serde_json::from_str::<Vec<DirectoryInfo>>(json_str) {
                return dirs;
            }
        }

        // Fall back to text parsing
        let mut directories = Vec::new();
        for line in analysis.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                let path = trimmed[2..].trim();
                if !path.is_empty() {
                    directories.push(DirectoryInfo {
                        path: path.to_string(),
                        analysis: String::new(),
                        summary: path.to_string(),
                    });
                }
            }
        }

        // If no directories found, add common ones
        if directories.is_empty() {
            for dir in &["src", "crates", "apps", "lib"] {
                let path = self.workspace.root().join(dir);
                if path.exists() && path.is_dir() {
                    directories.push(DirectoryInfo {
                        path: dir.to_string(),
                        analysis: String::new(),
                        summary: dir.to_string(),
                    });
                }
            }
        }

        directories
    }
}

/// Information about a directory for gba.md generation.
#[derive(Debug, Clone, serde::Deserialize)]
struct DirectoryInfo {
    /// Directory path relative to repository root.
    path: String,
    /// Analysis context for the directory.
    #[serde(default)]
    analysis: String,
    /// One-line summary of the directory's purpose.
    #[serde(default)]
    summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_info_deserialization() {
        let json = r#"[
            {"path": "src/auth", "analysis": "Auth module", "summary": "Authentication"},
            {"path": "src/db", "analysis": "", "summary": "Database"}
        ]"#;

        let dirs: Vec<DirectoryInfo> = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].path, "src/auth");
        assert_eq!(dirs[1].summary, "Database");
    }

    #[test]
    fn test_directory_info_default_fields() {
        let json = r#"[
            {"path": "src/core"}
        ]"#;

        let dirs: Vec<DirectoryInfo> = serde_json::from_str(json).expect("Failed to parse");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].path, "src/core");
        assert_eq!(dirs[0].analysis, "");
        assert_eq!(dirs[0].summary, "");
    }
}
