//! Workspace management for GBA.
//!
//! This module provides functions to manage the `.gba/` directory structure,
//! including specs files, feature numbering, and tree management.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::GbaCoreError;

/// The name of the GBA directory.
pub const GBA_DIR_NAME: &str = ".gba";

/// The name of the specs directory within `.gba/`.
pub const SPECS_DIR_NAME: &str = "specs";

/// The name of the trees directory within `.gba/`.
pub const TREES_DIR_NAME: &str = "trees";

/// The name of the templates directory within `.gba/`.
pub const TEMPLATES_DIR_NAME: &str = "templates";

/// The name of the config file.
pub const CONFIG_FILE_NAME: &str = "config.yaml";

/// The name of the design spec file.
pub const DESIGN_SPEC_FILE: &str = "design.md";

/// The name of the verification plan file.
pub const VERIFICATION_FILE: &str = "verification.md";

/// Workspace manager for GBA operations.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Root directory of the workspace (contains `.gba/`).
    root: PathBuf,
}

impl Workspace {
    /// Creates a new workspace manager for the given root directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the root directory path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the path to the `.gba/` directory.
    #[must_use]
    pub fn gba_dir(&self) -> PathBuf {
        PathBuf::from(&self.root).join(GBA_DIR_NAME)
    }

    /// Returns the path to the specs directory.
    #[must_use]
    pub fn specs_dir(&self) -> PathBuf {
        self.gba_dir().join(SPECS_DIR_NAME)
    }

    /// Returns the path to the trees directory.
    #[must_use]
    pub fn trees_dir(&self) -> PathBuf {
        self.gba_dir().join(TREES_DIR_NAME)
    }

    /// Returns the path to the templates directory.
    #[must_use]
    pub fn templates_dir(&self) -> PathBuf {
        self.gba_dir().join(TEMPLATES_DIR_NAME)
    }

    /// Returns the path to the config file.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.gba_dir().join(CONFIG_FILE_NAME)
    }

    /// Checks if the workspace has been initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.gba_dir().is_dir()
    }

    /// Initializes the workspace by creating the `.gba/` directory structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn initialize(&self) -> Result<(), GbaCoreError> {
        fs::create_dir_all(self.gba_dir())?;
        fs::create_dir_all(self.specs_dir())?;
        fs::create_dir_all(self.trees_dir())?;
        Ok(())
    }

    /// Returns the path to a feature's spec directory.
    #[must_use]
    pub fn feature_spec_dir(&self, feature_id: &str) -> PathBuf {
        self.specs_dir().join(format!("{}_", feature_id))
    }

    /// Creates a new feature spec directory and returns its ID.
    ///
    /// The feature ID is a zero-padded 4-digit number (e.g., "0001").
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or if
    /// there's an issue reading existing features.
    pub fn create_feature(&self, feature_slug: &str) -> Result<String, GbaCoreError> {
        let feature_id = self.next_feature_id()?;
        let feature_dir = self.feature_spec_dir(&feature_id);
        fs::create_dir_all(&feature_dir)?;

        // Create a slug file to store the feature slug
        let slug_file = feature_dir.join(".slug");
        fs::write(&slug_file, feature_slug)?;

        Ok(feature_id)
    }

    /// Gets the next available feature ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the specs directory cannot be read.
    pub fn next_feature_id(&self) -> Result<String, GbaCoreError> {
        let specs_dir = self.specs_dir();

        if !specs_dir.exists() {
            return Ok(String::from("0001"));
        }

        let mut max_id: u32 = 0;
        for entry in fs::read_dir(&specs_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Parse the feature ID from directory names like "0001_feature-slug"
            if let Some(id_str) = name_str.split('_').next()
                && let Ok(id) = id_str.parse::<u32>()
            {
                max_id = max_id.max(id);
            }
        }

        Ok(format!("{:04}", max_id + 1))
    }

    /// Returns the path to a feature's design spec file.
    #[must_use]
    pub fn design_spec_path(&self, feature_id: &str) -> PathBuf {
        self.feature_spec_dir(feature_id).join(DESIGN_SPEC_FILE)
    }

    /// Returns the path to a feature's verification plan file.
    #[must_use]
    pub fn verification_path(&self, feature_id: &str) -> PathBuf {
        self.feature_spec_dir(feature_id).join(VERIFICATION_FILE)
    }

    /// Reads the design spec for a feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_design_spec(&self, feature_id: &str) -> Result<String, GbaCoreError> {
        let path = self.design_spec_path(feature_id);
        Ok(fs::read_to_string(&path)?)
    }

    /// Writes the design spec for a feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write_design_spec(&self, feature_id: &str, content: &str) -> Result<(), GbaCoreError> {
        let path = self.design_spec_path(feature_id);
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(&path, content)?)
    }

    /// Reads the verification plan for a feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn read_verification(&self, feature_id: &str) -> Result<String, GbaCoreError> {
        let path = self.verification_path(feature_id);
        Ok(fs::read_to_string(&path)?)
    }

    /// Writes the verification plan for a feature.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn write_verification(&self, feature_id: &str, content: &str) -> Result<(), GbaCoreError> {
        let path = self.verification_path(feature_id);
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(&path, content)?)
    }

    /// Checks if a feature exists.
    #[must_use]
    pub fn feature_exists(&self, feature_id: &str) -> bool {
        self.feature_spec_dir(feature_id).is_dir()
    }

    /// Lists all feature IDs in the workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the specs directory cannot be read.
    pub fn list_features(&self) -> Result<Vec<String>, GbaCoreError> {
        let specs_dir = self.specs_dir();

        if !specs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut features = Vec::new();
        for entry in fs::read_dir(&specs_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(id_str) = name_str.split('_').next() {
                    features.push(id_str.to_string());
                }
            }
        }

        features.sort();
        Ok(features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_paths() {
        let ws = Workspace::new("/path/to/repo");
        assert_eq!(ws.root(), Path::new("/path/to/repo"));
        assert_eq!(ws.gba_dir(), PathBuf::from("/path/to/repo/.gba"));
        assert_eq!(ws.specs_dir(), PathBuf::from("/path/to/repo/.gba/specs"));
        assert_eq!(ws.trees_dir(), PathBuf::from("/path/to/repo/.gba/trees"));
        assert_eq!(
            ws.config_path(),
            PathBuf::from("/path/to/repo/.gba/config.yaml")
        );
    }

    #[test]
    fn test_feature_spec_dir() {
        let ws = Workspace::new("/repo");
        assert_eq!(
            ws.feature_spec_dir("0001"),
            PathBuf::from("/repo/.gba/specs/0001_")
        );
    }

    #[test]
    fn test_design_spec_path() {
        let ws = Workspace::new("/repo");
        assert_eq!(
            ws.design_spec_path("0001"),
            PathBuf::from("/repo/.gba/specs/0001_/design.md")
        );
    }

    #[test]
    fn test_verification_path() {
        let ws = Workspace::new("/repo");
        assert_eq!(
            ws.verification_path("0001"),
            PathBuf::from("/repo/.gba/specs/0001_/verification.md")
        );
    }

    #[test]
    fn test_initialize() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let ws = Workspace::new(temp_dir.path());

        assert!(!ws.is_initialized());
        ws.initialize().expect("Failed to initialize");
        assert!(ws.is_initialized());
        assert!(ws.gba_dir().is_dir());
        assert!(ws.specs_dir().is_dir());
        assert!(ws.trees_dir().is_dir());
    }

    #[test]
    fn test_create_feature() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let ws = Workspace::new(temp_dir.path());
        ws.initialize().expect("Failed to initialize");

        let id = ws
            .create_feature("add-auth")
            .expect("Failed to create feature");
        assert_eq!(id, "0001");
        assert!(ws.feature_exists("0001"));

        let id2 = ws
            .create_feature("add-logging")
            .expect("Failed to create feature");
        assert_eq!(id2, "0002");
        assert!(ws.feature_exists("0002"));
    }

    #[test]
    fn test_write_and_read_design_spec() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let ws = Workspace::new(temp_dir.path());
        ws.initialize().expect("Failed to initialize");

        ws.write_design_spec("0001", "# Design\n\nTest design")
            .expect("Failed to write design spec");

        let content = ws
            .read_design_spec("0001")
            .expect("Failed to read design spec");
        assert!(content.contains("# Design"));
        assert!(content.contains("Test design"));
    }

    #[test]
    fn test_write_and_read_verification() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let ws = Workspace::new(temp_dir.path());
        ws.initialize().expect("Failed to initialize");

        ws.write_verification("0001", "# Verification\n\nTest plan")
            .expect("Failed to write verification");

        let content = ws
            .read_verification("0001")
            .expect("Failed to read verification");
        assert!(content.contains("# Verification"));
        assert!(content.contains("Test plan"));
    }

    #[test]
    fn test_list_features() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let ws = Workspace::new(temp_dir.path());
        ws.initialize().expect("Failed to initialize");

        ws.create_feature("feature-a")
            .expect("Failed to create feature");
        ws.create_feature("feature-b")
            .expect("Failed to create feature");
        ws.create_feature("feature-c")
            .expect("Failed to create feature");

        let features = ws.list_features().expect("Failed to list features");
        assert_eq!(features, vec!["0001", "0002", "0003"]);
    }
}
