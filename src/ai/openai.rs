use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::git::diff::{clean_diff, generate_prompt};
use crate::types::{CommitType, MessageStyle};

use super::prompts::{
    FALLBACK_MESSAGE, MAX_DIFF_LENGTH, MAX_TOKENS_LONG, MAX_TOKENS_SHORT, SYSTEM_PROMPT_LONG,
    SYSTEM_PROMPT_SHORT,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_completion_tokens: u32,
}

#[derive(serde::Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(serde::Deserialize)]
struct Choice {
    message: Option<ChoiceMessage>,
}

#[derive(serde::Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

pub async fn generate_commit_message(
    diff: &str,
    commit_type: Option<CommitType>,
    files: &[String],
    branch_name: &str,
    message_style: MessageStyle,
) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY environment variable is not set")?;

    if diff.trim().is_empty() {
        eprintln!("{} No diff found", "✖".red());
        return Ok(FALLBACK_MESSAGE.to_string());
    }

    let cleaned_diff = clean_diff(diff);
    let truncated_diff: String = cleaned_diff.chars().take(MAX_DIFF_LENGTH).collect();

    let system_prompt = match message_style {
        MessageStyle::Long => SYSTEM_PROMPT_LONG,
        MessageStyle::Short => SYSTEM_PROMPT_SHORT,
    };

    let max_tokens = match message_style {
        MessageStyle::Long => MAX_TOKENS_LONG,
        MessageStyle::Short => MAX_TOKENS_SHORT,
    };

    let user_prompt = generate_prompt(&truncated_diff, commit_type, files, branch_name);

    let client = reqwest::Client::new();
    let request = ChatCompletionRequest {
        model: "gpt-5.4-nano".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        temperature: 0.0,
        max_completion_tokens: max_tokens,
    };

    match client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(completion) = response.json::<ChatCompletionResponse>().await
                && let Some(content) = completion
                    .choices
                    .first()
                    .and_then(|c| c.message.as_ref())
                    .and_then(|m| m.content.as_ref())
                    .map(|c| c.trim().to_string())
                && !content.is_empty()
            {
                return Ok(content);
            }
            Ok(match commit_type {
                Some(t) => format!("{}(scope): update files (fallback)", t.as_str()),
                None => FALLBACK_MESSAGE.to_string(),
            })
        }
        Err(e) => {
            eprintln!("OpenAI API Error: {}", e);
            Ok(match commit_type {
                Some(t) => format!("{}(scope): update files (fallback)", t.as_str()),
                None => FALLBACK_MESSAGE.to_string(),
            })
        }
    }
}
