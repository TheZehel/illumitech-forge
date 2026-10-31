//! Verifiable local gate reuse. Cache is a performance hint, never authority for human review.
use crate::{
    budget, duration,
    engine::{self, Context},
};
use anyhow::{Context as _, Result, ensure};
use forge_adapters::{
    observability::{Phase, Trace},
    process,
};
use forge_domain::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Instant,
};
use tokio::sync::watch;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: String,
    created_ms: u64,
    evidence: Evidence,
    checksum: String,
}
fn checksum(key: &str, created: u64, evidence: &Evidence) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(key, created, evidence))?))
}
fn tree(
    root: &Path,
    path: &Path,
    files: &mut BTreeMap<PathBuf, String>,
    size: &mut u64,
) -> Result<()> {
    ensure!(
        files.len() < 10_000 && *size <= 64 * 1024 * 1024,
        "worktree exceeds cache observation bound"
    );
    let mut entries = std::fs::read_dir(path)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for p in entries {
        ensure!(files.len() < 10_000, "worktree exceeds cache file bound");
        if p.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        let meta = p.symlink_metadata()?;
        ensure!(
            !meta.file_type().is_symlink(),
            "symlink inputs unobservable for cache"
        );
        if meta.is_dir() {
            tree(root, &p, files, size)?;
        } else {
            ensure!(meta.is_file(), "non-file inputs unobservable");
            *size = size.saturating_add(meta.len());
            ensure!(
                *size <= 64 * 1024 * 1024,
                "worktree exceeds cache observation bound"
            );
            let mode = {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    meta.permissions().mode()
                }
                #[cfg(not(unix))]
                {
                    u32::from(meta.permissions().readonly())
                }
            };
            files.insert(
                p.strip_prefix(root)?.into(),
                digest(&serde_json::to_vec(&(mode, digest(&std::fs::read(&p)?)))?),
            );
        }
    }
    Ok(())
}
fn executable(command: &str) -> Result<PathBuf> {
    let p = Path::new(command);
    if p.is_absolute() {
        return Ok(p.canonicalize()?);
    }
    ensure!(
        p.components().count() == 1,
        "relative executable identity unknown"
    );
    std::env::split_paths(&std::env::var_os("PATH").context("PATH unavailable")?)
        .map(|p| p.join(command))
        .find(|p| p.is_file())
        .context("executable unavailable")?
        .canonicalize()
        .map_err(Into::into)
}
fn file_identity(path: &Path) -> Result<(PathBuf, u32, String)> {
    let canonical = path.canonicalize()?;
    let meta = canonical.metadata()?;
    ensure!(meta.is_file(), "cache dependency is not a file");
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    };
    #[cfg(not(unix))]
    let mode = u32::from(meta.permissions().readonly());
    Ok((canonical, mode, digest(&std::fs::read(path)?)))
}
fn platform_identity() -> Result<String> {
    #[cfg(unix)]
    {
        let mut info = std::mem::MaybeUninit::<libc::utsname>::uninit();
        ensure!(
            unsafe { libc::uname(info.as_mut_ptr()) } == 0,
            "platform environment unobservable"
        );
        let info = unsafe { info.assume_init() };
        let parts = [
            info.sysname.as_ptr(),
            info.release.as_ptr(),
            info.version.as_ptr(),
            info.machine.as_ptr(),
        ];
        let bytes = parts
            .iter()
            .map(|p| unsafe { std::ffi::CStr::from_ptr(*p) }.to_bytes())
            .collect::<Vec<_>>();
        Ok(digest(&serde_json::to_vec(&bytes)?))
    }
    #[cfg(not(unix))]
    {
        anyhow::bail!("platform environment unobservable for cache")
    }
}
fn key(c: &Context, task: &Task, gate: &Gate) -> Result<String> {
    let program = Path::new(&gate.argv[0])
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    ensure!(
        ![
            "codex", "claude", "gh", "curl", "wget", "git", "sh", "bash", "zsh", "ssh", "npm",
            "npx"
        ]
        .contains(&program.as_ref()),
        "native, remote or shell execution is not cacheable"
    );
    c.manifest.verify_evaluators(&c.base, &c.roots)?;
    for (name, value) in &gate.cache.environment {
        ensure!(
            std::env::var(name).ok().as_ref() == Some(value),
            "declared environment mismatch"
        );
    }
    for (p, hash) in &gate.cache.toolchain {
        ensure!(
            p.is_absolute() && digest(&std::fs::read(p)?) == hash.to_lowercase(),
            "toolchain mismatch"
        );
    }
    let mut environment = std::env::vars_os()
        .map(|(k, v)| (k.as_encoded_bytes().to_vec(), v.as_encoded_bytes().to_vec()))
        .collect::<Vec<_>>();
    environment.sort();
    let mut trees = BTreeMap::new();
    let mut size = 0;
    for (id, root) in &c.roots {
        let mut files = BTreeMap::new();
        tree(root, root, &mut files, &mut size)?;
        trees.insert(id, files);
    }
    let runtime = executable(&gate.argv[0])?;
    let runtime_identity = file_identity(&runtime)?;
    let evaluator_identities = gate
        .inputs
        .keys()
        .map(|p| file_identity(&c.base.join(p)))
        .collect::<Result<Vec<_>>>()?;
    let toolchain_identities = gate
        .cache
        .toolchain
        .keys()
        .map(|p| file_identity(p))
        .collect::<Result<Vec<_>>>()?;
    let forge_identity = digest(&std::fs::read(std::env::current_exe()?)?);
    // Raw environment values never leave this hash operation.
    Ok(digest(&serde_json::to_vec(&(
        1,
        c.hash.as_str(),
        &c.base,
        revisions(&c.roots)?,
        task,
        gate,
        trees,
        runtime,
        runtime_identity,
        evaluator_identities,
        toolchain_identities,
        forge_identity,
        platform_identity()?,
        digest(&serde_json::to_vec(&environment)?),
        std::env::consts::OS,
        std::env::consts::ARCH,
    ))?))
}
/// Only journal-owned relative paths are exposed. No arbitrary file read interface.
pub fn diagnostic(c: &Context, detail: &serde_json::Value) -> Result<serde_json::Value> {
    let log = &detail["log"];
    let path = Path::new(log["path"].as_str().context("missing diagnostic path")?);
    ensure!(local_path(path), "invalid diagnostic path");
    let state = c.db.parent().context("state parent")?.canonicalize()?;
    let full = state.join(path).canonicalize()?;
    ensure!(full.starts_with(&state), "diagnostic outside state");
    ensure!(
        full.metadata()?.len() <= 3 * 1024 * 1024,
        "diagnostic too large"
    );
    let bytes = std::fs::read(full)?;
    ensure!(
        Some(digest(&bytes).as_str()) == log["sha256"].as_str(),
        "diagnostic digest mismatch"
    );
    let result: process::ProcessResult = serde_json::from_slice(&bytes)?;
    Ok(json!({"log":log,"reason":result.reason,"code":result.code,
        "stdout":process::redact(&result.stdout).chars().take(8192).collect::<String>(),
        "stderr":process::redact(&result.stderr).chars().take(8192).collect::<String>(),"truncated":result.truncated}))
}
fn verified_log(c: &Context, detail: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(detail)
        .ok()
        .is_some_and(|v| diagnostic(c, &v).is_ok())
}
fn write_diagnostic(
    c: &Context,
    run: &Run,
    task: &Task,
    gate: &Gate,
    result: &process::ProcessResult,
) -> Result<serde_json::Value> {
    let phase = if run.duration.as_ref().is_some_and(|d| d.final_evaluation) {
        "final"
    } else {
        "initial"
    };
    let generation = run.duration.as_ref().map_or(0, |d| d.generation);
    let sequence = run.duration.as_ref().map_or(0, |d| d.next_sequence);
    let relative = PathBuf::from("logs")
        .join(&run.release_id)
        .join(format!("generation-{generation}"))
        .join(format!(
            "{phase}-{}-{}-attempt-{}-{sequence}.json",
            task.id, gate.id, run.tasks[&task.id].attempts
        ));
    let path = c.db.parent().context("state parent")?.join(&relative);
    std::fs::create_dir_all(path.parent().unwrap())?;
    let bytes = serde_json::to_vec(result)?;
    // A repeated evaluation must never silently overwrite a referenced capture.
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(&bytes)?;
    Ok(json!({"path":relative,"sha256":digest(&bytes)}))
}
fn lookup(
    c: &Context,
    store: &Store,
    gate: &Gate,
    slot: &str,
    key: &str,
    now: u64,
) -> Result<(Option<Evidence>, String)> {
    let Some(value) = store.metadata(slot)? else {
        return Ok((None, "not_found_or_inputs_changed".into()));
    };
    let Ok(entry) = serde_json::from_value::<Entry>(value) else {
        return Ok((None, "corrupt_cache".into()));
    };
    if entry.key != key
        || entry.evidence.kind != "gate"
        || entry.evidence.id != gate.id
        || entry.evidence.outcome != Outcome::Passed
        || entry
            .evidence
            .digest
            .as_ref()
            .is_none_or(|s| !valid_hash(s))
        || entry.checksum != checksum(key, entry.created_ms, &entry.evidence)?
    {
        return Ok((None, "corrupt_cache".into()));
    }
    if now < entry.created_ms
        || now - entry.created_ms >= gate.cache.ttl_seconds.saturating_mul(1000)
    {
        return Ok((None, "expired_or_clock_changed".into()));
    }
    if !verified_log(c, &entry.evidence.detail) {
        return Ok((None, "missing_or_corrupt_diagnostic".into()));
    }
    Ok((Some(entry.evidence), "identical_verified_inputs".into()))
}
pub async fn evaluate(
    c: &Context,
    store: &mut Store,
    run: &mut Run,
    task: &Task,
    gate: &Gate,
    cancel: watch::Receiver<bool>,
    measured: Instant,
) -> Result<Evidence> {
    let slot = format!(
        "gate-cache:{}",
        digest(&serde_json::to_vec(&(&run.release_id, &task.id, &gate.id))?)
    );
    let cache_key = if gate.cache.enabled {
        key(c, task, gate)
    } else {
        Err(anyhow::anyhow!("disabled"))
    };
    let (cached, reason) = match &cache_key {
        Ok(k) => lookup(c, store, gate, &slot, k, duration::now_ms())
            .unwrap_or((None, "corrupt_cache".into())),
        Err(e) => (None, e.to_string()),
    };
    store.save(run,"gate_cache",&json!({"task":task.id,"gate":gate.id,"result":if cached.is_some(){"hit"}else{"miss"},"reason":reason,"key":cache_key.as_ref().ok(),"ttl_seconds":gate.cache.ttl_seconds}))?;
    if let Some(mut evidence) = cached {
        duration::finish(run, measured, "passed");
        let original: serde_json::Value = serde_json::from_str(&evidence.detail)?;
        evidence.detail = json!({"cache":"hit","reason":reason,"key":cache_key.ok(),"log":original["log"],"provenance":original}).to_string();
        return Ok(evidence);
    }
    let args = gate
        .argv
        .iter()
        .map(|a| {
            a.replace("{repo}", &c.roots[&task.repo].display().to_string())
                .replace("{manifest_dir}", &c.base.display().to_string())
        })
        .collect::<Vec<_>>();
    let active_ms = run.duration.as_ref().map_or(0, |d| d.observed_active_ms);
    let timeout = match budget::timeout(c, run, task, gate.timeout_seconds, active_ms) {
        Ok(n) => n,
        Err(e) => {
            duration::finish(run, measured, "blocked");
            return Ok(Evidence {
                kind: "gate".into(),
                id: gate.id.clone(),
                outcome: Outcome::Blocked,
                digest: None,
                detail: e.to_string(),
            });
        }
    };
    let (tx, rx) = watch::channel(false);
    let watcher = engine::AbortOnDrop(tokio::spawn(engine::watch_stop(
        c.db.clone(),
        run.release_id.clone(),
        cancel,
        tx,
        c.manifest.execution.max_parallel > 1,
    )));
    let mut span = Trace::current().span(
        Phase::ValidationGates,
        Some(&task.id),
        run.tasks[&task.id].attempts,
    );
    let result = process::run_observed(&args, &c.base, timeout, rx, |pid| {
        run.tasks.get_mut(&task.id).unwrap().pid = Some(pid);
        store.save(
            run,
            "gate_spawned",
            &json!({"task":task.id,"gate":gate.id,"pid":pid}),
        )
    })
    .await;
    drop(watcher);
    run.tasks.get_mut(&task.id).unwrap().pid = None;
    let outcome = match &result {
        Ok(r) if r.passed() => "passed",
        Ok(r) if r.reason == "exited" => "failed",
        Ok(r) => r.reason.as_str(),
        Err(_) => "error",
    };
    duration::finish(run, measured, outcome);
    duration::refresh(c, store, run);
    store.save(
        run,
        "gate_exited",
        &json!({"task":task.id,"gate":gate.id,"outcome":outcome}),
    )?;
    span.finish(outcome);
    c.manifest.verify_evaluators(&c.base, &c.roots)?;
    let evidence = match result {
        Ok(r) => {
            let log = write_diagnostic(c, run, task, gate, &r)?;
            if !r.passed() {
                eprintln!(
                    "gate {} / {}: {} (exit {:?}) log {}\n{}\n{}",
                    task.id,
                    gate.id,
                    r.reason,
                    r.code,
                    log["path"],
                    r.stdout.chars().take(4096).collect::<String>(),
                    r.stderr.chars().take(4096).collect::<String>()
                );
            }
            Evidence {
            kind: "gate".into(),
            id: gate.id.clone(),
            outcome: if r.passed() {
                Outcome::Passed
            } else {
                Outcome::Failed
            },
            digest: Some(digest(&serde_json::to_vec(&r)?)),
            detail: json!({"cache":"miss","reason":reason,"exit":r.code,"reason_process":r.reason,"log":log})
                .to_string(),
        }
        }
        Err(e) => {
            let result = process::ProcessResult {
                code: None,
                reason: "spawn_error".into(),
                stdout: String::new(),
                stderr: process::redact(&e.to_string()),
                elapsed_ms: measured.elapsed().as_millis(),
                truncated: false,
            };
            let log = write_diagnostic(c, run, task, gate, &result)?;
            Evidence {kind:"gate".into(),id:gate.id.clone(),outcome:Outcome::Blocked,digest:Some(digest(&serde_json::to_vec(&result)?)),detail:json!({"cache":"miss","reason_process":"spawn_error","error":result.stderr,"log":log}).to_string()}
        }
    };
    if let Ok(k) = cache_key
        && evidence.outcome == Outcome::Passed
        && key(c, task, gate).ok().as_ref() == Some(&k)
    {
        let created = duration::now_ms();
        let entry = Entry {
            checksum: checksum(&k, created, &evidence)?,
            key: k.clone(),
            created_ms: created,
            evidence: evidence.clone(),
        };
        store.set_metadata(&slot, &serde_json::to_value(entry)?)?;
    }
    Ok(evidence)
}
