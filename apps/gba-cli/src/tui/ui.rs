//! TUI rendering and main event loop.
//!
//! This module provides the ratatui rendering for the planning session
//! and the main event loop that drives the TUI.

use std::io::Stdout;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use gba_core::{GbaEngine, GbaEvent, PlanSession};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use ratatui_textarea::{Input, Key};

use super::app::{App, AppState, Sender};
use super::event::{Event, EventHandler};

/// Command to finalize the planning session.
const FINALIZE_COMMAND: &str = "/finalize";
/// Command alias for finalizing.
const DONE_COMMAND: &str = "/done";

/// Runs the TUI application.
///
/// This is the main event loop that:
/// 1. Handles keyboard events
/// 2. Sends user messages to the planning session
/// 3. Receives and displays assistant responses
/// 4. Finalizes the session when complete
///
/// # Returns
///
/// Returns `Ok(Some(feature_id))` on successful completion,
/// `Ok(None)` if cancelled, or an error on failure.
///
/// # Errors
///
/// Returns an error if the terminal cannot be rendered or if
/// the planning session fails.
pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut app: App,
    engine: &GbaEngine,
) -> Result<Option<String>> {
    // Create the planning session
    let mut session = engine.plan(&app.feature_slug).await?;
    app.add_assistant_message(format!(
        "Welcome! Let's plan the feature: {}\n\nDescribe what you want to build.\n\n\
         Type '{}' or '{}' when you're ready to generate the specification.\n\n\
         Press Enter to send, Alt+Enter for new line.",
        app.feature_slug, FINALIZE_COMMAND, DONE_COMMAND
    ));

    let events = EventHandler::default();

    loop {
        // Draw the UI
        terminal.draw(|frame| draw(frame, &mut app))?;

        // Handle pending finalize
        if app.pending_finalize {
            app.pending_finalize = false;
            app.state = AppState::Finalizing;
            terminal.draw(|frame| draw(frame, &mut app))?;
            finalize_session(&mut app, &mut session).await?;
            continue;
        }

        // Handle pending message (send to AI with animation)
        if let Some(message) = app.pending_message.take() {
            handle_send_with_animation(terminal, &mut app, &mut session, &message).await?;
            continue;
        }

        // Calculate input area width for auto-wrapping
        // Account for: margin (2) + borders (2) = 4 chars
        let term_size = terminal.size()?;
        let input_width = term_size.width.saturating_sub(4) as usize;

        // Handle events
        match events.next()? {
            Event::Key(key) => {
                match app.state {
                    AppState::Dialogue => {
                        // Calculate visible height for scrolling
                        let term_size = terminal.size()?;
                        let input_lines = app.textarea.lines().len().max(1);
                        let input_height = (input_lines + 2).clamp(3, 10) as u16;
                        let visible_height =
                            term_size.height.saturating_sub(3 + input_height + 2) as usize; // header + input + margins

                        if !handle_dialogue_key(&mut app, key, input_width, visible_height) {
                            return Ok(None);
                        }
                    }
                    AppState::WaitingForResponse | AppState::Finalizing => {
                        // Ignore input while waiting
                    }
                    AppState::Completed | AppState::Error => {
                        if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                            return Ok(app.feature_id.clone());
                        }
                    }
                }
            }
            Event::Resize(_, _) => {
                // Terminal will be redrawn on next iteration
            }
            Event::Tick => {
                // Tick the spinner animation
                app.tick_spinner();
            }
        }

        if app.should_quit() {
            return Ok(app.feature_id.clone());
        }
    }
}

/// Handles sending message and receiving response with animated spinner.
async fn handle_send_with_animation(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    session: &mut PlanSession,
    message: &str,
) -> Result<()> {
    // Draw initial frame with spinner before API call
    app.tick_spinner();
    terminal.draw(|frame| draw(frame, app))?;

    // Get the stream with animation during the wait
    // We use tokio::select! in a loop to animate while waiting for send_stream
    let stream_fut = session.send_stream(message);
    let mut stream_fut = std::pin::pin!(stream_fut);

    let stream = loop {
        // Try to poll the future with a timeout
        tokio::select! {
            result = &mut stream_fut => {
                break result?;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Timeout - animate and continue polling
                app.tick_spinner();
                terminal.draw(|frame| draw(frame, app))?;
            }
        }
    };

    let mut stream = std::pin::pin!(stream);
    let mut response = String::new();

    // Process stream with animation
    loop {
        tokio::select! {
            // Process stream events
            result = stream.next() => {
                match result {
                    Some(Ok(GbaEvent::AssistantMessage(text))) => {
                        response.push_str(&text);
                    }
                    Some(Ok(GbaEvent::WaitingForInput)) => {
                        break;
                    }
                    Some(Err(e)) => {
                        app.set_error(e.to_string());
                        return Ok(());
                    }
                    None => break,
                    _ => {}
                }
            }
            // Animate spinner every 100ms
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                app.tick_spinner();
                terminal.draw(|frame| draw(frame, app))?;
            }
        }
    }

    if !response.is_empty() {
        app.add_assistant_message(response);
    }

    app.state = AppState::Dialogue;
    Ok(())
}

/// Handles key events in dialogue state.
/// Returns true to continue, false to quit.
fn handle_dialogue_key(
    app: &mut App,
    key: KeyEvent,
    input_width: usize,
    visible_height: usize,
) -> bool {
    // Handle special shortcuts first
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return false; // Quit
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return false; // Quit
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Finalize
            app.clear_input();
            app.pending_finalize = true;
            return true;
        }
        KeyCode::Esc => {
            return false; // Quit
        }
        // Page Up/Down for scrolling messages
        KeyCode::PageUp => {
            app.page_up(visible_height);
            return true;
        }
        KeyCode::PageDown => {
            app.page_down(visible_height, visible_height);
            return true;
        }
        // Home/End for scroll to top/bottom
        KeyCode::Home => {
            app.scroll_offset = 0;
            return true;
        }
        KeyCode::End => {
            app.scroll_to_bottom();
            return true;
        }
        // Ctrl+Up/Down for scrolling messages
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_up();
            return true;
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_down(visible_height);
            return true;
        }
        KeyCode::Enter => {
            // Alt+Enter inserts newline, Enter sends message
            if key.modifiers.contains(KeyModifiers::ALT) {
                // Insert newline
                app.textarea.input(Input {
                    key: Key::Enter,
                    ctrl: false,
                    alt: false,
                    shift: false,
                });
                return true;
            }

            let input = app.input_text();
            let input_trimmed = input.trim();

            // Check for finalize command
            if input_trimmed == FINALIZE_COMMAND || input_trimmed == DONE_COMMAND {
                app.clear_input();
                app.pending_finalize = true;
                return true;
            }

            if !input_trimmed.is_empty() {
                let message = input_trimmed.to_string();
                app.add_user_message(message.clone());
                app.clear_input();
                app.state = AppState::WaitingForResponse;
                app.pending_message = Some(message);
            }
            return true;
        }
        _ => {}
    }

    // Let textarea handle other keys
    let input = key_event_to_input(key);
    app.textarea.input(input);

    // Auto-wrap: check if current line exceeds width and wrap at word boundary
    if let KeyCode::Char(c) = key.code
        && c != ' '
        && !key.modifiers.contains(KeyModifiers::CONTROL)
    {
        auto_wrap_textarea(&mut app.textarea, input_width);
    }

    true
}

/// Auto-wraps text in textarea when current line exceeds max width.
/// Tries to wrap at word boundaries (spaces), otherwise wraps at exact width.
fn auto_wrap_textarea(textarea: &mut ratatui_textarea::TextArea<'static>, max_width: usize) {
    if max_width < 10 {
        return; // Too narrow to wrap
    }

    let lines = textarea.lines();
    let line_count = lines.len();

    if line_count == 0 {
        return;
    }

    // Check the last line (current line being edited)
    let current_line = &lines[line_count - 1];
    let line_len = current_line.chars().count();

    if line_len <= max_width {
        return;
    }

    // Find the last space before max_width to wrap at word boundary
    let chars: Vec<char> = current_line.chars().collect();
    let mut wrap_pos = max_width;

    // Look for a space in the last portion of the line
    for i in (max_width.saturating_sub(20)..max_width).rev() {
        if i < chars.len() && chars[i] == ' ' {
            wrap_pos = i + 1; // Wrap after the space
            break;
        }
    }

    // If no space found, wrap at exact width
    // Split the line: keep everything before wrap_pos on current line,
    // move everything after to a new line
    let after_wrap: String = chars[wrap_pos..].iter().collect();

    // Move cursor to end of current line
    textarea.move_cursor(ratatui_textarea::CursorMove::End);

    // Delete from cursor to end of line (everything after wrap point)
    for _ in wrap_pos..line_len {
        textarea.input(Input {
            key: Key::Backspace,
            ctrl: false,
            alt: false,
            shift: false,
        });
    }

    // Insert newline
    textarea.input(Input {
        key: Key::Enter,
        ctrl: false,
        alt: false,
        shift: false,
    });

    // Insert the remaining text
    for c in after_wrap.chars() {
        textarea.input(Input {
            key: Key::Char(c),
            ctrl: false,
            alt: false,
            shift: false,
        });
    }
}

/// Converts crossterm KeyEvent to tui_textarea Input.
fn key_event_to_input(key: KeyEvent) -> Input {
    let key_code = match key.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Tab => Key::Tab,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        _ => Key::Null,
    };

    Input {
        key: key_code,
        ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
        alt: key.modifiers.contains(KeyModifiers::ALT),
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
    }
}

/// Finalizes the planning session.
async fn finalize_session(app: &mut App, session: &mut PlanSession) -> Result<()> {
    app.add_assistant_message(
        "Generating design specification and verification plan...".to_string(),
    );

    session.finalize().await?;

    if let Some(feature_id) = session.feature_id() {
        app.set_completed(feature_id.to_string());
    } else {
        app.set_error("Failed to get feature ID after finalization".to_string());
    }

    Ok(())
}

/// Draws the TUI.
fn draw(frame: &mut Frame, app: &mut App) {
    // Calculate input area height based on content
    let input_lines = app.textarea.lines().len().max(1);
    let input_height = (input_lines + 2).clamp(3, 10) as u16; // Min 3, max 10

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),            // Header
            Constraint::Min(5),               // Messages
            Constraint::Length(input_height), // Input (dynamic)
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_messages(frame, app, chunks[1]);
    draw_input(frame, app, chunks[2]);

    // Draw overlay for special states
    if matches!(app.state, AppState::Completed | AppState::Error) {
        draw_overlay(frame, app);
    }
}

/// Draws the header section.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" GBA Plan: {} ", app.feature_slug);

    let header = Paragraph::new(title.clone())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(header, area);
}

/// Draws the messages section with scrolling support.
fn draw_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for message in &app.messages {
        let (prefix, style) = match message.sender {
            Sender::User => ("You: ", Style::default().fg(Color::Green)),
            Sender::Assistant => ("Assistant: ", Style::default().fg(Color::Yellow)),
        };

        // Add sender prefix
        lines.push(Line::from(Span::styled(
            prefix,
            style.add_modifier(Modifier::BOLD),
        )));

        // Add message content (wrap long lines)
        for line in message.content.lines() {
            lines.push(Line::from(Span::styled(line.to_string(), style)));
        }

        // Add spacing between messages
        lines.push(Line::from(""));
    }

    // Add loading indicator when waiting
    if app.state == AppState::WaitingForResponse || app.state == AppState::Finalizing {
        let spinner = app.spinner();
        let loading_text = if app.state == AppState::Finalizing {
            format!(" {} Generating specification...", spinner)
        } else {
            format!(" {} Assistant is thinking...", spinner)
        };
        lines.push(Line::from(Span::styled(
            loading_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    // Calculate visible height (minus borders and status line)
    let visible_height = area.height.saturating_sub(4) as usize; // 2 borders + 2 for status

    // Calculate total lines
    let total_lines = lines.len();

    // Calculate max scroll offset
    let max_scroll = total_lines.saturating_sub(visible_height);

    // Clamp scroll offset to valid range
    let scroll_offset = app.scroll_offset.min(max_scroll);

    // Calculate title with scroll indicator
    let title = if total_lines > visible_height {
        let scroll_percent = if max_scroll > 0 {
            (scroll_offset * 100) / max_scroll
        } else {
            0
        };
        format!(" Messages ({}%, Ctrl+↑↓ to scroll) ", scroll_percent)
    } else {
        " Messages ".to_string()
    };

    // Add status indicator at the bottom
    let status = match app.state {
        AppState::Dialogue => {
            "Ready. Enter to send, Alt+Enter for newline, Esc to cancel. /finalize or Ctrl+F when ready."
        }
        AppState::WaitingForResponse => "Waiting for assistant response...",
        AppState::Finalizing => "Finalizing and generating specs...",
        AppState::Completed => "Planning complete! Press Enter to exit.",
        AppState::Error => "Error occurred. Press Enter to exit.",
    };

    lines.push(Line::from(Span::styled(
        format!(" [{status}]"),
        Style::default().fg(Color::DarkGray),
    )));

    let messages_widget = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    frame.render_widget(messages_widget, area);
}

/// Draws the input section.
fn draw_input(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.state == AppState::Dialogue {
        // Set textarea block style
        let input_style = Style::default().fg(Color::White);
        app.textarea.set_block(
            Block::default()
                .title(" Input (Enter to send, Alt+Enter for newline, Ctrl+C to quit) ")
                .borders(Borders::ALL)
                .border_style(input_style),
        );

        // Render the textarea widget
        frame.render_widget(&app.textarea, area);
    } else {
        // Show disabled input
        let input_style = Style::default().fg(Color::DarkGray);
        let input = Paragraph::new("").style(input_style).block(
            Block::default()
                .title(" Input (disabled) ")
                .borders(Borders::ALL)
                .border_style(input_style),
        );
        frame.render_widget(input, area);
    }
}

/// Draws an overlay for completed/error states.
fn draw_overlay(frame: &mut Frame, app: &App) {
    // Clear a centered area
    let area = centered_rect(60, 30, frame.area());
    frame.render_widget(Clear, area);

    let (title, message, style) = match &app.state {
        AppState::Completed => {
            let feature_id = app.feature_id.as_deref().unwrap_or("unknown");
            (
                " Completed ",
                format!("Feature ID: {feature_id}\n\nPress Enter to exit."),
                Style::default().fg(Color::Green),
            )
        }
        AppState::Error => {
            let error = app.error_message.as_deref().unwrap_or("Unknown error");
            (
                " Error ",
                format!("Error: {error}\n\nPress Enter to exit."),
                Style::default().fg(Color::Red),
            )
        }
        _ => return,
    };

    let overlay = Paragraph::new(message)
        .style(style)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(style.add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(overlay, area);
}

/// Returns a centered rect for overlay positioning.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
