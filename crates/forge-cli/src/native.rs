//! Versioned App Server JSONL transport. No model/auth/config overrides or replay.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{Mutex, broadcast, oneshot},
};

pub const FRAME_LIMIT: usize = 1024 * 1024;
type Pending = Arc<Mutex<BTreeMap<u64, oneshot::Sender<Value>>>>;
pub struct Native {
    writer: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Pending,
    sequence: AtomicU64,
    pub events: broadcast::Sender<Value>,
    reader: tokio::task::JoinHandle<()>,
    pub handshake: Value,
    owned_group: Arc<AtomicU32>,
    active_turns: Arc<Mutex<BTreeMap<String, String>>>,
}
impl Native {
    pub async fn connect(executable: Option<&Path>, root: &Path) -> Result<Self> {
        let mut cmd = Command::new(executable.unwrap_or(Path::new("codex")));
        cmd.args(["app-server", "--listen", "stdio://"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd.spawn().context("native App Server unavailable")?;
        let writer = child.stdin.take().context("native stdin unavailable")?;
        let stdout = child.stdout.take().context("native stdout unavailable")?;
        let pending: Pending = Arc::default();
        let (events, _) = broadcast::channel(256);
        let owned_group = Arc::new(AtomicU32::new(child.id().unwrap_or(0)));
        let disconnected_group = owned_group.clone();
        let active_turns = Arc::new(Mutex::new(BTreeMap::new()));
        let observed_turns = active_turns.clone();
        let p = pending.clone();
        let e = events.clone();
        let reader = tokio::spawn(async move {
            let mut input = BufReader::new(stdout);
            loop {
                let mut frame = Vec::new();
                let result = async {
                    loop {
                        let bytes = input.fill_buf().await?;
                        if bytes.is_empty() {
                            return Ok::<bool, std::io::Error>(false);
                        }
                        let n = bytes
                            .iter()
                            .position(|b| *b == b'\n')
                            .map_or(bytes.len(), |n| n + 1);
                        if frame.len() + n > FRAME_LIMIT {
                            return Ok(false);
                        }
                        let done = bytes[n - 1] == b'\n';
                        frame.extend_from_slice(&bytes[..n]);
                        input.consume(n);
                        if done {
                            return Ok(true);
                        }
                    }
                }
                .await;
                if !matches!(result, Ok(true)) {
                    break;
                }
                let Ok(value) = serde_json::from_slice::<Value>(&frame) else {
                    break;
                };
                if value.get("method").is_none() {
                    if let Some(id) = value["id"].as_u64()
                        && let Some(tx) = p.lock().await.remove(&id)
                    {
                        let _ = tx.send(value);
                    }
                } else {
                    if let Some(thread) = value["params"]["threadId"].as_str() {
                        if value["method"] == "turn/started" {
                            if let Some(turn) = value["params"]["turn"]["id"].as_str() {
                                observed_turns
                                    .lock()
                                    .await
                                    .insert(thread.to_owned(), turn.to_owned());
                            }
                        } else if value["method"] == "turn/completed" {
                            observed_turns.lock().await.remove(thread);
                        }
                    }
                    let _ = e.send(value);
                }
            }
            #[cfg(unix)]
            {
                let id = disconnected_group.swap(0, Ordering::SeqCst);
                if id > 0 {
                    unsafe {
                        libc::kill(-(id as i32), libc::SIGKILL);
                    }
                }
            }
            p.lock().await.clear();
            let _ = e.send(json!({"method":"bridge/disconnected","params":{"reason":"Native connection closed; pending outcomes unknown. No action was replayed."}}));
        });
        let mut native = Self {
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            pending,
            sequence: AtomicU64::new(1),
            events,
            reader,
            handshake: Value::Null,
            owned_group,
            active_turns,
        };
        native.handshake = native.call("initialize", json!({"clientInfo":{"name":"illumitech_forge","title":"Illumitech Forge","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}})).await?;
        native
            .write(json!({"method":"initialized","params":{}}))
            .await?;
        Ok(native)
    }
    pub async fn write(&self, value: Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(&value)?;
        ensure!(bytes.len() < FRAME_LIMIT, "native request too large");
        bytes.push(b'\n');
        let mut writer = self.writer.lock().await;
        tokio::time::timeout(Duration::from_secs(10), async {
            writer.write_all(&bytes).await?;
            writer.flush().await
        })
        .await??;
        Ok(())
    }
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut p = self.pending.lock().await;
            ensure!(p.len() < 64, "native request queue full");
            p.insert(id, tx);
        }
        let result = async {
            self.write(json!({"id":id,"method":method,"params":params}))
                .await?;
            let response = tokio::time::timeout(Duration::from_secs(20), rx)
                .await
                .context("native response timed out; outcome unknown, do not retry automatically")?
                .context("native disconnected; outcome unknown")?;
            if response.get("error").is_some() {
                bail!(
                    "native request rejected: {}",
                    forge_adapters::process::redact(&response["error"].to_string())
                );
            }
            response
                .get("result")
                .cloned()
                .context("invalid native response")
        }
        .await;
        self.pending.lock().await.remove(&id);
        result
    }
    pub async fn shutdown(&self) {
        // Let the native server cancel tools it owns, including tools with their own groups.
        // The deadline bounds an unresponsive server; cancellation never grants approval.
        let active = self.active_turns.lock().await.clone();
        let _ = tokio::time::timeout(Duration::from_millis(500), async {
            for (thread, turn) in active {
                let _ = self
                    .call("turn/interrupt", json!({"threadId":thread,"turnId":turn}))
                    .await;
            }
        })
        .await;
        let mut child = self.child.lock().await;
        let group = self.owned_group.swap(0, Ordering::SeqCst);
        #[cfg(unix)]
        if group > 0 {
            unsafe {
                libc::kill(-(group as i32), libc::SIGTERM);
            }
        }
        let _ = tokio::time::timeout(Duration::from_secs(1), child.wait()).await;
        // Parent exit does not prove descendant exit; reap the entire owned group.
        #[cfg(unix)]
        if group > 0 {
            unsafe {
                libc::kill(-(group as i32), libc::SIGKILL);
            }
        }
        let _ = child.kill().await;
        self.reader.abort();
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.reader.abort();
        #[cfg(unix)]
        {
            let id = self.owned_group.swap(0, Ordering::SeqCst);
            if id > 0 {
                unsafe {
                    libc::kill(-(id as i32), libc::SIGKILL);
                }
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    async fn owned_descendant(mode: &str) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let exe = root.join("server.py");
        let pidfile = root.join("descendant");
        let heartbeat = root.join("heartbeat");
        let interrupted = root.join("interrupted");
        std::fs::write(&exe,format!(r#"#!/usr/bin/env python3
import json,sys,subprocess,time
from pathlib import Path
for line in sys.stdin:
 v=json.loads(line)
 if v.get('method')=='initialize':
  print(json.dumps({{'id':v['id'],'result':{{'protocol':'fixture'}}}}),flush=True)
 elif v.get('method')=='initialized':
  code="import signal,time\nfrom pathlib import Path\nsignal.signal(signal.SIGTERM,signal.SIG_IGN)\np=Path("+repr({heartbeat:?})+")\nfor i in range(6000):\n p.write_text(str(i))\n time.sleep(.01)"
  child=subprocess.Popen([sys.executable,'-c',code],stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  Path({pid:?}).write_text(str(child.pid))
  for _ in range(200):
   if Path({heartbeat:?}).exists(): break
   time.sleep(.005)
  print(json.dumps({{'method':'turn/started','params':{{'threadId':'fixture-thread','turn':{{'id':'fixture-turn'}}}}}}),flush=True)
  if {mode:?}=='disconnect': sys.exit(0)
  print(json.dumps({{'id':'approval','method':'item/commandExecution/requestApproval','params':{{'reason':'pending fixture approval'}}}}),flush=True)
 elif v.get('method')=='turn/interrupt':
  Path({interrupted:?}).write_text('interrupted')
  print(json.dumps({{'id':v['id'],'result':{{}}}}),flush=True)
"#,pid=pidfile.to_str().unwrap(),heartbeat=heartbeat.to_str().unwrap())).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let native = Native::connect(Some(&exe), &root).await.unwrap();
        for _ in 0..200 {
            if heartbeat.exists() && !native.active_turns.lock().await.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(heartbeat.exists(), "descendant did not start");
        if mode == "shutdown" {
            native.shutdown().await;
            assert!(
                interrupted.exists(),
                "native turn interrupt was not requested before termination"
            );
        }
        drop(native);
        // The sandbox denies process inventory. Observe the owned descendant's heartbeat
        // instead: a surviving TERM-resistant child continues writing every 10 ms.
        tokio::time::sleep(Duration::from_millis(150)).await;
        let stopped = std::fs::read(&heartbeat).unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(
            std::fs::read(&heartbeat).unwrap(),
            stopped,
            "owned descendant survived {mode}"
        );
    }
    #[tokio::test]
    async fn shutdown_cleans_descendants_with_approval_outstanding() {
        owned_descendant("shutdown").await;
    }
    #[tokio::test]
    async fn disconnect_cleans_descendants_without_replay() {
        owned_descendant("disconnect").await;
    }
    #[tokio::test]
    async fn drop_cleans_owned_process_group() {
        owned_descendant("drop").await;
    }
}
