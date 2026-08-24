use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{RequestBuilder, Response, StatusCode, header::HeaderMap};

const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const CONNECT_TIMEOUT_SECONDS: u64 = 5;
const MAX_ATTEMPTS: usize = 3;
const BASE_BACKOFF_MILLIS: u64 = 400;
const MAX_BACKOFF_MILLIS: u64 = 8_000;

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
                let delay = retry_delay(response.headers(), attempt);
                let _ = response.bytes().await;
                tokio::time::sleep(delay).await;
            }
            Ok(response) => return Ok(response),
            Err(error)
                if (error.is_connect() || error.is_timeout()) && attempt + 1 < MAX_ATTEMPTS =>
            {
                tokio::time::sleep(retry_delay(&HeaderMap::new(), attempt)).await;
            }
            Err(error) => return Err(error).context("AI request failed"),
        }
    }
    unreachable!("request loop always returns on its final attempt")
}

fn retry_delay(headers: &HeaderMap, attempt: usize) -> Duration {
    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map_or_else(
            || BASE_BACKOFF_MILLIS << attempt,
            |seconds| seconds.saturating_mul(1_000),
        );
    Duration::from_millis(retry_after.min(MAX_BACKOFF_MILLIS))
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

    #[test]
    fn retry_delay_honors_retry_after_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "2".parse().expect("valid header value"),
        );

        assert_eq!(retry_delay(&headers, 0), Duration::from_secs(2));
    }

    #[test]
    fn retry_delay_grows_exponentially_and_is_capped() {
        let headers = HeaderMap::new();

        assert_eq!(
            retry_delay(&headers, 0),
            Duration::from_millis(BASE_BACKOFF_MILLIS)
        );
        assert_eq!(
            retry_delay(&headers, 1),
            Duration::from_millis(BASE_BACKOFF_MILLIS * 2)
        );
        assert_eq!(
            retry_delay(&headers, 9),
            Duration::from_millis(MAX_BACKOFF_MILLIS)
        );
    }

    #[test]
    fn retry_delay_ignores_non_numeric_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "https://example.com".parse().expect("valid header value"),
        );

        assert_eq!(
            retry_delay(&headers, 0),
            Duration::from_millis(BASE_BACKOFF_MILLIS)
        );
    }
}
