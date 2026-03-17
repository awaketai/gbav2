//! Staged execution runner for GBA.
//!
//! This module provides the `Runner` which executes implementation phases
//! parsed from a design specification.

use std::pin::Pin;
use std::sync::Arc;

use futures::stream::Stream;
use tracing::{debug, info};

use crate::config::GbaConfig;
use crate::error::GbaCoreError;
use crate::event::GbaEvent;
use crate::git::GitOps;
use crate::preset::AgentPreset;
use crate::session::{AgentMessage, AgentSession};
use crate::workspace::Workspace;
use gba_pm::{PromptContext, PromptId, PromptManager};

/// A single phase extracted from a design specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    /// Phase name (e.g., "Core Implementation").
    pub name: String,
    /// Phase description with implementation details.
    pub description: String,
}

/// Parses phases from a design specification markdown document.
///
/// Phases are expected to be in the format:
/// ```markdown
/// ## Phase 1: Phase Name
/// Phase description...
///
/// ## Phase 2: Next Phase Name
/// ...
/// ```
fn parse_phases(design_spec: &str) -> Vec<Phase> {
    let mut phases = Vec::new();
    let mut current_name = String::new();
    let mut current_description = String::new();

    for line in design_spec.lines() {
        let trimmed = line.trim();

        // Check for phase header
        if trimmed.starts_with("## Phase")
            || trimmed.starts_with("## phase")
            || trimmed.starts_with("## PHASE")
        {
            // Save previous phase if exists
            if !current_name.is_empty() {
                phases.push(Phase {
                    name: current_name.clone(),
                    description: current_description.trim().to_string(),
                });
            }

            // Extract phase name from header
            // Format: "## Phase N: Phase Name" or "## Phase N - Phase Name"
            let header = trimmed.trim_start_matches('#').trim();
            if let Some(colon_pos) = header.find(':') {
                current_name = header[colon_pos + 1..].trim().to_string();
            } else if let Some(dash_pos) = header.find('-') {
                current_name = header[dash_pos + 1..].trim().to_string();
            } else {
                // Just use the whole header as name
                current_name = header.to_string();
            }
            current_description = String::new();
        } else if !current_name.is_empty() {
            // Accumulate description
            if !current_description.is_empty() {
                current_description.push('\n');
            }
            current_description.push_str(line);
        }
    }

    // Save last phase
    if !current_name.is_empty() {
        phases.push(Phase {
            name: current_name.clone(),
            description: current_description.trim().to_string(),
        });
    }

    // If no phases found, create a single default phase
    if phases.is_empty() {
        phases.push(Phase {
            name: String::from("Implementation"),
            description: String::from(
                "Implement the feature according to the design specification.",
            ),
        });
    }

    phases
}

/// Staged execution runner for implementing features.
///
/// The `Runner` executes implementation phases one by one, committing
/// after each phase completes.
///
/// # Examples
///
/// ```no_run
/// use gba_core::{GbaConfig, PromptManager, Runner};
/// use std::sync::Arc;
/// use futures::StreamExt;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), gba_core::GbaCoreError> {
/// let config = GbaConfig::new("/repo");
/// let pm = Arc::new(PromptManager::new(None)?);
/// let runner = Runner::new(&config, pm.clone(), "0001")?;
///
/// let mut stream = runner.run();
/// while let Some(event) = stream.next().await {
///     match event? {
///         gba_core::GbaEvent::PhaseStarted { name, index, total } => {
///             println!("Phase {}/{}: {}", index, total, name);
///         }
///         gba_core::GbaEvent::PhaseCommitted { name } => {
///             println!("Completed: {}", name);
///         }
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct Runner {
    /// Configuration.
    config: GbaConfig,
    /// Prompt manager for rendering templates.
    prompt_manager: Arc<PromptManager>,
    /// Workspace for file operations.
    workspace: Workspace,
    /// Git operations handler.
    git: GitOps,
    /// Feature ID.
    feature_id: String,
    /// Design specification content.
    design_spec: String,
    /// Verification plan content.
    verification_plan: String,
    /// Parsed phases.
    phases: Vec<Phase>,
}

impl Runner {
    /// Creates a new runner for the given feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the design spec cannot be read.
    pub fn new(
        config: &GbaConfig,
        prompt_manager: Arc<PromptManager>,
        feature_id: &str,
    ) -> Result<Self, GbaCoreError> {
        let workspace = Workspace::new(&config.working_dir);
        let git = GitOps::new(&config.working_dir);

        // Read design spec and verification plan
        let design_spec = workspace.read_design_spec(feature_id)?;
        let verification_plan = workspace.read_verification(feature_id)?;

        // Parse phases from design spec
        let phases = parse_phases(&design_spec);

        debug!(
            feature_id = %feature_id,
            phase_count = phases.len(),
            "Creating runner"
        );

        Ok(Self {
            config: config.clone(),
            prompt_manager,
            workspace,
            git,
            feature_id: feature_id.to_string(),
            design_spec,
            verification_plan,
            phases,
        })
    }

    /// Returns the parsed phases.
    #[must_use]
    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }

    /// Returns the total number of phases.
    #[must_use]
    pub fn total_phases(&self) -> usize {
        self.phases.len()
    }

    /// Executes all phases and returns a stream of events.
    ///
    /// # Errors
    ///
    /// Errors are yielded through the stream rather than returned directly.
    pub fn run(self) -> Pin<Box<dyn Stream<Item = Result<GbaEvent, GbaCoreError>> + Send>> {
        Box::pin(async_stream::stream! {
            // Extract all needed values before moving self.phases
            let repo_tree = self.get_repo_tree().unwrap_or_default();
            let working_dir = self.workspace.root().to_path_buf();
            let feature_id = self.feature_id.clone();
            let design_spec = self.design_spec.clone();
            let verification_plan = self.verification_plan.clone();
            let prompt_manager = Arc::clone(&self.prompt_manager);
            let session_config = self.config.sessions.run_phase.clone();
            let cli_path = self.config.cli_path.clone();
            let git = self.git.clone();
            let total = self.phases.len();
            let phases = self.phases;

            // Render system prompt once
            let ctx = PromptContext::new()
                .with_working_dir(&working_dir)
                .with_feature_slug(&feature_id)
                .with_design_spec(&design_spec)
                .with_verification_plan(&verification_plan)
                .with_repo_tree(&repo_tree);

            let system_prompt = prompt_manager.render(PromptId::RunSystem, &ctx)?;

            for (index, phase) in phases.into_iter().enumerate() {
                let phase_index = index + 1;

                // Emit phase started event
                yield Ok(GbaEvent::PhaseStarted {
                    name: phase.name.clone(),
                    index: phase_index,
                    total,
                });

                info!(
                    phase_name = %phase.name,
                    phase_index,
                    total,
                    "Starting phase"
                );

                // Create session for this phase
                let phase_ctx = ctx.clone()
                    .with_phase_name(&phase.name)
                    .with_phase_description(&phase.description)
                    .with_phase_index(phase_index)
                    .with_phase_total(total);

                let user_prompt = prompt_manager.render(PromptId::RunPhase, &phase_ctx)?;

                let mut session = AgentSession::new(
                    AgentPreset::FullCoding,
                    &session_config,
                    system_prompt.clone(),
                    cli_path.clone(),
                )?;

                session.connect().await?;

                // Execute phase
                let stream_result = execute_phase(&mut session, &user_prompt).await;

                // Ensure session is disconnected
                let disconnect_result = session.disconnect().await;

                // Check for errors
                if let Err(e) = stream_result {
                    yield Err(e);
                    return;
                }
                if let Err(e) = disconnect_result {
                    yield Err(e);
                    return;
                }

                // Commit changes
                let commit_message = format!("feat({}): {} - Phase {}/{}",
                    feature_id, phase.name, phase_index, total);

                if let Err(e) = git.commit_phase(&commit_message) {
                    yield Err(e);
                    return;
                }

                yield Ok(GbaEvent::PhaseCommitted {
                    name: phase.name,
                });
            }
        })
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
}

/// Executes a single phase and streams events.
async fn execute_phase(session: &mut AgentSession, user_prompt: &str) -> Result<(), GbaCoreError> {
    let stream = session.send(user_prompt).await?;

    use futures::StreamExt;
    let mut inner = stream;

    while let Some(result) = inner.next().await {
        match result {
            Ok(AgentMessage::Text(text)) => {
                // Yield assistant message
                debug!(text_len = text.len(), "Phase message");
            }
            Ok(AgentMessage::Completed { .. }) => {
                break;
            }
            Ok(AgentMessage::ToolUse { name, input }) => {
                debug!(tool = %name, input = ?input, "Tool use in phase");
            }
            Ok(AgentMessage::ToolResult { name, output }) => {
                debug!(tool = %name, output_len = output.len(), "Tool result in phase");
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_phases_empty() {
        let phases = parse_phases("");
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].name, "Implementation");
    }

    #[test]
    fn test_parse_phases_single() {
        let spec = r#"## Phase 1: Core Implementation

This is the core implementation phase.
It involves setting up the basic structure.
"#;
        let phases = parse_phases(spec);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].name, "Core Implementation");
        assert!(phases[0].description.contains("core implementation phase"));
    }

    #[test]
    fn test_parse_phases_multiple() {
        let spec = r#"## Phase 1: Setup

Set up the project structure.

## Phase 2: Core Logic

Implement the main logic.

## Phase 3: Tests

Add test coverage.
"#;
        let phases = parse_phases(spec);
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0].name, "Setup");
        assert_eq!(phases[1].name, "Core Logic");
        assert_eq!(phases[2].name, "Tests");
    }

    #[test]
    fn test_parse_phases_with_dash() {
        let spec = r#"## Phase 1 - Authentication

Add auth module.
"#;
        let phases = parse_phases(spec);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].name, "Authentication");
    }

    #[test]
    fn test_phase_debug() {
        let phase = Phase {
            name: "Test".to_string(),
            description: "Test description".to_string(),
        };
        let debug_str = format!("{:?}", phase);
        assert!(debug_str.contains("Test"));
    }
}
