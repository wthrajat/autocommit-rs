mod gemini;
mod openai;
pub(crate) mod prompts;

use crate::types::{CommitType, MessageStyle, ModelType};

pub struct GenerateOptions<'a> {
    pub diff: &'a str,
    pub commit_type: Option<CommitType>,
    pub files: &'a [String],
    pub branch_name: &'a str,
    pub message_style: MessageStyle,
}

pub async fn generate_commit_message(
    model: ModelType,
    options: GenerateOptions<'_>,
) -> anyhow::Result<String> {
    match model {
        ModelType::Gemini => {
            self::gemini::generate_commit_message(
                options.diff,
                options.commit_type,
                options.files,
                options.branch_name,
                options.message_style,
            )
            .await
        }
        ModelType::Openai => {
            self::openai::generate_commit_message(
                options.diff,
                options.commit_type,
                options.files,
                options.branch_name,
                options.message_style,
            )
            .await
        }
    }
}
