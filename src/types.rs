use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitType {
    Feat,
    Fix,
    Docs,
    Style,
    Refactor,
    Perf,
    Test,
    Build,
    Ci,
    Chore,
    Revert,
}

impl CommitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitType::Feat => "feat",
            CommitType::Fix => "fix",
            CommitType::Docs => "docs",
            CommitType::Style => "style",
            CommitType::Refactor => "refactor",
            CommitType::Perf => "perf",
            CommitType::Test => "test",
            CommitType::Build => "build",
            CommitType::Ci => "ci",
            CommitType::Chore => "chore",
            CommitType::Revert => "revert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    Openai,
    Gemini,
}

impl Default for ModelType {
    fn default() -> Self {
        ModelType::Openai
    }
}

impl ModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelType::Openai => "openai",
            ModelType::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStyle {
    Short,
    Long,
}

impl Default for MessageStyle {
    fn default() -> Self {
        MessageStyle::Short
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    Accept,
    Edit,
    Regenerate,
    Quit,
}
