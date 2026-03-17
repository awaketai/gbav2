//! Agent session management.
//!
//! This module provides `AgentSession` which wraps `ClaudeClient` and manages
//! multi-turn conversations with tool permission presets.

use std::path::PathBuf;
use std::pin::Pin;

use claude_agent_sdk_rs::{
    ClaudeAgentOptions, ClaudeClient, Message, PermissionMode, SystemPrompt,
};
use futures::stream::Stream;
use tracing::debug;

use crate::config::SessionConfig;
use crate::error::GbaCoreError;
use crate::preset::AgentPreset;

/// Agent message wrapper for streaming responses.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Text message from the assistant.
    Text(String),
    /// Tool use event.
    ToolUse {
        /// Name of the tool being used.
        name: String,
        /// Input to the tool.
        input: serde_json::Value,
    },
    /// Tool result event.
    ToolResult {
        /// Name of the tool.
        name: String,
        /// Output from the tool.
        output: String,
    },
    /// Session completed.
    Completed {
        /// Total cost in USD.
        cost_usd: Option<f64>,
    },
}

/// Agent session managing a Claude client with preset-based tool permissions.
///
/// This struct wraps `ClaudeClient` and automatically configures allowed tools
/// based on the specified `AgentPreset`.
pub struct AgentSession {
    /// The underlying Claude client.
    client: ClaudeClient,
    /// The preset controlling tool permissions.
    preset: AgentPreset,
    /// The system prompt for this session.
    system_prompt: String,
}

impl AgentSession {
    /// Creates a new agent session with the specified preset and configuration.
    ///
    /// The session is configured with tool permissions based on the preset.
    /// This is a security boundary - tools are restricted according to the
    /// preset's allowed tools list.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gba_core::{AgentSession, AgentPreset, SessionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), gba_core::GbaCoreError> {
    /// let config = SessionConfig::default();
    /// let session = AgentSession::new(
    ///     AgentPreset::ReadOnly,
    ///     &config,
    ///     "You are a code reviewer.".to_string(),
    ///     None,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        preset: AgentPreset,
        config: &SessionConfig,
        system_prompt: String,
        cli_path: Option<PathBuf>,
    ) -> Result<Self, GbaCoreError> {
        let allowed_tools: Vec<String> = preset
            .allowed_tools()
            .iter()
            .map(|s| s.to_string())
            .collect();

        debug!(
            preset = ?preset,
            tools = ?allowed_tools,
            model = %config.model,
            max_turns = config.max_turns,
            cli_path = ?cli_path,
            "Creating agent session"
        );

        let mut options = ClaudeAgentOptions::builder()
            .model(&config.model)
            .max_turns(config.max_turns as u32)
            .allowed_tools(allowed_tools)
            .system_prompt(SystemPrompt::Text(system_prompt.clone()))
            .permission_mode(PermissionMode::BypassPermissions)
            .build();

        // Set cli_path if provided
        if let Some(path) = cli_path {
            options.cli_path = Some(path);
        }

        let client = ClaudeClient::new(options);

        Ok(Self {
            client,
            preset,
            system_prompt,
        })
    }

    /// Returns the preset for this session.
    #[must_use]
    pub const fn preset(&self) -> AgentPreset {
        self.preset
    }

    /// Returns the system prompt for this session.
    #[must_use]
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Connects to Claude and starts the session.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    pub async fn connect(&mut self) -> Result<(), GbaCoreError> {
        debug!("Connecting agent session");
        self.client.connect().await?;
        Ok(())
    }

    /// Disconnects from Claude and ends the session.
    ///
    /// # Errors
    ///
    /// Returns an error if the disconnection fails.
    pub async fn disconnect(&mut self) -> Result<(), GbaCoreError> {
        debug!("Disconnecting agent session");
        self.client.disconnect().await?;
        Ok(())
    }

    /// Sends a user message and returns a stream of agent messages.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gba_core::{AgentSession, AgentPreset, SessionConfig, AgentMessage};
    /// use futures::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), gba_core::GbaCoreError> {
    /// let config = SessionConfig::default();
    /// let mut session = AgentSession::new(
    ///     AgentPreset::ReadOnly,
    ///     &config,
    ///     "You are helpful.".to_string(),
    ///     None,  // cli_path - use default
    /// )?;
    /// session.connect().await?;
    ///
    /// {
    ///     let mut stream = session.send("Hello!").await?;
    ///     while let Some(msg) = stream.next().await {
    ///         match msg? {
    ///             AgentMessage::Text(text) => println!("Assistant: {}", text),
    ///             AgentMessage::Completed { cost_usd } => {
    ///                 println!("Session completed, cost: {:?}", cost_usd);
    ///                 break;
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// } // stream is dropped here, releasing the borrow
    ///
    /// session.disconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(
        &mut self,
        user_prompt: &str,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<AgentMessage, GbaCoreError>> + Send + '_>>,
        GbaCoreError,
    > {
        debug!(prompt_len = user_prompt.len(), "Sending user prompt");
        self.client.query(user_prompt).await?;

        let stream = self.client.receive_response();
        Ok(Box::pin(async_stream::stream! {
            use futures::StreamExt;
            let mut inner_stream = stream;

            while let Some(result) = inner_stream.next().await {
                match result {
                    Ok(message) => {
                        match message {
                            Message::Assistant(msg) => {
                                for block in &msg.message.content {
                                    match block {
                                        claude_agent_sdk_rs::ContentBlock::Text(text) => {
                                            yield Ok(AgentMessage::Text(text.text.clone()));
                                        }
                                        claude_agent_sdk_rs::ContentBlock::ToolUse(tool) => {
                                            yield Ok(AgentMessage::ToolUse {
                                                name: tool.name.clone(),
                                                input: tool.input.clone(),
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Message::Result(result_msg) => {
                                yield Ok(AgentMessage::Completed {
                                    cost_usd: result_msg.total_cost_usd,
                                });
                                break;
                            }
                            // Handle other message types without producing output
                            Message::User(_)
                            | Message::System(_)
                            | Message::StreamEvent(_)
                            | Message::ControlCancelRequest(_) => {}
                        }
                    }
                    Err(e) => {
                        yield Err(GbaCoreError::from(e));
                        break;
                    }
                }
            }
        }))
    }

    /// Sends a user message and collects all text responses.
    ///
    /// This is a convenience method that collects all text messages
    /// and returns them as a single string.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or received.
    pub async fn send_and_collect(&mut self, user_prompt: &str) -> Result<String, GbaCoreError> {
        use futures::StreamExt;

        let mut stream = self.send(user_prompt).await?;
        let mut text = String::new();

        while let Some(msg) = stream.next().await {
            match msg? {
                AgentMessage::Text(t) => {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&t);
                }
                AgentMessage::Completed { .. } => break,
                _ => {}
            }
        }

        Ok(text)
    }
}

impl std::fmt::Debug for AgentSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentSession")
            .field("preset", &self.preset)
            .field("system_prompt_len", &self.system_prompt.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_session_creation() {
        let config = SessionConfig::default();
        let session = AgentSession::new(
            AgentPreset::ReadOnly,
            &config,
            "You are helpful.".to_string(),
            None,
        );

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.preset(), AgentPreset::ReadOnly);
        assert_eq!(session.system_prompt(), "You are helpful.");
    }

    #[test]
    fn test_agent_session_debug() {
        let config = SessionConfig::default();
        let session = AgentSession::new(
            AgentPreset::FullCoding,
            &config,
            "System prompt".to_string(),
            None,
        )
        .unwrap();

        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("FullCoding"));
        assert!(debug_str.contains("system_prompt_len"));
    }

    #[test]
    fn test_agent_message_debug() {
        let msg = AgentMessage::Text("Hello".to_string());
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Text"));

        let msg = AgentMessage::ToolUse {
            name: "Read".to_string(),
            input: serde_json::json!({"path": "/test"}),
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("ToolUse"));

        let msg = AgentMessage::Completed {
            cost_usd: Some(0.5),
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Completed"));
    }
}
