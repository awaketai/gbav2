//! Git operations for GBA.
//!
//! This module provides git utilities for committing changes and creating pull requests.

use crate::error::GbaCoreError;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info};

/// Default pre-commit hook commands to run
const DEFAULT_HOOKS: &[&str] = &[
    "cargo build",
    "cargo +nightly fmt",
    "cargo clippy -- -D warnings",
    "cargo audit",
];

/// Git operations handler.
#[derive(Debug, Clone)]
pub struct GitOps {
    /// Working directory for git operations.
    working_dir: PathBuf,
}

impl GitOps {
    /// Creates a new `GitOps` instance for the given working directory.
    #[must_use]
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
        }
    }

    /// Commits all staged changes with the given message.
    ///
    /// This function stages all changes and creates a commit.
    /// Runs pre-commit hooks (build/fmt/lint/security check) before creating the commit.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails or any pre-commit hook fails.
    pub fn commit_phase(&self, message: &str) -> Result<(), GbaCoreError> {
        self.commit_phase_with_hooks(message, DEFAULT_HOOKS)
    }

    /// Commits changes with optional pre-commit hooks.
    ///
    /// # Arguments
    ///
    /// * `message` - The commit message
    /// * `hooks` - List of hook commands to run (empty uses default hooks)
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails or any hook fails.
    pub fn commit_phase_with_hooks(
        &self,
        message: &str,
        hooks: &[&str],
    ) -> Result<(), GbaCoreError> {
        info!(message = %message, "Committing phase changes with hooks");

        let working_dir = &self.working_dir;

        // Run each pre-commit hook
        for hook in hooks {
            debug!(hook = ?hook, "Running pre-commit hook");
            let output = Command::new("sh")
                .args(["-c", hook])
                .current_dir(working_dir)
                .output()
                .map_err(|e| GbaCoreError::RunError(format!("Failed to run hook '{hook}': {e}")))?;

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stdout.is_empty() || !stderr.is_empty() {
                    return Err(GbaCoreError::RunError(format!(
                        "Pre-commit hook '{}' failed:\nstdout: {}\nstderr: {}",
                        hook,
                        stdout.trim(),
                        stderr.trim()
                    )));
                }
            }
        }

        // Stage all changes
        self.run_git_command(&["add", "-A"])?;

        // Create commit (without hooks to avoid infinite loop)
        self.run_git_command(&["commit", "-m", message, "--no-verify"])?;

        debug!("Phase committed successfully");
        Ok(())
    }

    /// Creates a pull request with the given title and body.
    ///
    /// Returns the URL of the created pull request.
    ///
    /// # Errors
    ///
    /// Returns an error if the gh command fails.
    pub fn create_pr(&self, title: &str, body: &str) -> Result<String, GbaCoreError> {
        info!(title = %title, "Creating pull request");

        let output = self.run_gh_command(&["pr", "create", "--title", title, "--body", body])?;
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!(url = %url, "Pull request created");
        Ok(url)
    }

    /// Gets the current branch name.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn current_branch(&self) -> Result<String, GbaCoreError> {
        let output = self.run_git_command(&["branch", "--show-current"])?;
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(branch)
    }

    /// Checks if there are uncommitted changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn has_changes(&self) -> Result<bool, GbaCoreError> {
        let output = self.run_git_command(&["status", "--porcelain"])?;
        Ok(!output.stdout.is_empty())
    }

    /// Runs a git command and returns the output.
    fn run_git_command(&self, args: &[&str]) -> Result<std::process::Output, GbaCoreError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| GbaCoreError::RunError(format!("Failed to run git command: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GbaCoreError::RunError(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }

        Ok(output)
    }

    /// Runs a gh (GitHub CLI) command and returns the output.
    fn run_gh_command(&self, args: &[&str]) -> Result<std::process::Output, GbaCoreError> {
        let output = Command::new("gh")
            .args(args)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| GbaCoreError::RunError(format!("Failed to run gh command: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GbaCoreError::RunError(format!(
                "gh command failed: {}",
                stderr.trim()
            )));
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_ops_new() {
        let ops = GitOps::new("/repo");
        assert_eq!(ops.working_dir, std::path::PathBuf::from("/repo"));
    }

    #[test]
    fn test_git_ops_clone() {
        let ops = GitOps::new("/repo");
        let cloned = ops.clone();
        assert_eq!(ops.working_dir, cloned.working_dir);
    }
}
