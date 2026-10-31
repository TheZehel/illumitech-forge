//! Conservative retry policy. Only identified transient native failures retry automatically.
use crate::{clients::structured_ok, process::ProcessResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Transient,
    Permission,
    Authentication,
    Contract,
    Evidence,
    Execution,
    Timeout,
    Cancelled,
    OutputLimit,
}
impl FailureKind {
    pub fn retryable(self) -> bool {
        matches!(self, Self::Transient | Self::Execution)
    }
}
fn classify_error(v: &Value) -> Option<FailureKind> {
    // Inspect error payloads only, never natural-language claims in agent messages.
    let s = v.to_string().to_ascii_lowercase();
    if [
        "permission_denied",
        "permission denied",
        "not permitted",
        "sandbox",
        "approval required",
        "approval_required",
        "approval denied",
        "approval rejected",
        "automatic approval review",
        "eacces",
        "access_denied",
    ]
    .iter()
    .any(|p| s.contains(p))
    {
        Some(FailureKind::Permission)
    } else if [
        "unauthorized",
        "authentication",
        "invalid_api_key",
        "invalid api key",
        "login required",
        "not authenticated",
        "token expired",
        "401",
        "403",
    ]
    .iter()
    .any(|p| s.contains(p))
    {
        Some(FailureKind::Authentication)
    } else if [
        "rate_limit",
        "rate limit",
        "429",
        "server_error",
        "internal_server_error",
        "service_unavailable",
        "connection_error",
        "connection reset",
        "temporarily unavailable",
        "stream disconnected",
        "502",
        "503",
        "504",
    ]
    .iter()
    .any(|p| s.contains(p))
    {
        Some(FailureKind::Transient)
    } else {
        None
    }
}

pub fn assess(client: Option<&str>, r: &ProcessResult) -> Result<(), FailureKind> {
    if let Some(kind) = classify_error(&Value::String(r.stderr.clone())).filter(|k| !k.retryable())
    {
        return Err(kind);
    }
    let mut protocol_errors = Vec::new();
    if let Some(client) = client {
        let events: Vec<Value> = if client == "claude" {
            vec![serde_json::from_str(&r.stdout).map_err(|_| {
                classify_error(&Value::String(r.stderr.clone()))
                    .filter(|k| !k.retryable())
                    .unwrap_or(FailureKind::Contract)
            })?]
        } else {
            r.stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()
                .map_err(|_| FailureKind::Contract)?
        };
        for e in &events {
            if e["permission_denials"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
            {
                return Err(FailureKind::Permission);
            }
            if e["type"] == "error" || e["type"] == "turn.failed" || e["is_error"] == true {
                let payload = e
                    .get("error")
                    .or_else(|| e.get("message"))
                    .or_else(|| e.get("errors"))
                    .or_else(|| e.get("subtype"))
                    .unwrap_or(&Value::Null);
                protocol_errors.push(classify_error(payload).unwrap_or(FailureKind::Contract));
            }
        }
        if let Some(kind) = protocol_errors.iter().find(|k| !k.retryable()) {
            return Err(*kind);
        }
        if let Some(kind) =
            classify_error(&Value::String(r.stderr.clone())).filter(|k| !k.retryable())
        {
            return Err(kind);
        }
    } else if let Some(kind) =
        classify_error(&Value::String(r.stderr.clone())).filter(|k| !k.retryable())
    {
        return Err(kind);
    }
    if r.truncated || r.reason == "output_limit" {
        return Err(FailureKind::OutputLimit);
    }
    match r.reason.as_str() {
        "cancelled" => return Err(FailureKind::Cancelled),
        "timeout" => return Err(FailureKind::Timeout),
        "exited" => {}
        _ => return Err(FailureKind::Contract),
    }
    if let Some(kind) = protocol_errors.first() {
        return Err(*kind);
    }
    if let Some(client) = client {
        structured_ok(client, r)
            .map(|_| ())
            .map_err(|_| FailureKind::Contract)
    } else if r.passed() {
        Ok(())
    } else {
        Err(FailureKind::Execution)
    }
}

/// A denied spawn is a permission block even when no protocol could be produced.
pub fn spawn_failure(error: &anyhow::Error) -> FailureKind {
    if error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
    {
        FailureKind::Permission
    } else {
        classify_error(&Value::String(error.to_string()))
            .filter(|k| !k.retryable())
            .unwrap_or(FailureKind::Contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn result(stdout: &str) -> ProcessResult {
        ProcessResult {
            code: Some(0),
            reason: "exited".into(),
            stdout: stdout.into(),
            stderr: String::new(),
            elapsed_ms: 0,
            truncated: false,
        }
    }
    #[test]
    fn retry_policy_refuses_permission_and_contract_failures() {
        for (error, expected) in [
            ("permission_denied", FailureKind::Permission),
            ("401 authentication failed", FailureKind::Authentication),
            ("unknown_failure", FailureKind::Contract),
        ] {
            let r = result(
                &serde_json::json!({"type":"turn.failed","error":{"message":error}}).to_string(),
            );
            assert_eq!(assess(Some("codex"), &r), Err(expected));
            assert!(!expected.retryable());
        }
        assert_eq!(
            assess(
                Some("codex"),
                &result(r#"{"type":"turn.failed","error":{"code":"rate_limit_exceeded"}}"#)
            ),
            Err(FailureKind::Transient)
        );
        let r = result(
            "{\"type\":\"error\",\"message\":\"permission_denied\"}\n{\"type\":\"turn.failed\",\"error\":{\"code\":\"server_error\"}}",
        );
        assert_eq!(assess(Some("codex"), &r), Err(FailureKind::Permission));
        assert_eq!(
            assess(Some("codex"), &result("All done")),
            Err(FailureKind::Contract)
        );
        assert!(!FailureKind::Evidence.retryable());
    }
    #[test]
    fn plain_text_never_completes_a_task() {
        for text in [
            "All done; tests passed",
            "{\"type\":\"item.completed\",\"item\":{\"text\":\"all done\"}}",
            "{\"type\":\"turn.started\"}",
        ] {
            assert_eq!(
                assess(Some("codex"), &result(text)),
                Err(FailureKind::Contract)
            );
        }
    }
    #[test]
    fn auth_and_permission_failures_before_protocol_are_classified_without_retry() {
        let mut r = result("client setup failed");
        r.stderr = "authentication failed: login required".into();
        assert_eq!(assess(Some("codex"), &r), Err(FailureKind::Authentication));
        r.stderr = "permission denied".into();
        assert_eq!(assess(Some("claude"), &r), Err(FailureKind::Permission));
        let denied =
            anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        assert_eq!(spawn_failure(&denied), FailureKind::Permission);
        let r = result(
            r#"{"type":"turn.failed","error":{"code":"server_error"},"usage":{"input_tokens":401}}"#,
        );
        assert_eq!(assess(Some("codex"), &r), Err(FailureKind::Transient));
    }
}
