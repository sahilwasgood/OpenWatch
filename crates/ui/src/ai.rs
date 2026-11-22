//! Opt-in, BYOK interpretation of a single raw frame.
//!
//! With no key, [`explain`] returns [`AiError::Disabled`] before any transport
//! is touched: no health data leaves the device without explicit opt-in. The
//! HTTP function is injectable per thread so tests can count requests without
//! opening a socket.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{json, Value};
use thiserror::Error;

/// Errors from the interpretation path.
#[derive(Debug, Error)]
pub enum AiError {
    /// No key configured. Interpretation is off; nothing was sent.
    #[error("AI interpretation is disabled (no key)")]
    Disabled,
    #[error("AI transport error: {0}")]
    Transport(String),
    #[error("unexpected AI response: {0}")]
    Response(String),
}

/// A user-supplied OpenAI-compatible endpoint and key. No key means off.
#[derive(Debug, Clone, Default)]
pub struct AiConfig {
    pub endpoint: String,
    pub key: Option<String>,
}

/// Interpretation is enabled only when a non-empty key is present.
pub fn enabled(config: &AiConfig) -> bool {
    config
        .key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty())
}

/// The exact JSON body shown to the user before anything is sent. It carries
/// only one frame and the user's note — no other stored health data.
pub fn payload(frame_hex: &str, note: &str) -> String {
    json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": "Explain this smartwatch protocol frame in one short paragraph. Use only the frame and note supplied; do not infer health facts."
            },
            { "role": "user", "content": format!("Frame: {frame_hex}\nNote: {note}") }
        ],
        "temperature": 0
    })
    .to_string()
}

/// HTTP function used by [`explain`] when enabled: (endpoint, key, body).
pub type PostJson = fn(&str, &str, &str) -> Result<String, AiError>;

thread_local! {
    static POST: RefCell<PostJson> = const { RefCell::new(http_post) };
}

/// Replace the transport on this thread. Used by tests to inject a mock and
/// assert that a disabled config performs zero requests.
#[doc(hidden)]
pub fn set_transport(post: PostJson) {
    POST.with(|slot| *slot.borrow_mut() = post);
}

/// Ask the configured endpoint to explain one frame. Returns [`AiError::Disabled`]
/// without touching the transport when no key is configured.
pub async fn explain(config: &AiConfig, frame_hex: &str) -> Result<String, AiError> {
    explain_blocking(config, frame_hex)
}

/// Same path as [`explain`] for callers without an async runtime (the Slint
/// host calls it directly). The no-key guard is shared by both.
pub fn explain_blocking(config: &AiConfig, frame_hex: &str) -> Result<String, AiError> {
    if !enabled(config) {
        return Err(AiError::Disabled);
    }
    let body = payload(frame_hex, "");
    let key = config.key.as_deref().unwrap_or_default();
    let raw = POST.with(|slot| (slot.borrow())(&config.endpoint, key, &body))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|error| AiError::Response(format!("invalid JSON: {error}")))?;
    value["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AiError::Response("missing choices[0].message.content".into()))
}

/// Minimal HTTP/1.1 POST for a local OpenAI-compatible server. The host may
/// inject a TLS-capable transport for `https://` endpoints.
/// shortcut: no TLS or chunked-response handling here, add a TLS transport when
/// hosted endpoints must be supported.
fn http_post(endpoint: &str, key: &str, body: &str) -> Result<String, AiError> {
    // Never let an endpoint or key smuggle extra request headers.
    if endpoint.contains(['\r', '\n']) || key.contains(['\r', '\n']) {
        return Err(AiError::Transport("invalid endpoint or key".into()));
    }
    let rest = endpoint.strip_prefix("http://").ok_or_else(|| {
        AiError::Transport("https endpoints need a host-provided transport".into())
    })?;
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return Err(AiError::Transport("empty endpoint".into()));
    }
    let address = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:80")
    };
    let mut stream = TcpStream::connect(&address)
        .map_err(|error| AiError::Transport(format!("connect: {error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| AiError::Transport(format!("timeout: {error}")))?;
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(|error| AiError::Transport(format!("write: {error}")))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| AiError::Transport(format!("read: {error}")))?;
    let start = response
        .find("\r\n\r\n")
        .map(|index| index + 4)
        .unwrap_or(0);
    Ok(response[start..].to_string())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use futures::executor::block_on;

    use super::*;

    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }

    fn config(key: Option<&str>) -> AiConfig {
        AiConfig {
            endpoint: "http://127.0.0.1:8000/v1/chat/completions".into(),
            key: key.map(str::to_string),
        }
    }

    #[test]
    fn enabled_requires_a_key() {
        assert!(!enabled(&config(None)));
        assert!(!enabled(&config(Some("   "))));
        assert!(enabled(&config(Some("sk-local"))));
    }

    #[test]
    fn payload_carries_the_exact_frame_and_note() {
        let body = payload("FEEA200553", "Why two bytes?");
        assert!(body.contains("FEEA200553"), "{body}");
        assert!(body.contains("Why two bytes?"), "{body}");
    }

    #[test]
    fn no_key_disables_explain_with_zero_requests() {
        CALLS.with(|calls| calls.set(0));
        set_transport(|_, _, _| {
            CALLS.with(|calls| calls.set(calls.get() + 1));
            Ok("{}".into())
        });
        let result = block_on(explain(&config(None), "FEEA200553"));
        assert!(matches!(result, Err(AiError::Disabled)), "{result:?}");
        assert_eq!(
            CALLS.with(|calls| calls.get()),
            0,
            "no request may leave the device without opt-in"
        );
    }

    #[test]
    fn enabled_explain_sends_only_the_frame_and_reads_the_reply() {
        CALLS.with(|calls| calls.set(0));
        set_transport(|endpoint, key, body| {
            CALLS.with(|calls| calls.set(calls.get() + 1));
            assert!(endpoint.contains("chat/completions"));
            assert_eq!(key, "sk-local");
            assert!(body.contains("FEEA200553"), "{body}");
            assert!(!body.contains("7667"), "no other health data may be sent");
            Ok(r#"{"choices":[{"message":{"content":"A steps read."}}]}"#.into())
        });
        let result = block_on(explain(&config(Some("sk-local")), "FEEA200553")).unwrap();
        assert_eq!(result, "A steps read.");
        assert_eq!(CALLS.with(|calls| calls.get()), 1);
    }
}
