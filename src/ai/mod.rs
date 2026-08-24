mod cache;
mod candidates;
mod client;
mod gemini;
mod openai;
pub(crate) mod prompts;

use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use crate::git::diff::generate_prompt;
use crate::types::{CommitType, MessageStyle, ModelType};

use self::prompts::{
    MAX_TOKENS_LONG, MAX_TOKENS_SHORT, PROMPT_VERSION, SYSTEM_PROMPT_LONG, SYSTEM_PROMPT_SHORT,
};

const DEFAULT_OPENAI_MODEL: &str = "gpt-5-nano";
const DEFAULT_GEMINI_MODEL: &str = "gemini-3.5-flash-lite";

// Models occasionally return empty or malformed output; one extra attempt
// recovers without surfacing an error to the user.
const GENERATION_ATTEMPTS: usize = 2;
const GENERATION_RETRY_PAUSE_MILLIS: u64 = 350;

pub struct GenerateOptions<'a> {
    pub diff: &'a str,
    pub diff_fingerprint: &'a str,
    pub commit_type: Option<CommitType>,
    pub files: &'a [String],
    pub branch_name: &'a str,
    pub message_style: MessageStyle,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct GenerationMetrics {
    pub model: String,
    pub duration: Duration,
    pub cache_hit: bool,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub candidates: Vec<String>,
    pub metrics: GenerationMetrics,
}

pub(crate) struct ProviderResponse {
    pub content: String,
    pub usage: TokenUsage,
}

pub struct Generator {
    model: ModelType,
    model_name: String,
    api_key: String,
    client: reqwest::Client,
}

impl Generator {
    pub fn new(model: ModelType, api_key: String) -> Result<Self> {
        if api_key.trim().is_empty() {
            bail!("No {} API key is configured", model.as_str());
        }

        Ok(Self {
            model,
            model_name: configured_model_name(model),
            api_key,
            client: client::build_client()?,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        match self.model {
            ModelType::Gemini => "Gemini",
            ModelType::Openai => "OpenAI",
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub async fn generate(
        &self,
        options: GenerateOptions<'_>,
        excluded_candidates: &[String],
        use_cache: bool,
    ) -> Result<GenerationResult> {
        let system_prompt = match options.message_style {
            MessageStyle::Long => SYSTEM_PROMPT_LONG,
            MessageStyle::Short => SYSTEM_PROMPT_SHORT,
        };
        let max_tokens = match options.message_style {
            MessageStyle::Long => MAX_TOKENS_LONG,
            MessageStyle::Short => MAX_TOKENS_SHORT,
        };
        let user_prompt = generate_prompt(
            options.diff,
            options.commit_type,
            options.files,
            options.branch_name,
            excluded_candidates,
        );
        let cache_key = cache::build_key(&[
            self.model.as_str(),
            &self.model_name,
            options.message_style.as_str(),
            PROMPT_VERSION,
            options.diff_fingerprint,
            &user_prompt,
        ]);

        if use_cache
            && excluded_candidates.is_empty()
            && let Some(cached) = cache::load(&cache_key)
        {
            let candidates = candidates::normalize(cached, options.message_style, &[])?;
            return Ok(GenerationResult {
                candidates,
                metrics: GenerationMetrics {
                    model: self.model_name.clone(),
                    duration: Duration::ZERO,
                    cache_hit: true,
                    usage: TokenUsage::default(),
                },
            });
        }

        let started_at = Instant::now();
        let (provider_response, candidates) = self
            .request_and_parse(
                system_prompt,
                &user_prompt,
                options.diff_fingerprint,
                max_tokens,
                options.message_style,
                excluded_candidates,
            )
            .await?;

        if use_cache && excluded_candidates.is_empty() {
            let _ = cache::store(&cache_key, &candidates);
        }

        Ok(GenerationResult {
            candidates,
            metrics: GenerationMetrics {
                model: self.model_name.clone(),
                duration: started_at.elapsed(),
                cache_hit: false,
                usage: provider_response.usage,
            },
        })
    }

    async fn request_and_parse(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        diff_fingerprint: &str,
        max_tokens: u32,
        message_style: MessageStyle,
        excluded_candidates: &[String],
    ) -> Result<(ProviderResponse, Vec<String>)> {
        for attempt in 0..GENERATION_ATTEMPTS {
            let provider_response = self
                .request_provider(system_prompt, user_prompt, diff_fingerprint, max_tokens)
                .await?;
            match candidates::parse(
                &provider_response.content,
                message_style,
                excluded_candidates,
            ) {
                Ok(candidates) => return Ok((provider_response, candidates)),
                Err(_) if attempt + 1 < GENERATION_ATTEMPTS => {
                    tokio::time::sleep(Duration::from_millis(GENERATION_RETRY_PAUSE_MILLIS)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("generation loop always returns on its final attempt")
    }

    async fn request_provider(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        diff_fingerprint: &str,
        max_tokens: u32,
    ) -> Result<ProviderResponse> {
        match self.model {
            ModelType::Gemini => {
                gemini::generate_commit_messages(
                    &self.client,
                    &self.api_key,
                    &self.model_name,
                    system_prompt,
                    user_prompt,
                    max_tokens,
                )
                .await
            }
            ModelType::Openai => {
                let prompt_cache_key = format!("autocommit-{diff_fingerprint}");
                openai::generate_commit_messages(
                    &self.client,
                    &self.api_key,
                    &self.model_name,
                    system_prompt,
                    user_prompt,
                    &prompt_cache_key,
                    max_tokens,
                )
                .await
            }
        }
    }
}

fn configured_model_name(model: ModelType) -> String {
    let (variable, default) = match model {
        ModelType::Openai => ("AUTOCOMMIT_OPENAI_MODEL", DEFAULT_OPENAI_MODEL),
        ModelType::Gemini => ("AUTOCOMMIT_GEMINI_MODEL", DEFAULT_GEMINI_MODEL),
    };
    std::env::var(variable)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}
