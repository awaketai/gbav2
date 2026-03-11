//! Git operations for GBA.
//!
//! This module provides git utilities for committing changes and creating pull requests.

use std::process::Command;

use tracing::{debug, info};

use crate::error::GbaCoreError;

/// Git operations handler.
#[derive(Debug, Clone)]
pub struct GitOps {
    /// Working directory for git operations.
    working_dir: std::path::PathBuf,
}

impl GitOps {
    /// Creates a new `GitOps` instance for the given working directory.
    #[must_use]
    pub fn new(working_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
        }
    }

    /// Commits all staged changes with the given message.
    ///
    /// This function stages all changes and creates a commit.
    ///
    /// # Errors
    ///
    /// Returns an error if the git command fails.
    pub fn commit_phase(&self, message: &str) -> Result<(), GbaCoreError> {
        info!(message = %message, "Committing phase changes");

        // Stage all changes
        self.run_git_command(&["add", "-A"])?;

        // Create commit
        self.run_git_command(&["commit", "-m", message])?;

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
