use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{RequestBuilder, Response, StatusCode};

const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const CONNECT_TIMEOUT_SECONDS: u64 = 5;
const MAX_ATTEMPTS: usize = 2;

pub(crate) fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .user_agent(concat!("autocommit-rs/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Failed to build the AI HTTP client")
}

pub(crate) async fn send_with_retry(request: RequestBuilder) -> Result<Response> {
    for attempt in 0..MAX_ATTEMPTS {
        let response = request
            .try_clone()
            .context("AI request body could not be retried")?
            .send()
            .await;

        match response {
            Ok(response)
                if is_retryable_status(response.status()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                let _ = response.bytes().await;
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Ok(response) => return Ok(response),
            Err(error)
                if (error.is_connect() || error.is_timeout()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(error) => return Err(error).context("AI request failed"),
        }
    }
    unreachable!("request loop always returns on its final attempt")
}

pub(crate) async fn response_body(provider: &str, response: Response) -> Result<String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read {provider} response"))?;
    if !status.is_success() {
        let detail = utf8_prefix(body.trim(), 500);
        bail!("{provider} API returned HTTP {status}: {detail}");
    }
    Ok(body)
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn utf8_prefix(value: &str, max_bytes: usize) -> &str {
    let mut end = value.len().min(max_bytes);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_retryable_status_codes_without_network_access() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }
}
