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
#[derive(Default)]
pub enum ModelType {
    #[default]
    Openai,
    Gemini,
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
#[derive(Default)]
pub enum MessageStyle {
    #[default]
    Short,
    Long,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    Accept,
    Edit,
    Regenerate,
    Quit,
}
