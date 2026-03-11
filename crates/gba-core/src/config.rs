//! Configuration types for the GBA core engine.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

    /// Creates a new `GbaConfig` with the specified working directory and sessions config.
    #[must_use]
    pub fn with_sessions(working_dir: impl Into<PathBuf>, sessions: SessionsConfig) -> Self {
        Self {
            working_dir: working_dir.into(),
            sessions,
        }
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
    pub fn feature_spec_dir(&self, feature_id: &str) -> PathBuf {
        self.specs_dir().join(format!("{}_", feature_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            config.feature_spec_dir("0001"),
            PathBuf::from("/path/to/repo/.gba/specs/0001_")
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
}
