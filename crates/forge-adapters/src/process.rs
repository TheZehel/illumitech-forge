use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, sync::watch};
#[cfg(unix)]
use {std::process::Stdio, tokio::process::Command};

pub const OUTPUT_LIMIT: usize = 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub code: Option<i32>,
    pub reason: String,
    pub stdout: String,
    pub stderr: String,
    pub elapsed_ms: u128,
    pub truncated: bool,
}
impl ProcessResult {
    pub fn passed(&self) -> bool {
        self.code == Some(0) && self.reason == "exited" && !self.truncated
    }
}
const SENSITIVE_FIELDS: &[&str] = &[
    "authorization",
    "api_key",
    "apikey",
    "access_token",
    "accesstoken",
    "refresh_token",
    "refreshtoken",
    "id_token",
    "idtoken",
    "password",
    "secret",
    "token",
    "credential",
    "credentials",
    "client_secret",
    "clientsecret",
    "private_key",
    "privatekey",
    "session_token",
];
fn redact_plain(text: &str, secrets: &[String]) -> String {
    let mut clean = text.to_owned();
    for value in secrets {
        clean = clean.replace(value, "[REDACTED]");
    }
    // Broken or embedded JSON cannot be sanitized structurally. Detect named
    // fields even across newlines/escaped quotes and redact the whole fragment.
    // This changes only the plain fallback; valid event framing stays intact.
    if clean.contains('"') {
        let detection = clean.to_ascii_lowercase().replace('\\', "");
        if SENSITIVE_FIELDS.iter().any(|key| {
            let needle = format!("\"{key}\"");
            detection.match_indices(&needle).any(|(offset, _)| {
                detection[offset + needle.len()..]
                    .trim_start()
                    .starts_with(':')
            })
        }) {
            return "[REDACTED sensitive fragment]".into();
        }
    }
    clean
        .lines()
        .map(|line| {
            let lower = line.to_lowercase();
            if [
                "authorization:",
                "bearer ",
                "api_key=",
                "password=",
                "secret=",
                "token=",
            ]
            .iter()
            .any(|part| lower.contains(part))
            {
                "[REDACTED sensitive line]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn redact_value(value: &mut serde_json::Value, secrets: &[String]) {
    match value {
        serde_json::Value::String(text) => {
            let redacted = redact_plain(text, secrets);
            // The legacy plain log formatter normalizes line endings. Safe protocol
            // strings must retain their exact bytes (issue hashes and PR body readback).
            let normalized = text.lines().collect::<Vec<_>>().join("\n");
            if redacted != normalized {
                *text = redacted;
            }
        }
        serde_json::Value::Array(values) => {
            values.iter_mut().for_each(|v| redact_value(v, secrets))
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let sensitive = SENSITIVE_FIELDS.contains(&key.to_ascii_lowercase().as_str());
                if sensitive {
                    *value = serde_json::json!("[REDACTED]");
                } else {
                    redact_value(value, secrets);
                }
            }
        }
        _ => {}
    }
}
/// Preserve structured client framing: sanitize values, never replace a JSON event
/// with an unparseable line. Terminal/error event types remain authoritative.
pub fn redact(text: &str) -> String {
    // Snapshot secret values once per bounded capture. Re-reading the environment
    // for every plain output line made a flood of short lines effectively unbounded.
    let secrets = std::env::vars()
        .filter(|(key, value)| {
            value.len() >= 8
                && ["TOKEN", "SECRET", "PASSWORD", "API_KEY"]
                    .iter()
                    .any(|part| key.to_uppercase().contains(part))
        })
        .map(|(_, value)| value)
        .collect::<Vec<_>>();
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) {
        redact_value(&mut value, &secrets);
        return serde_json::to_string(&value).expect("serializable redacted JSON");
    }
    let mut output = Vec::new();
    let mut plain = String::new();
    for line in text.lines() {
        let structured = matches!(
            line.trim_start().as_bytes().first(),
            Some(b'{' | b'[' | b'"')
        );
        let parsed = structured
            .then(|| serde_json::from_str::<serde_json::Value>(line))
            .and_then(Result::ok);
        if let Some(mut value) = parsed {
            if !plain.is_empty() {
                output.push(redact_plain(&plain, &secrets));
                plain.clear();
            }
            redact_value(&mut value, &secrets);
            output.push(serde_json::to_string(&value).expect("serializable redacted event"));
        } else {
            plain.push_str(line);
            plain.push('\n');
        }
    }
    if !plain.is_empty() {
        output.push(redact_plain(&plain, &secrets));
    }
    output.join("\n")
}

async fn drain<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    overflow: watch::Sender<bool>,
) -> Result<(Vec<u8>, bool)> {
    let mut bytes = Vec::new();
    let mut block = [0u8; 8192];
    let mut truncated = false;
    loop {
        let n = reader.read(&mut block).await?;
        if n == 0 {
            break;
        }
        let keep = n.min(OUTPUT_LIMIT.saturating_sub(bytes.len()));
        bytes.extend_from_slice(&block[..keep]);
        if keep < n {
            truncated = true;
            let _ = overflow.send(true);
        }
    }
    Ok((bytes, truncated))
}
/// RAII group cleanup also runs if the supervisor future is aborted.
#[cfg(unix)]
struct Group(u32);
#[cfg(unix)]
impl Group {
    fn terminate(&self, force: bool) {
        let sig = if force { libc::SIGKILL } else { libc::SIGTERM };
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.0 as i32), sig);
        }
    }
}
#[cfg(unix)]
impl Drop for Group {
    fn drop(&mut self) {
        self.terminate(true);
    }
}
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::pid_alive;
#[cfg(unix)]
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return true;
    }
    // Permission denied is not proof of death.
    unsafe {
        libc::kill(pid as i32, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
}

/// Observe a recorded Unix process group without terminating it. A dead leader does
/// not establish that descendants have stopped. Unknown ownership is fail-closed.
#[cfg(unix)]
pub fn group_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return true;
    }
    unsafe {
        libc::kill(-(pid as i32), 0) == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
}
#[cfg(windows)]
pub fn group_alive(pid: u32) -> bool {
    pid_alive(pid)
}

pub async fn run(
    argv: &[String],
    cwd: &Path,
    seconds: u64,
    cancel: watch::Receiver<bool>,
) -> Result<ProcessResult> {
    run_observed(argv, cwd, seconds, cancel, |_| Ok(())).await
}

/// A long-lived service child stays in a private kill-on-close Job. Unlike task
/// execution, there is no task deadline or captured output: the daemon owns its
/// bounded service logs, and duplicate console output is drained without storage.
#[cfg(windows)]
pub async fn service_child(
    argv: &[String],
    cwd: &Path,
    mut cancel: watch::Receiver<bool>,
) -> Result<Option<i32>> {
    ensure!(!argv.is_empty(), "empty service command");
    let executable = crate::clients::resolve_command(&argv[0], cwd)?;
    let (mut child, group) = windows::spawn(&executable, &argv[1..], cwd)?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    // Keep these futures inline: cancellation never leaves detached pipe readers.
    let out = async { tokio::io::copy(&mut stdout, &mut tokio::io::sink()).await };
    let err = async { tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await };
    let wait = async {
        tokio::select! {
            status = child.wait() => Ok(Some(status?.code().unwrap_or(1))),
            _ = async { while !*cancel.borrow() { if cancel.changed().await.is_err() { break; } } } => Ok(None),
        }
    };
    let drain = async {
        tokio::try_join!(out, err)?;
        std::future::pending::<Result<Option<i32>>>().await
    };
    let result = tokio::select! { result = wait => result, result = drain => result };
    group.terminate(true);
    result
}
pub async fn run_observed(
    argv: &[String],
    cwd: &Path,
    seconds: u64,
    mut cancel: watch::Receiver<bool>,
    on_spawn: impl FnOnce(u32) -> Result<()>,
) -> Result<ProcessResult> {
    ensure!(!argv.is_empty(), "empty command");
    let started = Instant::now();
    let executable = crate::clients::resolve_command(&argv[0], cwd)?;
    #[cfg(unix)]
    let (mut child, group) = {
        let mut command = Command::new(executable);
        command
            .args(&argv[1..])
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command.process_group(0);
        let child = command.spawn()?;
        let group = Group(
            child
                .id()
                .ok_or_else(|| anyhow::anyhow!("missing child PID"))?,
        );
        (child, group)
    };
    #[cfg(windows)]
    let (mut child, group) = windows::spawn(&executable, &argv[1..], cwd)?;
    on_spawn(
        child
            .id()
            .ok_or_else(|| anyhow::anyhow!("missing child PID"))?,
    )?;
    let (overflow_tx, mut overflow_rx) = watch::channel(false);
    let stdout = tokio::spawn(drain(child.stdout.take().unwrap(), overflow_tx.clone()));
    let stderr = tokio::spawn(drain(child.stderr.take().unwrap(), overflow_tx));
    let (status, reason) = tokio::select! {
        s=child.wait()=>(Some(s?),"exited"),
        _=tokio::time::sleep(Duration::from_secs(seconds))=>(None,"timeout"),
        _=async {while !*cancel.borrow(){if cancel.changed().await.is_err(){std::future::pending::<()>().await;}}}=>(None,"cancelled"),
        _=async {while !*overflow_rx.borrow(){if overflow_rx.changed().await.is_err(){std::future::pending::<()>().await;}}}=>(None,"output_limit"),
    };
    let status = if status.is_none() {
        group.terminate(false);
        match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
            Ok(s) => Some(s?),
            Err(_) => {
                group.terminate(true);
                Some(child.wait().await?)
            }
        }
    } else {
        status
    };
    // Descendants may retain pipes after the direct child exits.
    group.terminate(true);
    let (out, out_truncated) = tokio::time::timeout(Duration::from_secs(3), stdout).await???;
    let (err, err_truncated) = tokio::time::timeout(Duration::from_secs(3), stderr).await???;
    Ok(ProcessResult {
        code: status.and_then(|s| s.code()),
        reason: reason.into(),
        stdout: redact(&String::from_utf8_lossy(&out)),
        stderr: redact(&String::from_utf8_lossy(&err)),
        elapsed_ms: started.elapsed().as_millis(),
        truncated: out_truncated || err_truncated,
    })
}

/// Console interrupt without replacing the caller's model or permission policy.
pub async fn interrupt() {
    #[cfg(windows)]
    {
        if let Ok(mut breaks) = tokio::signal::windows::ctrl_break() {
            tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=breaks.recv()=>{} }
        } else {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
    #[cfg(unix)]
    {
        if let Ok(mut term) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
        } else {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_kills_group() {
        let (_tx, rx) = watch::channel(false);
        let r = run(
            &["sh".into(), "-c".into(), "sleep 60 & wait".into()],
            Path::new("/tmp"),
            1,
            rx,
        )
        .await
        .unwrap();
        assert_eq!(r.reason, "timeout");
        assert!(r.elapsed_ms < 6000);
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn excessive_output_is_not_success() {
        let (_tx, rx) = watch::channel(false);
        let r = run(
            &["sh".into(), "-c".into(), "yes x".into()],
            Path::new("/tmp"),
            10,
            rx,
        )
        .await
        .unwrap();
        assert!(!r.passed());
        assert!(r.stdout.len() <= OUTPUT_LIMIT);
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_is_bounded() {
        let (tx, rx) = watch::channel(false);
        tx.send(true).unwrap();
        let r = run(&["sleep".into(), "60".into()], Path::new("/tmp"), 60, rx)
            .await
            .unwrap();
        assert_eq!(r.reason, "cancelled");
    }
    #[test]
    fn structured_safe_strings_preserve_exact_content_for_hashes() {
        let input = serde_json::json!({"body":"Demand\r\n\nwith trailing newline\n", "empty":"\n"});
        let output: serde_json::Value = serde_json::from_str(&redact(&input.to_string())).unwrap();
        assert_eq!(input, output);
    }
    #[test]
    fn redacts_auth() {
        assert_eq!(
            redact("Authorization: Bearer abc"),
            "[REDACTED sensitive line]"
        );
    }
    #[test]
    fn structured_logs_keep_valid_json_and_redact_secrets() {
        let log = concat!(
            "{\"type\":\"thread.started\",\"thread_id\":\"sanitized-thread\"}\n",
            "{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"text\":\"token=fixture-only\\ncontinued\",\"nested\":[{\"password\":\"fixture-only\"}]}}\n",
            "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":25,\"cached_input_tokens\":10,\"output_tokens\":5}}"
        );
        let clean = redact(log);
        assert!(!clean.contains("fixture-only"));
        let events: Vec<serde_json::Value> = clean
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["thread_id"], "sanitized-thread");
        assert_eq!(events[1]["item"]["id"], "item_1");
        assert_eq!(events[2]["type"], "turn.completed");
        assert_eq!(events[2]["usage"]["cached_input_tokens"], 10);
        let result = ProcessResult {
            code: Some(0),
            reason: "exited".into(),
            stdout: clean,
            stderr: String::new(),
            elapsed_ms: 0,
            truncated: false,
        };
        assert!(crate::clients::structured_ok("codex", &result).is_ok());
        // Sanitized reproduction of the blocked FORGE-020 shape: exit 0,
        // sensitive tool output, no terminal completion. Redaction cannot repair it.
        let mut incomplete = result.clone();
        incomplete.stdout = redact(&log.lines().take(2).collect::<Vec<_>>().join("\n"));
        assert!(crate::clients::structured_ok("codex", &incomplete).is_err());
        let failure = redact(
            r#"{"type":"turn.failed","error":{"code":"permission_denied","message":"Authorization: Bearer fixture-only"}}"#,
        );
        let v: serde_json::Value = serde_json::from_str(&failure).unwrap();
        assert_eq!(v["error"]["code"], "permission_denied");
        assert_eq!(v["type"], "turn.failed");
        assert!(!failure.contains("fixture-only"));
    }
    #[test]
    fn pretty_json_nested_secrets_and_escaped_values_remain_json() {
        let input = serde_json::json!({"type":"result","session_id":"c","is_error":false,
            "subtype":"success","result":"quoted \"value\" and \\ path\napi_key=fixture-only",
            "array":[{"access_token":"fixture-only","usage":{"input_tokens":9}}]});
        let clean = redact(&serde_json::to_string_pretty(&input).unwrap());
        let v: serde_json::Value = serde_json::from_str(&clean).unwrap();
        assert_eq!(v["session_id"], "c");
        assert_eq!(v["array"][0]["usage"]["input_tokens"], 9);
        assert!(!clean.contains("fixture-only"));
        assert_eq!(redact(&clean), clean);
    }
}

#[cfg(test)]
mod credential_value_tests {
    use super::*;
    #[test]
    fn malformed_and_nested_json_text_does_not_leak_named_credentials() {
        for fragment in [
            "{\"type\":\"tool\",\"password\":{\"nested\":\"fixture-not-for-logs\"",
            "{\"PASSWORD\"\n : [\"fixture-not-for-logs\"",
            r#"{\"access_token\":\"fixture-not-for-logs"#,
        ] {
            let clean = redact(fragment);
            assert!(!clean.contains("fixture-not-for-logs"), "{clean}");
            assert_eq!(redact(&clean), clean);
            let outer = serde_json::json!({"type":"turn.failed","id":"event-1","error":{"message":fragment}});
            let nested: serde_json::Value =
                serde_json::from_str(&redact(&outer.to_string())).unwrap();
            assert_eq!(nested["type"], "turn.failed");
            assert_eq!(nested["id"], "event-1");
            assert!(!nested.to_string().contains("fixture-not-for-logs"));
        }
    }
    #[test]
    fn nested_credential_values_are_fully_redacted_without_damaging_envelopes() {
        let original = serde_json::json!({"type":"turn.completed","id":"event-1","usage":{"input_tokens":5,"output_tokens":2},"token":{"nested":["credential-value"]},"password":["secret-one",{"extra":"secret-two"}]});
        let clean: serde_json::Value =
            serde_json::from_str(&redact(&original.to_string())).unwrap();
        assert_eq!(clean["type"], "turn.completed");
        assert_eq!(clean["id"], "event-1");
        assert_eq!(clean["usage"]["input_tokens"], 5);
        assert_eq!(clean["token"], "[REDACTED]");
        assert_eq!(clean["password"], "[REDACTED]");
        assert!(!clean.to_string().contains("credential-value"));
    }
}

#[cfg(test)]
mod flood_tests {
    #[test]
    fn flood_redaction_keeps_safe_diagnostics_and_jsonl_boundaries() {
        let safe = "safe diagnostic\n".repeat(60000);
        let input = format!(
            "{safe}\n{{\"type\":\"failure\",\"id\":7,\"credential\":[{{\"nested\":\"synthetic-sensitive-value\"}}]}}\nplain final diagnostic\n"
        );
        let clean = super::redact(&input);
        assert!(clean.starts_with(&format!("{safe}\n")));
        let frame = clean.lines().find(|line| line.starts_with('{')).unwrap();
        let frame: serde_json::Value = serde_json::from_str(frame).unwrap();
        assert_eq!(frame["type"], "failure");
        assert_eq!(frame["id"], 7);
        assert_eq!(frame["credential"], "[REDACTED]");
        assert!(!clean.contains("synthetic-sensitive-value"));
        assert!(clean.ends_with("plain final diagnostic"));
    }
}
