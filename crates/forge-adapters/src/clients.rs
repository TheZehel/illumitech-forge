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
/// PATH discovery does not execute a shell or search the agent's working directory.
pub fn find_executable(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|p| p.is_absolute())
        .flat_map(|p| {
            #[cfg(windows)]
            let candidates = if std::path::Path::new(name).extension().is_none() {
                vec![p.join(format!("{name}.exe")), p.join(format!("{name}.com"))]
            } else {
                vec![p.join(name)]
            };
            #[cfg(not(windows))]
            let candidates = vec![p.join(name)];
            candidates
        })
        .find(|p| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                p.metadata()
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            }
            #[cfg(not(unix))]
            {
                p.is_file()
            }
        })
}

pub fn resolve_command(name: &str, cwd: &std::path::Path) -> Result<std::path::PathBuf> {
    #[cfg(windows)]
    {
        let path = std::path::Path::new(name);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else if path.components().count() > 1 {
            cwd.join(path)
        } else {
            find_executable(name).ok_or_else(|| anyhow::anyhow!("native executable not found: {name}; install an EXE or configure an explicit executable path (CMD/BAT/PS1 wrappers are not implicit shells)"))?
        };
        ensure!(
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("com")),
            "use a native EXE, or explicit pwsh -File for a trusted PowerShell script; batch wrappers are unsupported"
        );
        Ok(path)
    }
    #[cfg(not(windows))]
    {
        let _ = cwd;
        Ok(name.into())
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
