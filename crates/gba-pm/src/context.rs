//! Context types for prompt rendering.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An entry representing a generated `gba.md` file.
///
/// Used by the `init_claude_md` template to reference all generated documentation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GbaMdEntry {
    /// The file path relative to the repository root.
    pub path: String,
    /// A one-line summary of the directory's purpose.
    pub summary: String,
}

impl GbaMdEntry {
    /// Creates a new `GbaMdEntry` with the given path and summary.
    ///
    /// # Examples
    ///
    /// ```
    /// use gba_pm::GbaMdEntry;
    ///
    /// let entry = GbaMdEntry::new("src/auth/gba.md", "Authentication module");
    /// assert_eq!(entry.path, "src/auth/gba.md");
    /// assert_eq!(entry.summary, "Authentication module");
    /// ```
    #[must_use]
    pub fn new(path: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            summary: summary.into(),
        }
    }
}

/// Context data passed to prompt templates.
///
/// Not all fields are used in every template; unused fields retain their default values.
/// This struct is designed to be incrementally populated as needed for different operations.
///
/// # Examples
///
/// ```
/// use gba_pm::PromptContext;
///
/// let ctx = PromptContext::default()
///     .with_feature_slug("add-auth")
///     .with_feature_id("0001")
///     .with_working_dir("/path/to/repo");
///
/// assert_eq!(ctx.feature_slug, "add-auth");
/// assert_eq!(ctx.feature_id, "0001");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct PromptContext {
    /// Feature identifier slug (e.g., "add-auth").
    pub feature_slug: String,
    /// Feature number (e.g., "0001"), used for spec file paths.
    pub feature_id: String,
    /// Current working directory.
    pub working_dir: PathBuf,
    /// Repository directory tree snapshot.
    pub repo_tree: String,
    /// Design specification content (used during run phase).
    pub design_spec: String,
    /// Verification plan content (used during run phase).
    pub verification_plan: String,
    /// Current phase name (used by run_phase).
    pub phase_name: String,
    /// Current phase description (used by run_phase).
    pub phase_description: String,
    /// Current phase index, starting from 1 (used by run_phase).
    pub phase_index: usize,
    /// Total number of phases (used by run_phase).
    pub phase_total: usize,
    /// List of issues found during review (used by run_verify).
    pub review_issues: Vec<String>,
    /// Directory path (used by init_gba_md).
    pub directory_path: String,
    /// Directory analysis context (used by init_gba_md).
    pub directory_analysis: String,
    /// List of generated gba.md files (used by init_claude_md).
    pub gba_md_files: Vec<GbaMdEntry>,
}

impl PromptContext {
    /// Creates a new `PromptContext` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the feature slug.
    #[must_use]
    pub fn with_feature_slug(mut self, value: impl Into<String>) -> Self {
        self.feature_slug = value.into();
        self
    }

    /// Sets the feature ID.
    #[must_use]
    pub fn with_feature_id(mut self, value: impl Into<String>) -> Self {
        self.feature_id = value.into();
        self
    }

    /// Sets the working directory.
    #[must_use]
    pub fn with_working_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.working_dir = value.into();
        self
    }

    /// Sets the repository tree.
    #[must_use]
    pub fn with_repo_tree(mut self, value: impl Into<String>) -> Self {
        self.repo_tree = value.into();
        self
    }

    /// Sets the design specification.
    #[must_use]
    pub fn with_design_spec(mut self, value: impl Into<String>) -> Self {
        self.design_spec = value.into();
        self
    }

    /// Sets the verification plan.
    #[must_use]
    pub fn with_verification_plan(mut self, value: impl Into<String>) -> Self {
        self.verification_plan = value.into();
        self
    }

    /// Sets the phase name.
    #[must_use]
    pub fn with_phase_name(mut self, value: impl Into<String>) -> Self {
        self.phase_name = value.into();
        self
    }

    /// Sets the phase description.
    #[must_use]
    pub fn with_phase_description(mut self, value: impl Into<String>) -> Self {
        self.phase_description = value.into();
        self
    }

    /// Sets the phase index.
    #[must_use]
    pub fn with_phase_index(mut self, value: usize) -> Self {
        self.phase_index = value;
        self
    }

    /// Sets the total number of phases.
    #[must_use]
    pub fn with_phase_total(mut self, value: usize) -> Self {
        self.phase_total = value;
        self
    }

    /// Sets the review issues.
    #[must_use]
    pub fn with_review_issues(mut self, value: Vec<String>) -> Self {
        self.review_issues = value;
        self
    }

    /// Sets the directory path.
    #[must_use]
    pub fn with_directory_path(mut self, value: impl Into<String>) -> Self {
        self.directory_path = value.into();
        self
    }

    /// Sets the directory analysis.
    #[must_use]
    pub fn with_directory_analysis(mut self, value: impl Into<String>) -> Self {
        self.directory_analysis = value.into();
        self
    }

    /// Sets the gba.md files list.
    #[must_use]
    pub fn with_gba_md_files(mut self, value: Vec<GbaMdEntry>) -> Self {
        self.gba_md_files = value;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gba_md_entry_new() {
        let entry = GbaMdEntry::new("src/auth/gba.md", "Auth module");
        assert_eq!(entry.path, "src/auth/gba.md");
        assert_eq!(entry.summary, "Auth module");
    }

    #[test]
    fn test_gba_md_entry_default() {
        let entry = GbaMdEntry::default();
        assert!(entry.path.is_empty());
        assert!(entry.summary.is_empty());
    }

    #[test]
    fn test_gba_md_entry_serialization() {
        let entry = GbaMdEntry::new("src/auth/gba.md", "Auth module");
        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        assert!(json.contains("src/auth/gba.md"));
        assert!(json.contains("Auth module"));

        let deserialized: GbaMdEntry = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized, entry);
    }

    #[test]
    fn test_prompt_context_default() {
        let ctx = PromptContext::default();
        assert!(ctx.feature_slug.is_empty());
        assert!(ctx.feature_id.is_empty());
        assert!(ctx.working_dir.as_os_str().is_empty());
        assert!(ctx.repo_tree.is_empty());
        assert!(ctx.design_spec.is_empty());
        assert!(ctx.verification_plan.is_empty());
        assert!(ctx.phase_name.is_empty());
        assert!(ctx.phase_description.is_empty());
        assert_eq!(ctx.phase_index, 0);
        assert_eq!(ctx.phase_total, 0);
        assert!(ctx.review_issues.is_empty());
        assert!(ctx.directory_path.is_empty());
        assert!(ctx.directory_analysis.is_empty());
        assert!(ctx.gba_md_files.is_empty());
    }

    #[test]
    fn test_prompt_context_builder() {
        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_feature_id("0001")
            .with_working_dir("/path/to/repo")
            .with_repo_tree("src/\n  main.rs")
            .with_phase_name("Implementation")
            .with_phase_description("Add auth module")
            .with_phase_index(1)
            .with_phase_total(3);

        assert_eq!(ctx.feature_slug, "add-auth");
        assert_eq!(ctx.feature_id, "0001");
        assert_eq!(ctx.working_dir, PathBuf::from("/path/to/repo"));
        assert_eq!(ctx.repo_tree, "src/\n  main.rs");
        assert_eq!(ctx.phase_name, "Implementation");
        assert_eq!(ctx.phase_description, "Add auth module");
        assert_eq!(ctx.phase_index, 1);
        assert_eq!(ctx.phase_total, 3);
    }

    #[test]
    fn test_prompt_context_serialization() {
        let ctx = PromptContext::new()
            .with_feature_slug("add-auth")
            .with_feature_id("0001")
            .with_phase_index(2);

        let json = serde_json::to_string(&ctx).expect("Failed to serialize");
        assert!(json.contains("add-auth"));
        assert!(json.contains("0001"));
        assert!(json.contains("phase_index"));

        let deserialized: PromptContext =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.feature_slug, "add-auth");
        assert_eq!(deserialized.feature_id, "0001");
        assert_eq!(deserialized.phase_index, 2);
    }

    #[test]
    fn test_prompt_context_with_gba_md_files() {
        let files = vec![
            GbaMdEntry::new("src/auth/gba.md", "Auth"),
            GbaMdEntry::new("src/db/gba.md", "Database"),
        ];

        let ctx = PromptContext::new().with_gba_md_files(files.clone());
        assert_eq!(ctx.gba_md_files.len(), 2);
        assert_eq!(ctx.gba_md_files[0].path, "src/auth/gba.md");
    }
}
