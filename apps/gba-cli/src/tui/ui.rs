//! TUI rendering and main event loop.
//!
//! This module provides the ratatui rendering for the planning session
//! and the main event loop that drives the TUI.

use std::io::Stdout;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
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
         Type '{}' or '{}' when you're ready to generate the specification.",
        app.feature_slug, FINALIZE_COMMAND, DONE_COMMAND
    ));

    let events = EventHandler::default();

    loop {
        // Draw the UI
        terminal.draw(|frame| draw(frame, &app))?;

        // Handle events
        match events.next()? {
            Event::Key(key) => {
                match app.state {
                    AppState::Dialogue => {
                        match key.code {
                            KeyCode::Char(c) => {
                                // Handle Ctrl+C and Ctrl+D for quit
                                if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                                    return Ok(None);
                                }
                                if c == 'd' && key.modifiers.contains(KeyModifiers::CONTROL) {
                                    return Ok(None);
                                }
                                // Handle Ctrl+F for finalize
                                if c == 'f' && key.modifiers.contains(KeyModifiers::CONTROL) {
                                    app.state = AppState::Finalizing;
                                    match finalize_session(&mut app, &mut session).await {
                                        Ok(()) => {}
                                        Err(e) => {
                                            app.set_error(e.to_string());
                                        }
                                    }
                                    continue;
                                }
                                app.handle_char(c);
                            }
                            KeyCode::Backspace => {
                                app.handle_backspace();
                            }
                            KeyCode::Enter => {
                                let input = app.input.clone();

                                // Check for finalize command
                                if input == FINALIZE_COMMAND || input == DONE_COMMAND {
                                    app.clear_input();
                                    app.state = AppState::Finalizing;
                                    match finalize_session(&mut app, &mut session).await {
                                        Ok(()) => {}
                                        Err(e) => {
                                            app.set_error(e.to_string());
                                        }
                                    }
                                    continue;
                                }

                                if !input.is_empty() {
                                    app.add_user_message(input.clone());
                                    app.clear_input();
                                    app.state = AppState::WaitingForResponse;

                                    // Send message and get response
                                    match send_and_receive(&mut app, &mut session, &input).await {
                                        Ok(()) => {
                                            app.state = AppState::Dialogue;
                                        }
                                        Err(e) => {
                                            app.set_error(e.to_string());
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                return Ok(None);
                            }
                            _ => {}
                        }
                    }
                    AppState::WaitingForResponse => {
                        // Ignore input while waiting
                    }
                    AppState::Finalizing => {
                        // Ignore input while finalizing
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
                // Periodic update (no-op for now)
            }
        }

        if app.should_quit() {
            return Ok(app.feature_id.clone());
        }
    }
}

/// Sends a message and receives the response.
async fn send_and_receive(app: &mut App, session: &mut PlanSession, message: &str) -> Result<()> {
    // Use streaming for better UX
    let stream = session.send_stream(message).await?;
    let mut stream = std::pin::pin!(stream);

    let mut response = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(GbaEvent::AssistantMessage(text)) => {
                response.push_str(&text);
            }
            Ok(GbaEvent::WaitingForInput) => {
                break;
            }
            Err(e) => {
                return Err(e.into());
            }
            _ => {}
        }
    }

    if !response.is_empty() {
        app.add_assistant_message(response);
    }

    Ok(())
}

/// Finalizes the planning session.
async fn finalize_session(app: &mut App, session: &mut PlanSession) -> Result<()> {
    app.add_assistant_message("Generating design specification and verification plan...".to_string());

    session.finalize().await?;

    if let Some(feature_id) = session.feature_id() {
        app.set_completed(feature_id.to_string());
    } else {
        app.set_error("Failed to get feature ID after finalization".to_string());
    }

    Ok(())
}

/// Draws the TUI.
fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Messages
            Constraint::Length(3), // Input
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

/// Draws the messages section.
fn draw_messages(frame: &mut Frame, app: &App, area: Rect) {
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

    // Add status indicator
    let status = match app.state {
        AppState::Dialogue => "Ready. Enter to send, Esc to cancel. Type /finalize or /done when ready, or Ctrl+F.",
        AppState::WaitingForResponse => "Waiting for assistant response...",
        AppState::Finalizing => "Finalizing and generating specs...",
        AppState::Completed => "Planning complete! Press Enter to exit.",
        AppState::Error => "Error occurred. Press Enter to exit.",
    };

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" [{status}]"),
        Style::default().fg(Color::DarkGray),
    )));

    let messages_widget = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Messages ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    frame.render_widget(messages_widget, area);
}

/// Draws the input section.
fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let input_style = if app.state == AppState::Dialogue {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input_text = if app.state == AppState::Dialogue {
        app.input.clone()
    } else {
        String::new()
    };

    let input = Paragraph::new(input_text).style(input_style).block(
        Block::default()
            .title(" Input (Enter to send, /finalize when done, Ctrl+C to quit) ")
            .borders(Borders::ALL)
            .border_style(input_style),
    );

    frame.render_widget(input, area);

    // Show cursor position
    if app.state == AppState::Dialogue {
        let cursor_x = area.x + 1 + app.input.len() as u16;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
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
