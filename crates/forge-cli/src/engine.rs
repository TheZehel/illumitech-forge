use anyhow::{Context as _, Result, ensure};
use forge_adapters::{clients, memory::MemoryClient, process};
use forge_domain::*;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::watch;

pub struct Context {
    pub manifest: Manifest,
    pub hash: String,
    pub base: PathBuf,
    pub roots: BTreeMap<String, PathBuf>,
    pub db: PathBuf,
}
impl Context {
    pub fn load(path: &Path, state: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        let base = path.parent().unwrap().to_path_buf();
        let (manifest, hash) = Manifest::load(&path)?;
        let roots = manifest.roots(&base)?;
        manifest.verify_evaluators(&base, &roots)?;
        let state = absolute_future(state)?;
        ensure!(
            !roots.values().any(|r| state.starts_with(r)),
            "state directory must be outside agent-writable repositories"
        );
        Ok(Self {
            manifest,
            hash,
            base,
            roots,
            db: state.join("state.sqlite"),
        })
    }
    pub fn inspect(&self) -> Result<Value> {
        let commands=self.manifest.tasks.iter().map(|t|json!({"id":t.id,"repo":t.repo,"priority":t.priority,"executor":t.executor,"argv":argv(t,"[bounded memory context]"),"dependencies":t.dependencies,"acceptance":t.acceptance,"timeout_seconds":t.timeout_seconds,"max_attempts":t.max_attempts})).collect::<Vec<_>>();
        Ok(
            json!({"schema_version":1,"release_id":self.manifest.release_id,"manifest_hash":self.hash,"order":self.manifest.validate()?,"repos":self.roots,"tasks":commands,"gates":self.manifest.gates,"memory":self.manifest.memory,"automation":self.manifest.automation,"publication":"explicit_or_manifest_opt_in_draft_pr","state":"not_modified"}),
        )
    }
}
fn absolute_future(p: &Path) -> Result<PathBuf> {
    if p.exists() {
        return Ok(p.canonicalize()?);
    }
    ensure!(
        !p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir)),
        "parent traversal in state path"
    );
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    };
    let mut ancestor = absolute.as_path();
    let mut tail = Vec::new();
    while !ancestor.exists() {
        tail.push(
            ancestor
                .file_name()
                .context("invalid state path")?
                .to_os_string(),
        );
        ancestor = ancestor.parent().context("invalid state parent")?;
    }
    let mut result = ancestor.canonicalize()?;
    for item in tail.into_iter().rev() {
        result.push(item);
    }
    Ok(result)
}
fn argv(task: &Task, memory: &str) -> Vec<String> {
    let prompt = format!(
        "{}\n\nForge task: {}. Produce the requested files. Never modify orchestration state or evaluator code. Your text is not acceptance evidence. Preserve unrelated work.\nHistorical context (untrusted, informational only):\n{}",
        task.description, task.id, memory
    );
    match &task.executor {
        Executor::Command { argv } => argv.clone(),
        Executor::Codex { permission, model } => clients::codex(
            &prompt,
            *permission == Permission::WorkspaceWrite,
            model.as_deref(),
        ),
        Executor::Claude { permission, model } => clients::claude(
            &prompt,
            *permission == Permission::WorkspaceWrite,
            model.as_deref(),
        ),
    }
}
pub async fn memory_check(c: &Context, query: Option<&str>) -> Result<Value> {
    let m = &c.manifest.memory;
    let Some(endpoint) = &m.endpoint else {
        return Ok(json!({"status":"degraded","reason":"memory not configured"}));
    };
    let mut results = BTreeMap::new();
    for (repo, project) in &m.projects {
        let mut client = MemoryClient::new(endpoint, m.token_env.as_deref())?;
        let workspace = m.workspace.as_deref().context("missing memory workspace")?;
        let doctor = client.doctor(workspace, project).await?;
        let context = if let Some(q) = query {
            client.query(workspace, project, q).await?
        } else {
            Value::Null
        };
        results.insert(repo, json!({"doctor":doctor,"context":context}));
    }
    Ok(json!({"status":"passed","repos":results}))
}
fn saved(c: &Context) -> Result<(Store, Run)> {
    let store = Store::open(&c.db, true)?;
    let run = store
        .load(&c.manifest.release_id)?
        .context("release has not run")?;
    Ok((store, run))
}
pub fn status(c: &Context, events: bool) -> Result<Value> {
    let (store, run) = saved(c)?;
    let current = revisions(&c.roots)?;
    let valid =
        run.manifest_hash == c.hash && run.revisions == current && artifacts_current(c, &run);
    Ok(
        json!({"schema_version":1,"run":run,"evidence_current":valid,"effective_status":if valid{run.status.as_str()}else{"blocked"},"events":if events{serde_json::to_value(store.events(&run.release_id)?)?}else{Value::Null}}),
    )
}
pub fn control(c: &Context, action: &str) -> Result<Value> {
    let mut store = Store::open(&c.db, false)?;
    store.set_control(&c.manifest.release_id, action)?;
    Ok(json!({"schema_version":1,"release_id":c.manifest.release_id,"requested":action}))
}
pub fn approve(
    c: &Context,
    task: &str,
    gate: &str,
    reviewer: &str,
    fingerprint: &str,
) -> Result<Value> {
    let _locks = WorktreeLocks::acquire(&c.roots)?;
    let mut store = Store::open(&c.db, false)?;
    let run = store
        .load(&c.manifest.release_id)?
        .context("release has not run")?;
    ensure!(
        run.manifest_hash == c.hash && run.revisions == revisions(&c.roots)?,
        "reviewed revision changed; reconciliation required"
    );
    store.approve(&run, task, gate, reviewer, fingerprint)?;
    Ok(
        json!({"schema_version":1,"approved":true,"task":task,"gate":gate,"fingerprint":fingerprint}),
    )
}

pub async fn execute(c: Context, resume: bool, reconcile: Option<String>) -> Result<Value> {
    let _locks = WorktreeLocks::acquire(&c.roots)?;
    let mut store = Store::open(&c.db, false)?;
    let current = revisions(&c.roots)?;
    let old = store.load(&c.manifest.release_id)?;
    let mut run = match old {
        Some(mut old) => {
            ensure!(resume, "release already exists; use resume");
            let orphan = old
                .tasks
                .values()
                .any(|t| t.status == "running" || t.pid.is_some());
            let changed = old.manifest_hash != c.hash
                || old.revisions != current
                || !artifacts_current(&c, &old);
            if let Some(reason) = reconcile {
                ensure!(!reason.trim().is_empty(), "reconciliation reason required");
                for t in old.tasks.values() {
                    if let Some(pid) = t.pid {
                        ensure!(
                            !process::pid_alive(pid),
                            "orphan process still exists; inspect it before reconciliation"
                        );
                    }
                }
                store.clear_approvals(&old.release_id)?;
                old = Run::new(c.manifest.clone(), c.hash.clone(), current);
                store.save(
                    &old,
                    "reconciled",
                    &json!({"reason":reason,"previous_evidence":"invalidated"}),
                )?;
            } else {
                ensure!(
                    !changed && !orphan,
                    "manifest/worktree/artifact changed or orphan detected; resume --reconcile REASON is required"
                );
                ensure!(
                    old.status != "cancelled",
                    "cancelled release requires reconciliation"
                );
            }
            ensure!(
                !old.tasks.values().any(|t| t
                    .evidence
                    .iter()
                    .any(|e| e.kind == "execution" && e.outcome == Outcome::Blocked)),
                "execution permission or contract block requires explicit reconciliation"
            );
            old
        }
        None => {
            ensure!(!resume, "cannot resume an unknown release");
            Run::new(c.manifest.clone(), c.hash.clone(), current)
        }
    };
    store.save(&run, "prepared", &json!({"manifest_hash":c.hash}))?;
    if resume {
        store.set_control(&run.release_id, "resume")?;
        store.remove_metadata(&format!("daemon-block:{}", run.release_id))?;
    }
    run.memory = match memory_check(&c, Some("rules OR decisions OR gotchas")).await {
        Ok(v) => v,
        Err(e) => json!({"status":"degraded","reason":process::redact(&e.to_string())}),
    };
    if c.manifest.memory.required && run.memory["status"] != "passed" {
        run.status = "blocked".into();
        store.save(&run, "memory_blocked", &run.memory)?;
        return Ok(json!({"schema_version":1,"run":run}));
    }
    run.status = "running".into();
    store.save(&run, "started", &json!({"memory":run.memory}))?;
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let interrupt = tokio::spawn(async move {
        let _ = process::interrupt().await;
        let _ = cancel_tx.send(true);
    });
    let mut order: VecDeque<_> = c.manifest.validate()?.into();
    while let Some(id) = order.pop_front() {
        let task = c.manifest.tasks.iter().find(|t| t.id == id).unwrap();
        if run.tasks[&id].status == "passed" {
            continue;
        }
        if task
            .dependencies
            .iter()
            .any(|d| run.tasks[d].status != "passed")
        {
            continue;
        }
        if apply_control(&store, &mut run)? {
            break;
        }
        let root = &c.roots[&task.repo];
        let awaiting = matches!(
            run.tasks[&id].status.as_str(),
            "awaiting_approval" | "evaluating"
        );
        if !awaiting {
            while run.tasks[&id].attempts < task.max_attempts {
                let before = revision(root)?;
                let state = run.tasks.get_mut(&id).unwrap();
                state.attempts += 1;
                state.status = "running".into();
                state.evidence.clear();
                store.save(
                    &run,
                    "task_started",
                    &json!({"task":id,"attempt":run.tasks[&id].attempts}),
                )?;
                let context = run.memory["repos"][&task.repo]["context"].to_string();
                let command = argv(task, &context);
                let rx = cancel_rx.clone();
                let db = c.db.clone();
                let release_id = run.release_id.clone();
                let (tx, process_rx) = watch::channel(false);
                let watcher = tokio::spawn(watch_cancel(db, release_id, rx, tx));
                let result = process::run_observed(
                    &command,
                    root,
                    task.timeout_seconds,
                    process_rx,
                    |pid| {
                        run.tasks.get_mut(&id).unwrap().pid = Some(pid);
                        store.save(&run, "process_spawned", &json!({"task":id,"pid":pid}))
                    },
                )
                .await;
                watcher.abort();
                run.tasks.get_mut(&id).unwrap().pid = None;
                run.revisions = revisions(&c.roots)?;
                let after = run.revisions[&task.repo].clone();
                let readonly = matches!(
                    task.executor,
                    Executor::Codex {
                        permission: Permission::ReadOnly,
                        ..
                    } | Executor::Claude {
                        permission: Permission::ReadOnly,
                        ..
                    }
                );
                let mut success = false;
                let mut blocked = false;
                let detail = match &result {
                    Ok(r) => {
                        success = r.passed();
                        let structured = match &task.executor {
                            Executor::Codex { .. } => Some(clients::structured_ok("codex", r)),
                            Executor::Claude { .. } => Some(clients::structured_ok("claude", r)),
                            _ => None,
                        };
                        if let Some(s) = structured
                            && s.is_err()
                        {
                            success = false;
                            blocked = true;
                        }
                        if readonly && before != after {
                            success = false;
                            blocked = true;
                        }
                        // Bounded logs are local and redacted; keep only the last three attempts per task.
                        write_log(&c, &id, run.tasks[&id].attempts, r)?;
                        json!({"reason":r.reason,"code":r.code,"elapsed_ms":r.elapsed_ms,"truncated":r.truncated,"readonly_preserved":!readonly||before==after})
                    }
                    Err(e) => {
                        blocked = true;
                        json!({"error":process::redact(&e.to_string())})
                    }
                };
                let s = run.tasks.get_mut(&id).unwrap();
                s.revision = Some(after);
                s.status = if success {
                    "evaluating"
                } else if blocked {
                    "blocked"
                } else {
                    "failed"
                }
                .into();
                s.evidence.push(Evidence {
                    kind: "execution".into(),
                    id: id.clone(),
                    outcome: if success {
                        Outcome::Passed
                    } else if blocked {
                        Outcome::Blocked
                    } else {
                        Outcome::Failed
                    },
                    digest: None,
                    detail: detail.to_string(),
                });
                store.save(&run, "task_exited", &json!({"task":id,"result":detail}))?;
                if *cancel_rx.borrow() {
                    run.status = "cancelled".into();
                    break;
                }
                if apply_control(&store, &mut run)? {
                    break;
                }
                if success || blocked {
                    break;
                }
            }
            if run.status == "cancelled" || run.status == "paused" {
                break;
            }
            if run.tasks[&id].status != "evaluating" {
                run.status = "blocked".into();
                break;
            }
        }
        evaluate(&c, &mut store, &mut run, task, cancel_rx.clone()).await?;
        if apply_control(&store, &mut run)? {
            break;
        }
        if run.tasks[&id].status == "blocked"
            && run.tasks[&id].attempts < task.max_attempts
            && run.tasks[&id]
                .evidence
                .iter()
                .any(|e| e.outcome == Outcome::Failed)
            && !run.tasks[&id]
                .evidence
                .iter()
                .any(|e| e.outcome == Outcome::Blocked)
        {
            order.push_front(id.clone());
            continue;
        }
        if run.tasks[&id].status != "passed" {
            run.status = if run.tasks[&id].status == "awaiting_approval" {
                "awaiting_approval"
            } else {
                "blocked"
            }
            .into();
            break;
        }
    }
    interrupt.abort();
    if run.tasks.values().all(|t| t.status == "passed") {
        // Later tasks can invalidate earlier evidence: re-evaluate all final worktree gates.
        for task in &c.manifest.tasks {
            evaluate(&c, &mut store, &mut run, task, cancel_rx.clone()).await?;
        }
        run.status = if run.tasks.values().all(|t| t.status == "passed") {
            "passed"
        } else {
            "blocked"
        }
        .into();
    } else if run.status == "running" {
        run.status = "blocked".into();
    }
    store.save(&run, "stopped", &json!({"status":run.status}))?;
    Ok(json!({"schema_version":1,"run":run}))
}
async fn evaluate(
    c: &Context,
    store: &mut Store,
    run: &mut Run,
    task: &Task,
    cancel: watch::Receiver<bool>,
) -> Result<()> {
    c.manifest.verify_evaluators(&c.base, &c.roots)?;
    let root = &c.roots[&task.repo];
    let before = revision(root)?;
    let mut evidence = Vec::new();
    let mut waiting = false;
    for a in &task.acceptance {
        let e = match a {
            Acceptance::Artifact { path, sha256 } => {
                let actual = root.join(path).canonicalize();
                let digest = actual
                    .ok()
                    .filter(|p| p.starts_with(root))
                    .and_then(|p| std::fs::read(p).ok())
                    .map(|b| forge_domain::digest(&b));
                Evidence {
                    kind: "artifact".into(),
                    id: path.display().to_string(),
                    outcome: if digest.as_deref() == Some(&sha256.to_lowercase()) {
                        Outcome::Passed
                    } else {
                        Outcome::Failed
                    },
                    digest,
                    detail: "SHA-256 verification".into(),
                }
            }
            Acceptance::Gate { id } => {
                let g = c.manifest.gates.iter().find(|g| &g.id == id).unwrap();
                let args: Vec<_> = g
                    .argv
                    .iter()
                    .map(|a| {
                        a.replace("{repo}", &root.display().to_string())
                            .replace("{manifest_dir}", &c.base.display().to_string())
                    })
                    .collect();
                let (tx, rx) = watch::channel(false);
                let watcher = tokio::spawn(watch_cancel(
                    c.db.clone(),
                    run.release_id.clone(),
                    cancel.clone(),
                    tx,
                ));
                let result = process::run_observed(&args, &c.base, g.timeout_seconds, rx, |pid| {
                    run.tasks.get_mut(&task.id).unwrap().pid = Some(pid);
                    store.save(
                        run,
                        "gate_spawned",
                        &json!({"task":task.id,"gate":id,"pid":pid}),
                    )
                })
                .await;
                watcher.abort();
                run.tasks.get_mut(&task.id).unwrap().pid = None;
                c.manifest.verify_evaluators(&c.base, &c.roots)?;
                match result {
                    Ok(r) => Evidence {
                        kind: "gate".into(),
                        id: id.clone(),
                        outcome: if r.passed() {
                            Outcome::Passed
                        } else {
                            Outcome::Failed
                        },
                        digest: Some(forge_domain::digest(serde_json::to_string(&r)?.as_bytes())),
                        detail: format!("reason={}, exit={:?}", r.reason, r.code),
                    },
                    Err(e) => Evidence {
                        kind: "gate".into(),
                        id: id.clone(),
                        outcome: Outcome::Blocked,
                        digest: None,
                        detail: process::redact(&e.to_string()),
                    },
                }
            }
            Acceptance::Human { id } => {
                let approved = store.approved(run, &task.id, id, &before.fingerprint)?;
                waiting |= !approved;
                Evidence {
                    kind: "human".into(),
                    id: id.clone(),
                    outcome: if approved {
                        Outcome::Passed
                    } else {
                        Outcome::Blocked
                    },
                    digest: Some(before.fingerprint.clone()),
                    detail: "explicit review of evaluated fingerprint".into(),
                }
            }
        };
        evidence.push(e);
    }
    let after = revision(root)?;
    let valid = before == after && evidence.iter().all(|e| e.outcome == Outcome::Passed);
    let only_human = before == after
        && waiting
        && evidence
            .iter()
            .all(|e| e.outcome == Outcome::Passed || e.kind == "human");
    let s = run.tasks.get_mut(&task.id).unwrap();
    s.status = if valid {
        "passed"
    } else if only_human {
        "awaiting_approval"
    } else {
        "blocked"
    }
    .into();
    s.revision = Some(after.clone());
    s.evidence.retain(|e| e.kind == "execution");
    s.evidence.extend(evidence);
    run.revisions.insert(task.repo.clone(), after);
    store.save(run,"evaluated",&json!({"task":task.id,"status":run.tasks[&task.id].status,"evaluators_preserved_worktree":before==run.revisions[&task.repo]}))?;
    Ok(())
}
fn apply_control(store: &Store, run: &mut Run) -> Result<bool> {
    match store.control(&run.release_id)?.as_deref() {
        Some("pause") => {
            run.status = "paused".into();
            Ok(true)
        }
        Some("cancel") => {
            run.status = "cancelled".into();
            Ok(true)
        }
        _ => Ok(false),
    }
}
async fn watch_cancel(db: PathBuf, id: String, rx: watch::Receiver<bool>, tx: watch::Sender<bool>) {
    loop {
        if *rx.borrow()
            || Store::open(&db, true)
                .and_then(|s| s.control(&id))
                .ok()
                .flatten()
                .as_deref()
                == Some("cancel")
        {
            let _ = tx.send(true);
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}
fn write_log(c: &Context, id: &str, attempt: u32, r: &process::ProcessResult) -> Result<()> {
    let dir =
        c.db.parent()
            .unwrap()
            .join("logs")
            .join(&c.manifest.release_id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}-{attempt}.json"));
    std::fs::write(path, serde_json::to_vec(r)?)?;
    Ok(())
}
fn artifacts_current(c: &Context, run: &Run) -> bool {
    c.manifest.tasks.iter().all(|task| {
        task.acceptance.iter().all(|a| {
            if let Acceptance::Artifact { path, sha256 } = a {
                let root = &c.roots[&task.repo];
                let digest = root
                    .join(path)
                    .canonicalize()
                    .ok()
                    .filter(|p| p.starts_with(root))
                    .and_then(|p| std::fs::read(p).ok())
                    .map(|bytes| forge_domain::digest(&bytes));
                run.tasks[&task.id].status != "passed"
                    || digest.as_deref() == Some(&sha256.to_lowercase())
            } else {
                true
            }
        })
    })
}
pub fn assert_publishable(c: &Context) -> Result<Run> {
    assert_ready(c, true)
}
pub fn assert_ready(c: &Context, human: bool) -> Result<Run> {
    let (_, run) = saved(c)?;
    ensure!(
        run.manifest_hash == c.hash
            && run.revisions == revisions(&c.roots)?
            && artifacts_current(c, &run),
        "evidence stale"
    );
    ensure!(
        run.status == "passed"
            && run
                .tasks
                .values()
                .all(|t| t.status == "passed" && !t.evidence.is_empty()),
        "release has not passed all evidence gates"
    );
    ensure!(
        !human
            || run.effective_manifest.tasks.iter().any(|t| t
                .acceptance
                .iter()
                .any(|a| matches!(a, Acceptance::Human { .. }))),
        "publication requires a human acceptance gate"
    );
    ensure!(
        !c.manifest.memory.required || run.memory["status"] == "passed",
        "required memory has not passed"
    );
    Ok(run)
}

/// After a journaled local commit, gates must be checked against its new revision.
/// This never re-executes agent tasks or transfers an old human approval.
pub async fn reevaluate_commit(c: &Context, cancel: watch::Receiver<bool>) -> Result<Run> {
    ensure!(!*cancel.borrow(), "publication cancelled");
    let mut store = Store::open(&c.db, false)?;
    let mut run = store.load(&c.manifest.release_id)?.context("missing run")?;
    ensure!(
        run.manifest_hash == c.hash,
        "manifest changed during publication"
    );
    ensure!(
        run.tasks.values().all(|t| t.pid.is_none()),
        "active process during publication"
    );
    store.clear_approvals(&run.release_id)?;
    run.revisions = revisions(&c.roots)?;
    run.status = "evaluating".into();
    for task in run.tasks.values_mut() {
        task.status = "evaluating".into();
        task.evidence.retain(|e| e.kind == "execution");
        task.revision = None;
    }
    store.save(&run, "publication_commit_evaluation_started", &json!({}))?;
    for task in &c.manifest.tasks {
        ensure!(!*cancel.borrow(), "publication cancelled");
        evaluate(c, &mut store, &mut run, task, cancel.clone()).await?;
    }
    run.status = if run.tasks.values().all(|t| t.status == "passed") {
        "passed"
    } else if run.tasks.values().any(|t| t.status == "awaiting_approval") {
        "awaiting_approval"
    } else {
        "blocked"
    }
    .into();
    store.save(
        &run,
        "publication_commit_evaluated",
        &json!({"status":run.status}),
    )?;
    Ok(run)
}
