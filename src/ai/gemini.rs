use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use super::candidates::schema as candidate_schema;
use super::client::{response_body, send_with_retry};
use super::{ProviderResponse, TokenUsage};

const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: [GeminiContent<'a>; 1],
    #[serde(rename = "system_instruction")]
    system_instruction: GeminiContent<'a>,
    #[serde(rename = "generationConfig")]
    generation_config: Value,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: [GeminiPart<'a>; 1],
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    usage_metadata: Option<GeminiUsage>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<CandidateContent>,
    finish_reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(serde::Deserialize)]
struct ResponsePart {
    #[serde(default)]
    text: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsage {
    prompt_token_count: Option<u64>,
    candidates_token_count: Option<u64>,
    cached_content_token_count: Option<u64>,
}

pub async fn generate_commit_messages(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: u32,
) -> Result<ProviderResponse> {
    let request = GeminiRequest {
        contents: [GeminiContent {
            parts: [GeminiPart { text: user_prompt }],
        }],
        system_instruction: GeminiContent {
            parts: [GeminiPart {
                text: system_prompt,
            }],
        },
        generation_config: json!({
            "maxOutputTokens": max_tokens,
            "responseFormat": {
                "text": {
                    "mimeType": "application/json",
                    "schema": candidate_schema()
                }
            }
        }),
    };
    let url = format!("{GEMINI_API_BASE_URL}/{model}:generateContent");
    let response = send_with_retry(
        client
            .post(url)
            .header("x-goog-api-key", api_key)
            .json(&request),
    )
    .await?;
    let body = response_body("Gemini", response).await?;
    let response = serde_json::from_str::<GeminiResponse>(&body)
        .context("Gemini returned an invalid response")?;
    let candidate = response
        .candidates
        .first()
        .context("Gemini returned no completion candidates")?;
    let content = candidate
        .content
        .as_ref()
        .map(|content| {
            content
                .parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<String>()
        })
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty());
    let Some(content) = content else {
        let reason = candidate.finish_reason.as_deref().unwrap_or("unknown");
        bail!("Gemini returned an empty commit message (finish reason: {reason})");
    };
    let usage = response
        .usage_metadata
        .map_or_else(TokenUsage::default, |usage| TokenUsage {
            input_tokens: usage.prompt_token_count,
            cached_input_tokens: usage.cached_content_token_count,
            output_tokens: usage.candidates_token_count,
        });

    Ok(ProviderResponse { content, usage })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_current_rest_fields_without_network_access() {
        let request = GeminiRequest {
            contents: [GeminiContent {
                parts: [GeminiPart { text: "user" }],
            }],
            system_instruction: GeminiContent {
                parts: [GeminiPart { text: "system" }],
            },
            generation_config: json!({
                "maxOutputTokens": 320,
                "responseFormat": {
                    "text": {
                        "mimeType": "application/json",
                        "schema": candidate_schema()
                    }
                }
            }),
        };
        let value = serde_json::to_value(request).unwrap();

        assert_eq!(value["system_instruction"]["parts"][0]["text"], "system");
        assert_eq!(
            value["generationConfig"]["responseFormat"]["text"]["mimeType"],
            "application/json"
        );
        assert!(value["generationConfig"].get("temperature").is_none());
    }
}
