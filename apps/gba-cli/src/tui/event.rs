//! Terminal event handling for the TUI.
//!
//! This module provides event handling for keyboard input and window resize.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tracing::trace;

/// Terminal event types.
#[derive(Debug, Clone)]
pub enum Event {
    /// Key press event.
    Key(KeyEvent),
    /// Terminal resize event.
    #[allow(dead_code)]
    Resize(u16, u16),
    /// Tick event for periodic updates.
    Tick,
}

/// Event handler for terminal events.
///
/// This struct spawns a background thread to poll for terminal events
/// and sends them through a channel to the main event loop.
#[derive(Debug)]
pub struct EventHandler {
    /// Receiver for events.
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    /// Creates a new `EventHandler` with the specified tick rate.
    ///
    /// # Panics
    ///
    /// Panics if the background thread cannot be spawned.
    #[must_use]
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(crossterm_event) = event::read() {
                        match crossterm_event {
                            CrosstermEvent::Key(key) => {
                                // Only process key press events (not release)
                                if key.kind == KeyEventKind::Press {
                                    trace!(key = ?key, "Key pressed");
                                    if tx.send(Event::Key(key)).is_err() {
                                        break;
                                    }
                                }
                            }
                            CrosstermEvent::Resize(width, height) => {
                                trace!(width, height, "Terminal resized");
                                if tx.send(Event::Resize(width, height)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Send tick event if no other events
                    if tx.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx }
    }

    /// Receives the next event.
    ///
    /// # Errors
    ///
    /// Returns an error if the event channel is closed.
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }

    /// Receives the next event with a timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the timeout is reached or the channel is closed.
    #[allow(dead_code)]
    pub fn next_timeout(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
