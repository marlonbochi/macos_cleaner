//! Minimal JSON-over-HTTP client shared by the AI adapters: timeouts, retries with exponential
//! backoff on transient failures, and mapping of HTTP statuses to `AiError`.

use std::time::Duration;

use cleaner_core::ports::ai::AiError;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct JsonHttp {
    client: Client,
    retries: u32,
    backoff: Duration,
}

impl JsonHttp {
    pub(crate) fn new(timeout: Duration, retries: u32, backoff: Duration) -> Result<Self, AiError> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(concat!("disk-cleaner/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AiError::Network(e.to_string()))?;
        Ok(Self {
            client,
            retries,
            backoff,
        })
    }

    /// POSTs `body` as JSON and returns the JSON response.
    pub(crate) fn post(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &Value,
    ) -> Result<Value, AiError> {
        self.send(headers, || self.client.post(url).json(body))
    }

    /// GETs `url` and returns the JSON response.
    pub(crate) fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, AiError> {
        self.send(headers, || self.client.get(url))
    }

    fn send(
        &self,
        headers: &[(&str, &str)],
        build: impl Fn() -> reqwest::blocking::RequestBuilder,
    ) -> Result<Value, AiError> {
        let mut attempt = 0;
        loop {
            let mut request = build();
            for (name, value) in headers {
                request = request.header(*name, *value);
            }
            let outcome = match request.send() {
                Ok(response) => Self::read(response),
                Err(e) if e.is_timeout() || e.is_connect() => Err(Transient::Retry(
                    AiError::Network(e.without_url().to_string()),
                )),
                Err(e) => Err(Transient::Fatal(AiError::Network(
                    e.without_url().to_string(),
                ))),
            };
            match outcome {
                Ok(value) => return Ok(value),
                Err(Transient::Retry(error)) if attempt < self.retries => {
                    tracing::debug!(%error, attempt, "retrying AI request");
                    std::thread::sleep(self.backoff * 2u32.saturating_pow(attempt));
                    attempt += 1;
                }
                Err(Transient::Retry(error) | Transient::Fatal(error)) => return Err(error),
            }
        }
    }

    fn read(response: reqwest::blocking::Response) -> Result<Value, Transient> {
        let status = response.status();
        let text = response
            .text()
            .map_err(|e| Transient::Retry(AiError::Network(e.without_url().to_string())))?;
        if status.is_success() {
            return serde_json::from_str(&text)
                .map_err(|e| Transient::Fatal(AiError::InvalidResponse(e.to_string())));
        }
        let error = match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AiError::Unauthorized,
            StatusCode::TOO_MANY_REQUESTS => AiError::RateLimited,
            _ => AiError::Provider {
                status: status.as_u16(),
                message: error_message(&text),
            },
        };
        // 429, 5xx and Anthropic's 529 (overloaded) are worth retrying.
        if status == StatusCode::TOO_MANY_REQUESTS
            || status.is_server_error()
            || status.as_u16() == 529
        {
            Err(Transient::Retry(error))
        } else {
            Err(Transient::Fatal(error))
        }
    }
}

enum Transient {
    Retry(AiError),
    Fatal(AiError),
}

/// Extracts a readable message from the error bodies used by the supported providers.
fn error_message(body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let message = parsed.as_ref().and_then(|v| {
        v.pointer("/error/message")
            .or_else(|| v.get("error"))
            .or_else(|| v.get("message"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let message = message.unwrap_or_else(|| body.trim().to_owned());
    message.chars().take(500).collect()
}

/// Text of the first `text`-typed item, used by several response formats.
pub(crate) fn require_str<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, AiError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| AiError::InvalidResponse(format!("missing {pointer}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_messages_from_common_error_shapes() {
        assert_eq!(
            error_message(r#"{"error":{"message":"bad model"}}"#),
            "bad model"
        );
        assert_eq!(
            error_message(r#"{"error":"model not found"}"#),
            "model not found"
        );
        assert_eq!(error_message("  plain text  "), "plain text");
    }
}
