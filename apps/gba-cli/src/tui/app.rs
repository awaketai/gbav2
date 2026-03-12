//! TUI application state for the planning session.
//!
//! This module defines the `App` struct which manages the state of the
//! interactive planning dialogue.

/// Message in the chat history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The sender of the message.
    pub sender: Sender,
    /// The content of the message.
    pub content: String,
}

/// Message sender type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sender {
    /// User message.
    User,
    /// Assistant message.
    Assistant,
}

/// Current state of the planning session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppState {
    /// Session is in dialogue mode, waiting for user input.
    Dialogue,
    /// Session is waiting for assistant response.
    WaitingForResponse,
    /// Session is finalizing and generating specs.
    Finalizing,
    /// Session has completed successfully.
    Completed,
    /// Session has errored.
    Error,
}

/// TUI application state.
///
/// This struct manages all state for the interactive planning session,
/// including the input buffer, message history, and current state.
#[derive(Debug)]
#[allow(dead_code)]
pub struct App {
    /// Feature slug being planned.
    pub feature_slug: String,
    /// Current state of the session.
    pub state: AppState,
    /// Input buffer for user messages.
    pub input: String,
    /// Message history (user and assistant).
    pub messages: Vec<Message>,
    /// Current scroll offset in message history.
    pub scroll: usize,
    /// Whether verbose mode is enabled.
    pub verbose: bool,
    /// Error message if in error state.
    pub error_message: Option<String>,
    /// Feature ID after finalization.
    pub feature_id: Option<String>,
}

impl App {
    /// Creates a new `App` instance.
    #[must_use]
    pub fn new(feature_slug: String, verbose: bool) -> Self {
        Self {
            feature_slug,
            state: AppState::Dialogue,
            input: String::new(),
            messages: Vec::new(),
            scroll: 0,
            verbose,
            error_message: None,
            feature_id: None,
        }
    }

    /// Adds a user message to the history.
    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(Message {
            sender: Sender::User,
            content,
        });
        self.scroll_to_bottom();
    }

    /// Adds an assistant message to the history.
    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(Message {
            sender: Sender::Assistant,
            content,
        });
        self.scroll_to_bottom();
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Scrolls up in the message history.
    #[allow(dead_code)]
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    /// Scrolls down in the message history.
    #[allow(dead_code)]
    pub fn scroll_down(&mut self) {
        let max_scroll = self.messages.len().saturating_sub(1);
        if self.scroll < max_scroll {
            self.scroll += 1;
        }
    }

    /// Scrolls to the bottom of the message history.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.messages.len().saturating_sub(1);
    }

    /// Handles a character input.
    pub fn handle_char(&mut self, c: char) {
        if self.state == AppState::Dialogue {
            self.input.push(c);
        }
    }

    /// Handles backspace input.
    pub fn handle_backspace(&mut self) {
        if self.state == AppState::Dialogue {
            self.input.pop();
        }
    }

    /// Returns `true` if the app should quit.
    #[must_use]
    pub const fn should_quit(&self) -> bool {
        matches!(self.state, AppState::Error | AppState::Completed)
    }

    /// Sets the error state with a message.
    pub fn set_error(&mut self, message: String) {
        self.state = AppState::Error;
        self.error_message = Some(message);
    }

    /// Sets the completed state with the feature ID.
    #[allow(dead_code)]
    pub fn set_completed(&mut self, feature_id: String) {
        self.state = AppState::Completed;
        self.feature_id = Some(feature_id);
    }
}
