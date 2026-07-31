use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use super::candidates::schema as candidate_schema;
use super::client::{response_body, send_with_retry};
use super::{ProviderResponse, TokenUsage};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 2],
    reasoning_effort: &'static str,
    max_completion_tokens: u32,
    prompt_cache_key: &'a str,
    response_format: Value,
}

#[derive(serde::Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<OpenAiUsage>,
}

#[derive(serde::Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(serde::Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
    refusal: Option<String>,
}

#[derive(serde::Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    prompt_tokens_details: Option<PromptTokenDetails>,
}

#[derive(serde::Deserialize)]
struct PromptTokenDetails {
    cached_tokens: Option<u64>,
}

pub async fn generate_commit_messages(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    prompt_cache_key: &str,
    max_tokens: u32,
) -> Result<ProviderResponse> {
    let request = ChatCompletionRequest {
        model,
        messages: [
            ChatMessage {
                role: "system",
                content: system_prompt,
            },
            ChatMessage {
                role: "user",
                content: user_prompt,
            },
        ],
        reasoning_effort: "minimal",
        max_completion_tokens: max_tokens,
        prompt_cache_key,
        response_format: json!({
            "type": "json_schema",
            "json_schema": {
                "name": "commit_message_candidates",
                "strict": true,
                "schema": candidate_schema()
            }
        }),
    };
    let response = send_with_retry(
        client
            .post(OPENAI_API_URL)
            .bearer_auth(api_key)
            .json(&request),
    )
    .await?;
    let body = response_body("OpenAI", response).await?;
    let completion = serde_json::from_str::<ChatCompletionResponse>(&body)
        .context("OpenAI returned an invalid response")?;
    let choice = completion
        .choices
        .first()
        .context("OpenAI returned no completion choices")?;
    if let Some(refusal) = choice.message.refusal.as_deref() {
        bail!("OpenAI refused to generate a commit message: {refusal}");
    }
    let content = choice
        .message
        .content
        .as_deref()
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .context("OpenAI returned an empty commit message")?
        .to_string();
    let usage = completion
        .usage
        .map_or_else(TokenUsage::default, |usage| TokenUsage {
            input_tokens: usage.prompt_tokens,
            cached_input_tokens: usage
                .prompt_tokens_details
                .and_then(|details| details.cached_tokens),
            output_tokens: usage.completion_tokens,
        });

    Ok(ProviderResponse { content, usage })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_low_cost_parameters_and_structured_output_without_network_access() {
        let request = ChatCompletionRequest {
            model: "gpt-5-nano",
            messages: [
                ChatMessage {
                    role: "system",
                    content: "system",
                },
                ChatMessage {
                    role: "user",
                    content: "user",
                },
            ],
            reasoning_effort: "minimal",
            max_completion_tokens: 320,
            prompt_cache_key: "cache-key",
            response_format: json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "commit_message_candidates",
                    "strict": true,
                    "schema": candidate_schema()
                }
            }),
        };
        let value = serde_json::to_value(request).unwrap();

        assert_eq!(value["model"], "gpt-5-nano");
        assert_eq!(value["reasoning_effort"], "minimal");
        assert_eq!(value["response_format"]["type"], "json_schema");
        assert!(value.get("temperature").is_none());
    }
}
