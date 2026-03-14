//! Prompt identifier and role types.

/// The role of a prompt, distinguishing between System and User prompts.
///
/// System prompts define the agent's identity, rules, and constraints.
/// They are passed when creating a session and remain constant throughout.
///
/// User prompts contain specific task instructions.
/// They are sent as user messages with each agent call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptRole {
    /// System Prompt: Sets the agent's identity, rules, and constraints.
    /// Passed at session creation and remains unchanged throughout the session lifecycle.
    System,
    /// User Prompt: Specific task instructions.
    /// Sent as a user message with each agent call.
    User,
}

/// Identifier for all GBA prompt templates.
///
/// Each variant corresponds to a specific template file and usage scenario.
/// The enum is marked as `#[non_exhaustive]` to allow adding new variants
/// without breaking changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PromptId {
    // ═══════════════════════════════════════════════════════════════════════════
    // gba init command
    // ═══════════════════════════════════════════════════════════════════════════
    /// System prompt for the repository analyst role.
    /// Sets up an agent that is precise, fact-oriented, and avoids speculation.
    InitSystem,

    /// User prompt to analyze the repository structure and output a JSON report.
    InitAnalyze,

    /// User prompt to generate a `gba.md` document for a specific directory.
    InitGbaMd,

    /// User prompt to generate or update the GBA section in CLAUDE.md.
    InitClaudeMd,

    // ═══════════════════════════════════════════════════════════════════════════
    // gba plan command
    // ═══════════════════════════════════════════════════════════════════════════
    /// System prompt for the planning architect role.
    /// Sets up an agent for collaborative multi-round planning without writing code.
    PlanSystem,

    /// User prompt to generate the design specification document.
    PlanDesignSpec,

    /// User prompt to generate the verification plan document.
    PlanVerification,

    // ═══════════════════════════════════════════════════════════════════════════
    // gba run command
    // ═══════════════════════════════════════════════════════════════════════════
    /// System prompt for the coding agent role with code quality rules.
    RunSystem,

    /// User prompt to implement a specific development phase.
    RunPhase,

    /// User prompt to review generated code and output a JSON issue list.
    RunReview,

    /// User prompt to execute the verification plan and output JSON results.
    RunVerify,
}

impl PromptId {
    /// Returns the role of this prompt.
    ///
    /// # Examples
    ///
    /// ```
    /// use gba_pm::{PromptId, PromptRole};
    ///
    /// assert_eq!(PromptId::InitSystem.role(), PromptRole::System);
    /// assert_eq!(PromptId::InitAnalyze.role(), PromptRole::User);
    /// assert_eq!(PromptId::RunSystem.role(), PromptRole::System);
    /// assert_eq!(PromptId::RunPhase.role(), PromptRole::User);
    /// ```
    #[must_use]
    pub const fn role(&self) -> PromptRole {
        match self {
            Self::InitSystem | Self::PlanSystem | Self::RunSystem => PromptRole::System,
            Self::InitAnalyze
            | Self::InitGbaMd
            | Self::InitClaudeMd
            | Self::PlanDesignSpec
            | Self::PlanVerification
            | Self::RunPhase
            | Self::RunReview
            | Self::RunVerify => PromptRole::User,
        }
    }

    /// Returns the template filename for this prompt ID.
    #[must_use]
    pub(crate) const fn template_name(&self) -> &'static str {
        match self {
            Self::InitSystem => "init_system.jinja",
            Self::InitAnalyze => "init_analyze.jinja",
            Self::InitGbaMd => "init_gba_md.jinja",
            Self::InitClaudeMd => "init_claude_md.jinja",
            Self::PlanSystem => "plan_system.jinja",
            Self::PlanDesignSpec => "plan_design_spec.jinja",
            Self::PlanVerification => "plan_verification.jinja",
            Self::RunSystem => "run_system.jinja",
            Self::RunPhase => "run_phase.jinja",
            Self::RunReview => "run_review.jinja",
            Self::RunVerify => "run_verify.jinja",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompts_return_system_role() {
        assert_eq!(PromptId::InitSystem.role(), PromptRole::System);
        assert_eq!(PromptId::PlanSystem.role(), PromptRole::System);
        assert_eq!(PromptId::RunSystem.role(), PromptRole::System);
    }

    #[test]
    fn test_user_prompts_return_user_role() {
        assert_eq!(PromptId::InitAnalyze.role(), PromptRole::User);
        assert_eq!(PromptId::InitGbaMd.role(), PromptRole::User);
        assert_eq!(PromptId::InitClaudeMd.role(), PromptRole::User);
        assert_eq!(PromptId::PlanDesignSpec.role(), PromptRole::User);
        assert_eq!(PromptId::PlanVerification.role(), PromptRole::User);
        assert_eq!(PromptId::RunPhase.role(), PromptRole::User);
        assert_eq!(PromptId::RunReview.role(), PromptRole::User);
        assert_eq!(PromptId::RunVerify.role(), PromptRole::User);
    }

    #[test]
    fn test_template_names() {
        assert_eq!(PromptId::InitSystem.template_name(), "init_system.jinja");
        assert_eq!(PromptId::RunPhase.template_name(), "run_phase.jinja");
    }
}
