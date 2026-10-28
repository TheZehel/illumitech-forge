use crate::process::ProcessResult;
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn codex(prompt: &str, write: bool, model: Option<&str>) -> Vec<String> {
    let mut a = vec![
        "codex".into(),
        "exec".into(),
        "--json".into(),
        "--sandbox".into(),
        if write {
            "workspace-write".into()
        } else {
            "read-only".into()
        },
    ];
    if let Some(m) = model {
        a.extend(["--model".into(), m.into()]);
    }
    a.push(prompt.into());
    a
}
pub fn claude(prompt: &str, write: bool, model: Option<&str>) -> Vec<String> {
    let mut a = vec![
        "claude".into(),
        "--print".into(),
        "--output-format".into(),
        "json".into(),
        "--permission-mode".into(),
        "dontAsk".into(),
    ];
    if !write {
        a.extend(["--tools".into(), "Read,Glob,Grep".into()]);
    }
    // dontAsk preserves configured allow rules and denies new permissions. It never bypasses checks.
    if let Some(m) = model {
        a.extend(["--model".into(), m.into()]);
    }
    a.push(prompt.into());
    a
}
pub fn structured_ok(client: &str, r: &ProcessResult) -> Result<Value> {
    ensure!(r.passed(), "client process did not succeed: {}", r.reason);
    if client == "claude" {
        let v: Value = serde_json::from_str(&r.stdout)?;
        ensure!(
            v["type"] == "result" && v["is_error"] == false && v["subtype"] == "success",
            "Claude returned non-success result"
        );
        ensure!(
            v["permission_denials"]
                .as_array()
                .is_none_or(|a| a.is_empty()),
            "Claude permission denied"
        );
        Ok(serde_json::json!({"session_id":v["session_id"],"subtype":v["subtype"]}))
    } else {
        let events: Vec<Value> = r
            .stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<std::result::Result<_, _>>()?;
        ensure!(
            !events
                .iter()
                .any(|e| e["type"] == "turn.failed" || e["type"] == "error"),
            "Codex reported failure"
        );
        ensure!(
            events.iter().any(|e| e["type"] == "turn.completed"),
            "missing Codex turn.completed"
        );
        let thread = events
            .iter()
            .find(|e| e["type"] == "thread.started")
            .map(|e| e["thread_id"].clone());
        Ok(serde_json::json!({"thread_id":thread,"event_count":events.len()}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_config_and_permissions() {
        for a in [codex("x", true, None), claude("x", true, None)] {
            let s = a.join(" ");
            for forbidden in [
                "ignore-user-config",
                "ephemeral",
                "bypass",
                "skip-permissions",
                "fallback-model",
            ] {
                assert!(!s.contains(forbidden));
            }
        }
    }
    #[test]
    fn false_text_not_structured_success() {
        let r = ProcessResult {
            code: Some(0),
            reason: "exited".into(),
            stdout: "Everything passed".into(),
            stderr: String::new(),
            elapsed_ms: 0,
            truncated: false,
        };
        assert!(structured_ok("codex", &r).is_err());
        assert!(structured_ok("claude", &r).is_err());
    }
}
