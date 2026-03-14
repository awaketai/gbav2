//! Configuration types for the GBA core engine.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::GbaCoreError;

/// Single session configuration parameters.
///
/// These parameters can be tuned by users via `.gba/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionConfig {
    /// Claude model name (e.g., "claude-sonnet-4-20250514").
    pub model: String,
    /// Maximum conversation turns.
    pub max_turns: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: String::from("claude-sonnet-4-20250514"),
            max_turns: 10,
        }
    }
}

impl SessionConfig {
    /// Creates a new `SessionConfig` with the specified model and max_turns.
    #[must_use]
    pub fn new(model: impl Into<String>, max_turns: usize) -> Self {
        Self {
            model: model.into(),
            max_turns,
        }
    }
}

/// Session configuration for each scenario.
///
/// Users can tune these parameters in `.gba/config.yaml` to optimize
/// for different use cases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionsConfig {
    /// Configuration for `gba init` command.
    pub init: SessionConfig,
    /// Configuration for `gba plan` command.
    pub plan: SessionConfig,
    /// Configuration for `gba run` implementation phases.
    pub run_phase: SessionConfig,
    /// Configuration for `gba run` review phases.
    pub run_review: SessionConfig,
    /// Configuration for `gba run` verification phases.
    pub run_verify: SessionConfig,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            init: SessionConfig::new("claude-sonnet-4-20250514", 3),
            plan: SessionConfig::new("claude-sonnet-4-20250514", 30),
            run_phase: SessionConfig::new("claude-sonnet-4-20250514", 20),
            run_review: SessionConfig::new("claude-sonnet-4-20250514", 5),
            run_verify: SessionConfig::new("claude-sonnet-4-20250514", 10),
        }
    }
}

/// GBA engine global configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GbaConfig {
    /// Working directory for the GBA operations.
    pub working_dir: PathBuf,
    /// Session configurations for different scenarios.
    #[serde(default)]
    pub sessions: SessionsConfig,
}

impl GbaConfig {
    /// Creates a new `GbaConfig` with the specified working directory.
    #[must_use]
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: working_dir.into(),
            sessions: SessionsConfig::default(),
        }
    }

    /// Loads configuration from the working directory.
    ///
    /// This method:
    /// 1. Checks if `.gba/config.yaml` exists
    /// 2. If exists, loads and parses the YAML configuration
    /// 3. If not exists, returns default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be read or parsed.
    pub fn load(working_dir: impl Into<PathBuf>) -> Result<Self, GbaCoreError> {
        let working_dir = working_dir.into();
        let config_path = working_dir.join(".gba").join("config.yaml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Self = serde_yaml::from_str(&content)?;
            config.working_dir = working_dir;
            Ok(config)
        } else {
            Ok(Self::new(working_dir))
        }
    }

    /// Creates a new `GbaConfig` with the specified working directory and sessions config.
    #[must_use]
    pub fn with_sessions(working_dir: impl Into<PathBuf>, sessions: SessionsConfig) -> Self {
        Self {
            working_dir: working_dir.into(),
            sessions,
        }
    }

    /// Saves the configuration to `.gba/config.yaml`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> Result<(), GbaCoreError> {
        let config_path = self.config_file();

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Returns the path to the `.gba/` directory.
    #[must_use]
    pub fn gba_dir(&self) -> PathBuf {
        self.working_dir.join(".gba")
    }

    /// Returns the path to the `.gba/config.yaml` file.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.gba_dir().join("config.yaml")
    }

    /// Returns the path to the `.gba/specs/` directory.
    #[must_use]
    pub fn specs_dir(&self) -> PathBuf {
        self.gba_dir().join("specs")
    }

    /// Returns the path to the `.gba/trees/` directory.
    #[must_use]
    pub fn trees_dir(&self) -> PathBuf {
        self.gba_dir().join("trees")
    }

    /// Returns the path to a feature's spec directory.
    #[must_use]
    pub fn feature_spec_dir(&self, feature_id: &str, feature_slug: &str) -> PathBuf {
        self.specs_dir().join(format!("{}_{}", feature_id, feature_slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_turns, 10);
    }

    #[test]
    fn test_session_config_new() {
        let config = SessionConfig::new("claude-opus-4", 15);
        assert_eq!(config.model, "claude-opus-4");
        assert_eq!(config.max_turns, 15);
    }

    #[test]
    fn test_sessions_config_default() {
        let config = SessionsConfig::default();
        assert_eq!(config.init.max_turns, 3);
        assert_eq!(config.plan.max_turns, 30);
        assert_eq!(config.run_phase.max_turns, 20);
        assert_eq!(config.run_review.max_turns, 5);
        assert_eq!(config.run_verify.max_turns, 10);
    }

    #[test]
    fn test_gba_config_new() {
        let config = GbaConfig::new("/path/to/repo");
        assert_eq!(config.working_dir, PathBuf::from("/path/to/repo"));
        assert_eq!(config.sessions.init.max_turns, 3);
    }

    #[test]
    fn test_gba_config_paths() {
        let config = GbaConfig::new("/path/to/repo");
        assert_eq!(config.gba_dir(), PathBuf::from("/path/to/repo/.gba"));
        assert_eq!(
            config.config_file(),
            PathBuf::from("/path/to/repo/.gba/config.yaml")
        );
        assert_eq!(
            config.specs_dir(),
            PathBuf::from("/path/to/repo/.gba/specs")
        );
        assert_eq!(
            config.trees_dir(),
            PathBuf::from("/path/to/repo/.gba/trees")
        );
        assert_eq!(
            config.feature_spec_dir("0001", "add-auth"),
            PathBuf::from("/path/to/repo/.gba/specs/0001_add-auth")
        );
    }

    #[test]
    fn test_session_config_serialization() {
        let config = SessionConfig::new("claude-sonnet-4", 20);
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize");
        assert!(yaml.contains("claude-sonnet-4"));
        assert!(yaml.contains("max_turns: 20"));

        let deserialized: SessionConfig =
            serde_yaml::from_str(&yaml).expect("Failed to deserialize");
        assert_eq!(deserialized, config);
    }

    #[test]
    fn test_sessions_config_serialization() {
        let config = SessionsConfig::default();
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize");
        assert!(yaml.contains("init:"));
        assert!(yaml.contains("plan:"));
        assert!(yaml.contains("run_phase:"));
        assert!(yaml.contains("run_review:"));
        assert!(yaml.contains("run_verify:"));

        let deserialized: SessionsConfig =
            serde_yaml::from_str(&yaml).expect("Failed to deserialize");
        assert_eq!(deserialized, config);
    }

    #[test]
    fn test_gba_config_serialization() {
        let config = GbaConfig::new("/path/to/repo");
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize");
        assert!(yaml.contains("working_dir: /path/to/repo"));
        assert!(yaml.contains("sessions:"));

        let deserialized: GbaConfig = serde_yaml::from_str(&yaml).expect("Failed to deserialize");
        assert_eq!(deserialized.working_dir, config.working_dir);
    }

    #[test]
    fn test_gba_config_load_without_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = GbaConfig::load(temp_dir.path()).expect("Failed to load config");

        assert_eq!(config.working_dir, temp_dir.path());
        // Should use defaults
        assert_eq!(config.sessions.init.max_turns, 3);
    }

    #[test]
    fn test_gba_config_load_with_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create .gba directory and config file
        let gba_dir = temp_dir.path().join(".gba");
        std::fs::create_dir_all(&gba_dir).expect("Failed to create .gba dir");

        let config_content = r#"
working_dir: /dummy
sessions:
  init:
    model: claude-opus-4
    max_turns: 5
  plan:
    model: claude-sonnet-4-20250514
    max_turns: 50
  run_phase:
    model: claude-sonnet-4-20250514
    max_turns: 25
  run_review:
    model: claude-haiku
    max_turns: 10
  run_verify:
    model: claude-sonnet-4-20250514
    max_turns: 15
"#;
        std::fs::write(gba_dir.join("config.yaml"), config_content)
            .expect("Failed to write config");

        let config = GbaConfig::load(temp_dir.path()).expect("Failed to load config");

        // working_dir should be overridden by load()
        assert_eq!(config.working_dir, temp_dir.path());
        assert_eq!(config.sessions.init.model, "claude-opus-4");
        assert_eq!(config.sessions.init.max_turns, 5);
        assert_eq!(config.sessions.plan.max_turns, 50);
        assert_eq!(config.sessions.run_phase.max_turns, 25);
        assert_eq!(config.sessions.run_review.model, "claude-haiku");
    }

    #[test]
    fn test_gba_config_save() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut config = GbaConfig::new(temp_dir.path());
        config.sessions.init.max_turns = 7;

        config.save().expect("Failed to save config");

        // Verify file exists
        assert!(config.config_file().exists());

        // Load and verify
        let loaded = GbaConfig::load(temp_dir.path()).expect("Failed to load config");
        assert_eq!(loaded.sessions.init.max_turns, 7);
    }
}
