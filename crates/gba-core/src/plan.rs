//! Planning session for GBA.
//!
//! This module provides `PlanSession` which manages multi-round planning dialogue
//! with the agent, upgrading from ReadOnly to WriteSpec when finalizing.

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use futures::stream::Stream;
use tracing::{debug, info};

use crate::config::SessionConfig;
use crate::error::GbaCoreError;
use crate::event::GbaEvent;
use crate::preset::AgentPreset;
use crate::session::{AgentMessage, AgentSession};
use crate::workspace::Workspace;
use gba_pm::{PromptContext, PromptId, PromptManager};

/// Planning session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanState {
    /// Session is in multi-round dialogue mode (ReadOnly).
    Dialogue,
    /// Session is finalizing specs (WriteSpec).
    Finalizing,
    /// Session has completed.
    Completed,
}

/// Planning session for multi-round feature design.
///
/// This struct manages a multi-turn conversation with the agent to design
/// a feature. During dialogue, it uses a `ReadOnly` session. When `finalize()`
/// is called, it creates a new `WriteSpec` sub-session to generate the
/// design.md and verification.md files.
///
/// # Security Model
///
/// The planning dialogue uses `ReadOnly` preset to ensure no files can be
/// modified during the conversation. Only when `finalize()` is explicitly
/// called does a new session with `WriteSpec` preset get created for
/// spec file generation.
///
/// # Examples
///
/// ```no_run
/// use gba_core::{GbaConfig, PromptManager, PlanSession};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), gba_core::GbaCoreError> {
/// let config = GbaConfig::new("/repo");
/// let pm = Arc::new(PromptManager::new(None)?);
/// let mut session = PlanSession::new(&config, pm.clone(), "add-auth").await?;
///
/// // Multi-round dialogue
/// let response = session.send("I want to add OAuth support").await?;
/// let response = session.send("Use JWT tokens for session management").await?;
///
/// // Generate specs
/// session.finalize().await?;
/// # Ok(())
/// # }
/// ```
pub struct PlanSession {
    /// The underlying agent session for dialogue (ReadOnly).
    dialogue_session: AgentSession,
    /// Prompt manager for rendering templates.
    prompt_manager: Arc<PromptManager>,
    /// Workspace for file operations.
    workspace: Workspace,
    /// Feature slug identifier.
    feature_slug: String,
    /// Feature ID (assigned when finalize is called).
    feature_id: Option<String>,
    /// Current session state.
    state: PlanState,
    /// Session configuration for finalizing.
    finalize_config: SessionConfig,
    /// Path to the Claude CLI executable.
    cli_path: Option<PathBuf>,
}

impl PlanSession {
    /// Creates a new planning session for the given feature slug.
    ///
    /// This creates a `ReadOnly` session for multi-round dialogue.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be created.
    pub async fn new(
        config: &crate::config::GbaConfig,
        prompt_manager: Arc<PromptManager>,
        feature_slug: &str,
    ) -> Result<Self, GbaCoreError> {
        debug!(feature_slug = %feature_slug, "Creating plan session");

        let workspace = Workspace::new(&config.working_dir);

        // Render system prompt for planning
        let ctx = PromptContext::new().with_working_dir(&config.working_dir);
        let system_prompt = prompt_manager.render(PromptId::PlanSystem, &ctx)?;

        // Create ReadOnly session for dialogue
        let dialogue_session = AgentSession::new(
            AgentPreset::ReadOnly,
            &config.sessions.plan,
            system_prompt,
            config.cli_path.clone(),
        )?;

        let mut session = Self {
            dialogue_session,
            prompt_manager,
            workspace,
            feature_slug: feature_slug.to_string(),
            feature_id: None,
            state: PlanState::Dialogue,
            finalize_config: config.sessions.plan.clone(),
            cli_path: config.cli_path.clone(),
        };

        // Connect the session
        session.dialogue_session.connect().await?;

        Ok(session)
    }

    /// Sends a user message and returns the assistant's response.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if the session
    /// is not in dialogue mode.
    pub async fn send(&mut self, message: &str) -> Result<String, GbaCoreError> {
        if self.state != PlanState::Dialogue {
            return Err(GbaCoreError::PlanError(
                "Cannot send messages after finalization".to_string(),
            ));
        }

        debug!(
            message_len = message.len(),
            "Sending message in plan session"
        );
        let response = self.dialogue_session.send_and_collect(message).await?;
        Ok(response)
    }

    /// Sends a user message and returns a stream of events.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or if the session
    /// is not in dialogue mode.
    pub async fn send_stream(
        &mut self,
        message: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GbaEvent, GbaCoreError>> + Send + '_>>, GbaCoreError>
    {
        if self.state != PlanState::Dialogue {
            return Err(GbaCoreError::PlanError(
                "Cannot send messages after finalization".to_string(),
            ));
        }

        debug!(
            message_len = message.len(),
            "Sending message stream in plan session"
        );
        let stream = self.dialogue_session.send(message).await?;

        Ok(Box::pin(async_stream::stream! {
            use futures::StreamExt;
            let mut inner = stream;

            while let Some(result) = inner.next().await {
                match result {
                    Ok(AgentMessage::Text(text)) => {
                        yield Ok(GbaEvent::AssistantMessage(text));
                    }
                    Ok(AgentMessage::Completed { .. }) => {
                        yield Ok(GbaEvent::WaitingForInput);
                        break;
                    }
                    Ok(AgentMessage::ToolUse { name, input }) => {
                        debug!(tool = %name, input = ?input, "Tool use in plan session");
                    }
                    Ok(AgentMessage::ToolResult { name, output }) => {
                        debug!(tool = %name, output_len = output.len(), "Tool result in plan session");
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        }))
    }

    /// Finalizes the planning session by generating spec files.
    ///
    /// This creates a new `WriteSpec` sub-session to generate:
    /// 1. `.gba/specs/{feature_id}_/design.md` - Design specification
    /// 2. `.gba/specs/{feature_id}_/verification.md` - Verification plan
    ///
    /// # Errors
    ///
    /// Returns an error if spec generation fails.
    pub async fn finalize(&mut self) -> Result<(), GbaCoreError> {
        if self.state == PlanState::Completed {
            return Err(GbaCoreError::PlanError(
                "Session already finalized".to_string(),
            ));
        }

        info!(feature_slug = %self.feature_slug, "Finalizing plan session");

        // Disconnect dialogue session
        self.dialogue_session.disconnect().await?;
        self.state = PlanState::Finalizing;

        // Create feature directory
        let feature_id = self.workspace.create_feature(&self.feature_slug)?;
        self.feature_id = Some(feature_id.clone());

        // Create WriteSpec session for spec generation
        let ctx = PromptContext::new()
            .with_working_dir(self.workspace.root())
            .with_feature_slug(&self.feature_slug)
            .with_feature_id(&feature_id);
        let system_prompt = self.prompt_manager.render(PromptId::PlanSystem, &ctx)?;

        let mut spec_session = AgentSession::new(
            AgentPreset::WriteSpec,
            &self.finalize_config,
            system_prompt,
            self.cli_path.clone(),
        )?;
        spec_session.connect().await?;

        // Generate design spec
        let design_prompt = self.prompt_manager.render(PromptId::PlanDesignSpec, &ctx)?;
        debug!("Generating design specification");
        let _ = spec_session.send_and_collect(&design_prompt).await?;

        // Generate verification plan
        let verify_prompt = self
            .prompt_manager
            .render(PromptId::PlanVerification, &ctx)?;
        debug!("Generating verification plan");
        let _ = spec_session.send_and_collect(&verify_prompt).await?;

        spec_session.disconnect().await?;

        self.state = PlanState::Completed;
        info!(feature_id = %feature_id, "Plan session finalized");
        Ok(())
    }

    /// Returns the feature ID if the session has been finalized.
    #[must_use]
    pub fn feature_id(&self) -> Option<&str> {
        self.feature_id.as_deref()
    }

    /// Returns the feature slug.
    #[must_use]
    pub fn feature_slug(&self) -> &str {
        &self.feature_slug
    }

    /// Returns `true` if the session has been finalized.
    #[must_use]
    pub fn is_finalized(&self) -> bool {
        self.state == PlanState::Completed
    }
}

impl std::fmt::Debug for PlanSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanSession")
            .field("feature_slug", &self.feature_slug)
            .field("feature_id", &self.feature_id)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_state_debug() {
        let state = PlanState::Dialogue;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Dialogue"));
    }
}
