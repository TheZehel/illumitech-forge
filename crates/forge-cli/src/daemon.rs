use crate::{
    engine::{self, Context},
    publication,
};
use anyhow::{Result, ensure};
use forge_adapters::process;
use forge_domain::{Store, git};
use fs2::FileExt;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

fn epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn retry(state: &Path, release: &str, reason: &str) -> Result<Value> {
    ensure!(
        forge_domain::valid_id(release) && !reason.trim().is_empty(),
        "release and reconciliation reason required"
    );
    let store = Store::open(&state.join("state.sqlite"), false)?;
    if let Some(run) = store.load(release)? {
        ensure!(
            run.tasks.values().all(|t| t.pid.is_none()),
            "inspect active/orphan processes before retry"
        );
    }
    store.remove_metadata(&format!("daemon-block:{release}"))?;
    store.set_metadata(
        "daemon_last_retry",
        &json!({"release":release,"reason":reason,"at_epoch":epoch()}),
    )?;
    Ok(
        json!({"schema_version":1,"status":"passed","release":release,"execution_state":"unchanged","note":"blocked task state still requires release resume; this clears only the queue failure latch"}),
    )
}

pub fn control(state: &Path, action: &str, reason: Option<&str>) -> Result<Value> {
    let db = state.join("state.sqlite");
    if action == "status" {
        let (control, heartbeat, cycle) = if db.exists() {
            let s = Store::open(&db, true)?;
            (
                s.metadata("daemon_control")?,
                s.metadata("daemon_heartbeat")?,
                s.metadata("daemon_cycle")?,
            )
        } else {
            (None, None, None)
        };
        return Ok(
            json!({"schema_version":1,"control":control,"heartbeat":heartbeat,"last_cycle":cycle,"state":"not_modified"}),
        );
    }
    ensure!(
        ["pause", "resume"].contains(&action),
        "unknown daemon control"
    );
    let value = json!({"paused":action == "pause","reason":reason.unwrap_or("operator request"),"updated_at_epoch":epoch()});
    Store::open(&db, false)?.set_metadata("daemon_control", &value)?;
    Ok(json!({"schema_version":1,"status":"passed","control":value}))
}

async fn cycle(dir: &Path, state: &Path, dry: bool, stop: &watch::Receiver<bool>) -> Result<Value> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.retain(|p| {
        p.extension()
            .and_then(|v| v.to_str())
            .is_some_and(|s| ["yaml", "yml", "json"].contains(&s))
    });
    paths.sort();
    let db = state.join("state.sqlite");
    let mut results = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    // Inspect all contracts before mutating anything, including duplicate release IDs.
    let mut contexts = Vec::new();
    for path in paths {
        match Context::load(&path,state) {
            Ok(c) => {
                ensure!(ids.insert(c.manifest.release_id.clone()), "duplicate release ID in daemon queue");
                contexts.push((path,c));
            },
            Err(e) => results.push(json!({"manifest":path,"status":"blocked","error":forge_adapters::process::redact(&e.to_string())})),
        }
    }
    for (path, c) in contexts {
        if *stop.borrow() {
            break;
        }
        if !c.manifest.automation.enabled {
            results.push(json!({"manifest":path,"status":"disabled"}));
            continue;
        }
        if dry {
            results.push(c.inspect()?);
            continue;
        }
        let store = Store::open(&db, false)?;
        if store
            .metadata("daemon_control")?
            .is_some_and(|v| v["paused"] == true)
        {
            results.push(json!({"manifest":path,"status":"paused"}));
            break;
        }
        let id = c.manifest.release_id.clone();
        let blocked_key = format!("daemon-block:{id}");
        if let Some(block) = store.metadata(&blocked_key)? {
            results.push(json!({"release_id":id,"status":"blocked","reason":"explicit resume/reconciliation required","detail":block}));
            continue;
        }
        let attempt = crate::observability::phase(&c, false, forge_adapters::observability::Phase::DaemonCycle,
            crate::observability::nested(forge_adapters::observability::Phase::Release, Box::pin(async {
            let existing=store.load(&id)?;
            if let Some(old)=existing {
                ensure!(old.status == "passed", "existing release is {}; use explicit resume/reconciliation",old.status);
                // Never adopt material changes just because a watcher noticed them.
                engine::assert_ready(&c,false)?;
            } else {
                if c.manifest.automation.require_clean_worktree {
                    for root in c.roots.values() { ensure!(git(root,&["status","--porcelain"])?.trim().is_empty(), "automation requires a clean initial worktree"); }
                }
                if let Some(config)=&c.manifest.automation.publish { publication::draft_preflight(&c,config)?; }
                store.set_metadata("daemon_active_release",&json!(id))?;
                let execution=engine::execute_in_scope(Context::load(&path,state)?,false,None).await;
                store.set_metadata("daemon_active_release",&Value::Null)?;
                let output=execution?;
                ensure!(output["run"]["status"] == "passed", "release stopped with {}; explicit resume required",output["run"]["status"]);
            }
            if c.manifest.automation.publish.is_some() {
                // No duplicate network publication after a successful receipt on this revision.
                let current=engine::assert_ready(&c,false)?;
                let prior=Store::open(&db,true)?.events(&id)?.into_iter().rev().find(|e|e["kind"] == "draft_pr_published");
                if let Some(receipt)=prior
                    && current.revisions.values().any(|r|Some(r.commit.as_str()) == receipt["body"]["commit"].as_str()) {
                    return Ok(json!({"release_id":id,"status":"passed","publication":"already_recorded","url":receipt["body"]["url"]}));
                }
                publication::draft_pr_with_cancel(&c,false,stop).await
            } else { Ok(json!({"release_id":id,"status":"passed","publication":"disabled"})) }
        }))).await;
        match attempt {
            Ok(v) => results.push(v),
            Err(e) => {
                let detail = json!({"manifest_hash":c.hash,"reason":forge_adapters::process::redact(&e.to_string()),"at_epoch":epoch()});
                store.set_metadata(&blocked_key, &detail)?;
                results.push(json!({"release_id":id,"status":"blocked","detail":detail}));
            }
        }
    }
    let failed = results.iter().any(|r| r["status"] == "blocked");
    let output = json!({"schema_version":1,"status":if failed {"blocked"} else {"passed"},"dry_run":dry,"releases":results});
    if !dry {
        Store::open(&db, false)?.set_metadata("daemon_cycle", &output)?;
    }
    Ok(output)
}

pub async fn run(dir: &Path, state: &Path, once: bool, interval: u64, dry: bool) -> Result<Value> {
    ensure!(
        (1..=3600).contains(&interval),
        "interval must be 1..3600 seconds"
    );
    let (_tx, rx) = watch::channel(false);
    if dry {
        return cycle(dir, state, true, &rx).await;
    }
    // Context validation must precede creating locks/state inside a task root.
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|s| ["yaml", "yml", "json"].contains(&s))
        {
            Context::load(&path, state)?;
        }
    }
    std::fs::create_dir_all(state)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(state.join("daemon.lock"))?;
    lock.try_lock_exclusive()
        .map_err(|_| anyhow::anyhow!("another daemon owns this state directory"))?;
    let db = state.join("state.sqlite");
    Store::open(&db, false)?;
    let heartbeat_db = db.clone();
    let heartbeat = tokio::spawn(async move {
        loop {
            if let Ok(s) = Store::open(&heartbeat_db, false) {
                let _ = s.set_metadata(
                    "daemon_heartbeat",
                    &json!({"pid":std::process::id(),"running":true,"at_epoch":epoch()}),
                );
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let signal_db = db.clone();
    let signal = tokio::spawn(async move {
        #[cfg(unix)]
        {
            if let Ok(mut term) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                tokio::select! { _=process::interrupt()=>{}, _=term.recv()=>{} }
            } else {
                let _ = process::interrupt().await;
            }
        }
        #[cfg(not(unix))]
        let _ = process::interrupt().await;
        let _ = stop_tx.send(true);
        if let Ok(mut store) = Store::open(&signal_db, false)
            && let Ok(Some(Value::String(id))) = store.metadata("daemon_active_release")
        {
            let _ = store.set_control(&id, "cancel");
        }
    });
    let result=async {
        loop {
            let output=cycle(dir,state,false,&stop_rx).await?;
            if once || *stop_rx.borrow() { return Ok(output); }
            println!("{}",output);
            tokio::select! { _=tokio::time::sleep(Duration::from_secs(interval))=>{}, _=stop_rx.changed()=> {return Ok(json!({"schema_version":1,"status":"passed","stopped":true}));} }
        }
    }.await;
    heartbeat.abort();
    signal.abort();
    Store::open(&db, false)?.set_metadata(
        "daemon_heartbeat",
        &json!({"pid":std::process::id(),"running":false,"at_epoch":epoch()}),
    )?;
    drop(lock);
    result
}
