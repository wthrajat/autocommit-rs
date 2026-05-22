use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::git::diff::{clean_diff, generate_prompt};
use crate::types::{CommitType, MessageStyle};

use super::prompts::{
    FALLBACK_MESSAGE, MAX_DIFF_LENGTH, MAX_TOKENS_LONG, MAX_TOKENS_SHORT, SYSTEM_PROMPT_LONG,
    SYSTEM_PROMPT_SHORT,
};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-lite-preview:generateContent";

#[derive(Serialize)]
struct GeminiContent {
    contents: Vec<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct GeminiPart {
    parts: Vec<Part>,
}

#[derive(Serialize, serde::Deserialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(serde::Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}

#[derive(serde::Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<CandidateContent>,
}

#[derive(serde::Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<Part>,
    #[serde(default)]
    text: Option<String>,
}

pub async fn generate_commit_message(
    diff: &str,
    commit_type: Option<CommitType>,
    files: &[String],
    branch_name: &str,
    message_style: MessageStyle,
) -> Result<String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .context("GEMINI_API_KEY environment variable is not set")?;

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
    let request = GeminiContent {
        contents: vec![GeminiPart {
            parts: vec![Part { text: user_prompt }],
        }],
        system_instruction: Some(GeminiPart {
            parts: vec![Part {
                text: system_prompt.to_string(),
            }],
        }),
        generation_config: Some(GenerationConfig {
            temperature: Some(0.0),
            max_output_tokens: Some(max_tokens),
        }),
    };

    let url = format!("{}?key={}", GEMINI_API_URL, api_key);

    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            if let Ok(gemini_resp) = response.json::<GeminiResponse>().await {
                let content = gemini_resp
                    .candidates
                    .first()
                    .and_then(|c| c.content.as_ref())
                    .and_then(|c| {
                        if let Some(text) = &c.text
                            && !text.is_empty()
                        {
                            return Some(text.trim().to_string());
                        }
                        let combined: String = c
                            .parts
                            .iter()
                            .filter_map(|p| {
                                if p.text.is_empty() {
                                    None
                                } else {
                                    Some(p.text.as_str())
                                }
                            })
                            .collect();
                        if combined.is_empty() {
                            None
                        } else {
                            Some(combined.trim().to_string())
                        }
                    });

                if let Some(content) = content
                    && !content.is_empty()
                {
                    return Ok(content);
                }
            }
            Ok(match commit_type {
                Some(t) => format!("{}(scope): update files (fallback)", t.as_str()),
                None => FALLBACK_MESSAGE.to_string(),
            })
        }
        Err(e) => {
            eprintln!("Gemini API Error: {}", e);
            Ok(match commit_type {
                Some(t) => format!("{}(scope): update files (fallback)", t.as_str()),
                None => FALLBACK_MESSAGE.to_string(),
            })
        }
    }
}
