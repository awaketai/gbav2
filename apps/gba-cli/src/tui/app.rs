//! TUI application state for the planning session.
//!
//! This module defines the `App` struct which manages the state of the
//! interactive planning dialogue.

use ratatui_textarea::TextArea;

/// Spinner animation frames for loading indicator.
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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

/// A single message in the chat.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Who sent the message.
    pub sender: Sender,
    /// The message content.
    pub content: String,
}

/// TUI application state.
///
/// This struct manages all state for the interactive planning session,
/// including the input buffer, message history, and current state.
pub struct App {
    /// Feature slug being planned.
    pub feature_slug: String,
    /// Current state of the session.
    pub state: AppState,
    /// Multi-line text input for user messages.
    pub textarea: TextArea<'static>,
    /// Message history (user and assistant).
    pub messages: Vec<ChatMessage>,
    /// Vertical scroll offset in lines.
    pub scroll_offset: usize,
    /// Error message if in error state.
    pub error_message: Option<String>,
    /// Feature ID after finalization.
    pub feature_id: Option<String>,
    /// Pending message to send (used for async handling).
    pub pending_message: Option<String>,
    /// Whether to finalize after current message.
    pub pending_finalize: bool,
    /// Spinner frame index for loading animation.
    spinner_frame: usize,
}

impl App {
    /// Creates a new `App` instance.
    #[must_use]
    pub fn new(feature_slug: String, _verbose: bool) -> Self {
        let mut textarea = TextArea::default();
        // Disable cursor line style for cleaner look
        textarea.set_cursor_line_style(ratatui::style::Style::default());

        Self {
            feature_slug,
            state: AppState::Dialogue,
            textarea,
            messages: Vec::new(),
            scroll_offset: 0,
            error_message: None,
            feature_id: None,
            pending_message: None,
            pending_finalize: false,
            spinner_frame: 0,
        }
    }

    /// Returns the input text as a single string.
    #[must_use]
    pub fn input_text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Clears the input buffer.
    pub fn clear_input(&mut self) {
        self.textarea = TextArea::default();
        self.textarea
            .set_cursor_line_style(ratatui::style::Style::default());
    }

    /// Adds a user message to the history.
    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            sender: Sender::User,
            content,
        });
        self.scroll_to_bottom();
    }

    /// Adds an assistant message to the history.
    pub fn add_assistant_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            sender: Sender::Assistant,
            content,
        });
        self.scroll_to_bottom();
    }

    /// Returns the total number of lines in all messages.
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.messages
            .iter()
            .map(|m| m.content.lines().count() + 3) // +3 for sender, spacing
            .sum()
    }

    /// Scrolls up by one line.
    #[allow(dead_code)]
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scrolls down by one line.
    #[allow(dead_code)]
    pub fn scroll_down(&mut self, visible_height: usize) {
        let max_scroll = self.total_lines().saturating_sub(visible_height);
        self.scroll_offset = self.scroll_offset.saturating_add(1).min(max_scroll);
    }

    /// Scrolls up by one page.
    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scrolls down by one page.
    #[allow(dead_code)]
    pub fn page_down(&mut self, page_size: usize, visible_height: usize) {
        let max_scroll = self.total_lines().saturating_sub(visible_height);
        self.scroll_offset = self.scroll_offset.saturating_add(page_size).min(max_scroll);
    }

    /// Scrolls to the bottom of the message history.
    pub fn scroll_to_bottom(&mut self) {
        // Set to a large value - will be clamped when rendering
        self.scroll_offset = usize::MAX / 2;
    }

    /// Returns `true` if the app should quit.
    #[must_use]
    pub const fn should_quit(&self) -> bool {
        matches!(self.state, AppState::Error | AppState::Completed) && self.feature_id.is_some()
    }

    /// Sets the error state with a message.
    pub fn set_error(&mut self, message: String) {
        self.state = AppState::Error;
        self.error_message = Some(message);
    }

    /// Sets the completed state with the feature ID.
    pub fn set_completed(&mut self, feature_id: String) {
        self.state = AppState::Completed;
        self.feature_id = Some(feature_id);
    }

    /// Advances the spinner animation and returns the current frame.
    pub fn tick_spinner(&mut self) -> &'static str {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        SPINNER_FRAMES[self.spinner_frame]
    }

    /// Returns the current spinner frame without advancing.
    #[must_use]
    pub fn spinner(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_frame]
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("feature_slug", &self.feature_slug)
            .field("state", &self.state)
            .field("input", &self.input_text())
            .field("messages", &self.messages)
            .field("scroll_offset", &self.scroll_offset)
            .field("error_message", &self.error_message)
            .field("feature_id", &self.feature_id)
            .finish()
    }
}
