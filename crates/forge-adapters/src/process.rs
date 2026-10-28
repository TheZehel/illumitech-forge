use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{io::AsyncReadExt, process::Command, sync::watch};

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
pub fn redact(text: &str) -> String {
    let mut clean = text.to_string();
    for (key, val) in std::env::vars() {
        if val.len() >= 8
            && ["TOKEN", "SECRET", "PASSWORD", "API_KEY"]
                .iter()
                .any(|p| key.to_uppercase().contains(p))
        {
            clean = clean.replace(&val, "[REDACTED]");
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
            .any(|s| lower.contains(s))
            {
                "[REDACTED sensitive line]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
struct Group(u32);
impl Group {
    fn signal(&self, sig: i32) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.0 as i32), sig);
        }
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        self.signal(libc::SIGKILL);
    }
}
pub async fn run(
    argv: &[String],
    cwd: &Path,
    seconds: u64,
    cancel: watch::Receiver<bool>,
) -> Result<ProcessResult> {
    run_observed(argv, cwd, seconds, cancel, |_| Ok(())).await
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
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let group = Group(
        child
            .id()
            .ok_or_else(|| anyhow::anyhow!("missing child PID"))?,
    );
    on_spawn(group.0)?;
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
        group.signal(libc::SIGTERM);
        match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
            Ok(s) => Some(s?),
            Err(_) => {
                group.signal(libc::SIGKILL);
                Some(child.wait().await?)
            }
        }
    } else {
        status
    };
    // Descendants may retain pipes after the direct child exits.
    group.signal(libc::SIGKILL);
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn redacts_auth() {
        assert_eq!(
            redact("Authorization: Bearer abc"),
            "[REDACTED sensitive line]"
        );
    }
}
