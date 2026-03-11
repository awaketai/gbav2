//! Agent tool preset definitions.
//!
//! This module defines the `AgentPreset` enum which controls tool permissions
//! for different GBA phases. These presets are hardcoded security boundaries
//! and cannot be configured by users.

/// Agent tool permission presets.
///
/// These presets define which Claude Code tools are available in different
/// GBA phases. This is a security boundary - presets are hardcoded in the
/// engine and not exposed through configuration files.
///
/// # Security Rationale
///
/// - **ReadOnly**: Used for analysis and review phases where no modifications
///   should be made to the codebase.
/// - **WriteSpec**: Used for generating documentation files under `.gba/`
///   directory only, not source code.
/// - **FullCoding**: Used for implementation phases where full file system
///   access and command execution is needed.
/// - **Verify**: Used for verification phases where build/test commands can
///   be run but no files can be modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentPreset {
    /// Read-only access: Read, Glob, Grep.
    /// Used for analysis and review phases with zero side effects.
    ReadOnly,
    /// Spec writing access: Read, Glob, Grep, Write.
    /// Used for generating `.gba/` documentation files only.
    WriteSpec,
    /// Full coding access: Read, Write, Edit, Bash, Glob, Grep.
    /// Used for implementation phases with full file system access.
    FullCoding,
    /// Verification access: Read, Glob, Grep, Bash (read-only commands).
    /// Can run cargo build/test/clippy but cannot modify files.
    Verify,
}

impl AgentPreset {
    /// Returns the list of allowed Claude Code tool names for this preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use gba_core::AgentPreset;
    ///
    /// let tools = AgentPreset::ReadOnly.allowed_tools();
    /// assert!(tools.contains(&"Read"));
    /// assert!(tools.contains(&"Glob"));
    /// assert!(tools.contains(&"Grep"));
    /// assert!(!tools.contains(&"Write"));
    /// ```
    #[must_use]
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            Self::ReadOnly => vec!["Read", "Glob", "Grep"],
            Self::WriteSpec => vec!["Read", "Glob", "Grep", "Write"],
            Self::FullCoding => vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep"],
            Self::Verify => vec!["Read", "Glob", "Grep", "Bash"],
        }
    }

    /// Returns a description of this preset's capabilities.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::ReadOnly => "Read-only access for analysis and review",
            Self::WriteSpec => "Read and write access for spec file generation",
            Self::FullCoding => "Full access for implementation phases",
            Self::Verify => "Read and command execution for verification",
        }
    }

    /// Returns `true` if this preset allows file writing.
    #[must_use]
    pub const fn can_write(&self) -> bool {
        matches!(self, Self::WriteSpec | Self::FullCoding)
    }

    /// Returns `true` if this preset allows file editing.
    #[must_use]
    pub const fn can_edit(&self) -> bool {
        matches!(self, Self::FullCoding)
    }

    /// Returns `true` if this preset allows bash command execution.
    #[must_use]
    pub const fn can_execute(&self) -> bool {
        matches!(self, Self::FullCoding | Self::Verify)
    }
}

impl std::fmt::Display for AgentPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadOnly => write!(f, "ReadOnly"),
            Self::WriteSpec => write!(f, "WriteSpec"),
            Self::FullCoding => write!(f, "FullCoding"),
            Self::Verify => write!(f, "Verify"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_tools() {
        let tools = AgentPreset::ReadOnly.allowed_tools();
        assert_eq!(tools, vec!["Read", "Glob", "Grep"]);
    }

    #[test]
    fn test_write_spec_tools() {
        let tools = AgentPreset::WriteSpec.allowed_tools();
        assert!(tools.contains(&"Read"));
        assert!(tools.contains(&"Glob"));
        assert!(tools.contains(&"Grep"));
        assert!(tools.contains(&"Write"));
        assert!(!tools.contains(&"Edit"));
        assert!(!tools.contains(&"Bash"));
    }

    #[test]
    fn test_full_coding_tools() {
        let tools = AgentPreset::FullCoding.allowed_tools();
        assert_eq!(tools.len(), 6);
        assert!(tools.contains(&"Read"));
        assert!(tools.contains(&"Write"));
        assert!(tools.contains(&"Edit"));
        assert!(tools.contains(&"Bash"));
        assert!(tools.contains(&"Glob"));
        assert!(tools.contains(&"Grep"));
    }

    #[test]
    fn test_verify_tools() {
        let tools = AgentPreset::Verify.allowed_tools();
        assert!(tools.contains(&"Read"));
        assert!(tools.contains(&"Glob"));
        assert!(tools.contains(&"Grep"));
        assert!(tools.contains(&"Bash"));
        assert!(!tools.contains(&"Write"));
        assert!(!tools.contains(&"Edit"));
    }

    #[test]
    fn test_can_write() {
        assert!(!AgentPreset::ReadOnly.can_write());
        assert!(AgentPreset::WriteSpec.can_write());
        assert!(AgentPreset::FullCoding.can_write());
        assert!(!AgentPreset::Verify.can_write());
    }

    #[test]
    fn test_can_edit() {
        assert!(!AgentPreset::ReadOnly.can_edit());
        assert!(!AgentPreset::WriteSpec.can_edit());
        assert!(AgentPreset::FullCoding.can_edit());
        assert!(!AgentPreset::Verify.can_edit());
    }

    #[test]
    fn test_can_execute() {
        assert!(!AgentPreset::ReadOnly.can_execute());
        assert!(!AgentPreset::WriteSpec.can_execute());
        assert!(AgentPreset::FullCoding.can_execute());
        assert!(AgentPreset::Verify.can_execute());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", AgentPreset::ReadOnly), "ReadOnly");
        assert_eq!(format!("{}", AgentPreset::WriteSpec), "WriteSpec");
        assert_eq!(format!("{}", AgentPreset::FullCoding), "FullCoding");
        assert_eq!(format!("{}", AgentPreset::Verify), "Verify");
    }

    #[test]
    fn test_description() {
        assert!(!AgentPreset::ReadOnly.description().is_empty());
        assert!(!AgentPreset::WriteSpec.description().is_empty());
        assert!(!AgentPreset::FullCoding.description().is_empty());
        assert!(!AgentPreset::Verify.description().is_empty());
    }
}
