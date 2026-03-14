//! Event types for GBA execution progress.
//!
//! These events are emitted during GBA operations and can be consumed
//! by UI layers to display progress and status to users.

/// Events emitted during GBA execution.
///
/// These events provide visibility into the execution progress and can be
/// consumed by UI layers for real-time feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GbaEvent {
    /// Agent sent a text message.
    AssistantMessage(String),
    /// Agent is waiting for user input.
    WaitingForInput,
    /// A phase has started.
    PhaseStarted {
        /// Name of the phase.
        name: String,
        /// Current phase index (1-based).
        index: usize,
        /// Total number of phases.
        total: usize,
    },
    /// A phase has completed and changes have been committed.
    PhaseCommitted {
        /// Name of the phase.
        name: String,
    },
    /// Code review has started.
    ReviewStarted,
    /// Issues were found during review.
    IssuesFound(Vec<String>),
    /// Agent is fixing review issues.
    FixingIssues,
    /// Verification result.
    VerificationResult {
        /// Whether verification passed.
        passed: bool,
        /// Detailed verification output.
        details: String,
    },
    /// A pull request has been created.
    PrCreated {
        /// URL of the created pull request.
        url: String,
    },
    /// An error occurred during execution.
    Error(String),
}

impl GbaEvent {
    /// Returns `true` if this event indicates an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Returns `true` if this event requires user attention.
    #[must_use]
    pub const fn requires_attention(&self) -> bool {
        matches!(
            self,
            Self::WaitingForInput
                | Self::IssuesFound(_)
                | Self::VerificationResult { .. }
                | Self::Error(_)
        )
    }

    /// Returns a human-readable description of this event.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::AssistantMessage(msg) => msg.clone(),
            Self::WaitingForInput => String::from("Waiting for user input..."),
            Self::PhaseStarted { name, index, total } => {
                format!("Starting phase {index}/{total}: {name}")
            }
            Self::PhaseCommitted { name } => format!("Phase completed and committed: {name}"),
            Self::ReviewStarted => String::from("Starting code review..."),
            Self::IssuesFound(issues) => {
                format!("Found {} issue(s): {}", issues.len(), issues.join(", "))
            }
            Self::FixingIssues => String::from("Fixing review issues..."),
            Self::VerificationResult { passed, details } => {
                if *passed {
                    format!("Verification passed: {details}")
                } else {
                    format!("Verification failed: {details}")
                }
            }
            Self::PrCreated { url } => format!("Pull request created: {url}"),
            Self::Error(msg) => format!("Error: {msg}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_started_description() {
        let event = GbaEvent::PhaseStarted {
            name: String::from("Implementation"),
            index: 1,
            total: 3,
        };
        assert_eq!(event.description(), "Starting phase 1/3: Implementation");
    }

    #[test]
    fn test_phase_committed_description() {
        let event = GbaEvent::PhaseCommitted {
            name: String::from("Core Logic"),
        };
        assert_eq!(
            event.description(),
            "Phase completed and committed: Core Logic"
        );
    }

    #[test]
    fn test_issues_found_description() {
        let event = GbaEvent::IssuesFound(vec![
            String::from("Missing error handling"),
            String::from("Unused import"),
        ]);
        assert_eq!(
            event.description(),
            "Found 2 issue(s): Missing error handling, Unused import"
        );
    }

    #[test]
    fn test_verification_result_passed() {
        let event = GbaEvent::VerificationResult {
            passed: true,
            details: String::from("All tests passed"),
        };
        assert_eq!(event.description(), "Verification passed: All tests passed");
        assert!(!event.is_error());
    }

    #[test]
    fn test_verification_result_failed() {
        let event = GbaEvent::VerificationResult {
            passed: false,
            details: String::from("2 tests failed"),
        };
        assert_eq!(event.description(), "Verification failed: 2 tests failed");
        assert!(!event.is_error());
    }

    #[test]
    fn test_pr_created_description() {
        let event = GbaEvent::PrCreated {
            url: String::from("https://github.com/user/repo/pull/42"),
        };
        assert_eq!(
            event.description(),
            "Pull request created: https://github.com/user/repo/pull/42"
        );
    }

    #[test]
    fn test_error_description() {
        let event = GbaEvent::Error(String::from("Something went wrong"));
        assert_eq!(event.description(), "Error: Something went wrong");
        assert!(event.is_error());
    }

    #[test]
    fn test_is_error() {
        assert!(GbaEvent::Error(String::new()).is_error());
        assert!(!GbaEvent::WaitingForInput.is_error());
        assert!(!GbaEvent::ReviewStarted.is_error());
    }

    #[test]
    fn test_requires_attention() {
        assert!(GbaEvent::WaitingForInput.requires_attention());
        assert!(GbaEvent::Error(String::new()).requires_attention());
        assert!(GbaEvent::IssuesFound(vec![]).requires_attention());
        assert!(
            GbaEvent::VerificationResult {
                passed: true,
                details: String::new()
            }
            .requires_attention()
        );
        assert!(!GbaEvent::ReviewStarted.requires_attention());
        assert!(!GbaEvent::FixingIssues.requires_attention());
    }
}
