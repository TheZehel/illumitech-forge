//! Bounded worktree waves. Workers return bounded process results; only this coordinator
//! owns SQLite and the release ledger. No implicit integration commits or file copies.
use crate::{
    budget, duration,
    engine::{self, Context},
};
use anyhow::{Result, ensure};
use forge_adapters::{process, reliability, usage};
use forge_domain::{usage::TokenCall, *};
use serde_json::json;
use std::{collections::BTreeSet, time::Instant};
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
};

pub fn limit(c: &Context) -> usize {
    c.manifest
        .execution
        .max_parallel
        .min(c.manifest.budget.max_concurrency.unwrap_or(32)) as usize
}
struct Completed {
    task: Task,
    before: Revision,
    result: Result<process::ProcessResult>,
    call: Option<TokenCall>,
    started: Instant,
}
/// Worktrees are the integration surface selected by the operator. Conflicted Git
/// entries block with a preview, preserving every byte for manual resolution.
pub fn integration(c: &Context) -> Result<()> {
    for (repo, root) in &c.roots {
        let conflicts = git(root, &["ls-files", "--unmerged"])?;
        ensure!(
            conflicts.is_empty(),
            "integration conflict in {repo}; preserved files, resolve explicitly:\n{}",
            conflicts.chars().take(8192).collect::<String>()
        );
    }
    Ok(())
}
pub async fn execute(
    c: &Context,
    store: &mut Store,
    run: &mut Run,
    cancel: watch::Receiver<bool>,
    session: Instant,
    previous_ms: u64,
) -> Result<()> {
    if let Err(error) = integration(c) {
        run.status = "blocked".into();
        store.save(
            run,
            "integration_conflict",
            &json!({"preview":error.to_string(),"files":"preserved"}),
        )?;
        return Ok(());
    }
    let order = c.manifest.validate()?;
    let mut reviewed_waits = BTreeSet::new();
    loop {
        if *cancel.borrow() {
            run.status = "cancelled".into();
            return Ok(());
        }
        if engine::apply_control(store, run)? {
            return Ok(());
        }
        let mut repos = BTreeSet::new();
        let mut selected = Vec::new();
        // A token-limited native invocation cannot reserve unknown future usage.
        // Serialize native work under that ceiling until observed usage is available.
        let mut native_in_flight = false;
        for id in &order {
            let t = c.manifest.tasks.iter().find(|t| &t.id == id).unwrap();
            let s = &run.tasks[id];
            if matches!(s.status.as_str(), "passed" | "blocked")
                || (s.status == "awaiting_approval" && reviewed_waits.contains(id))
                || t.dependencies
                    .iter()
                    .any(|d| run.tasks[d].status != "passed")
            {
                continue;
            }
            let native = !matches!(t.executor, Executor::Command { .. });
            if native && native_in_flight && c.manifest.budget.max_tokens.is_some() {
                continue;
            }
            if repos.contains(&t.repo) {
                continue;
            }
            if !matches!(s.status.as_str(), "evaluating" | "awaiting_approval")
                && s.attempts >= t.max_attempts
            {
                continue;
            }
            repos.insert(t.repo.clone());
            native_in_flight |= native;
            selected.push(t.clone());
            if selected.len() >= limit(c) {
                break;
            }
        }
        if selected.is_empty() {
            break;
        }
        let (stop, worker_cancel) = watch::channel(false);
        let watcher = engine::AbortOnDrop(tokio::spawn(engine::watch_stop(
            c.db.clone(),
            run.release_id.clone(),
            cancel.clone(),
            stop,
            true,
        )));
        let (spawn_tx, mut spawns) = mpsc::channel::<(String, u32)>(32);
        let mut workers = JoinSet::new();
        let mut evaluate = Vec::new();
        for task in selected {
            let id = task.id.clone();
            if matches!(
                run.tasks[&id].status.as_str(),
                "evaluating" | "awaiting_approval"
            ) {
                evaluate.push(task);
                continue;
            }
            let active_ms = previous_ms
                .saturating_add(session.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
            let admission = budget::check(c, store, run, &task, active_ms)
                .and_then(|()| budget::timeout(c, run, &task, task.timeout_seconds, active_ms));
            let timeout = match admission {
                Ok(n) => n,
                Err(e) => {
                    let s = run.tasks.get_mut(&id).unwrap();
                    s.status = "blocked".into();
                    s.evidence.push(Evidence {
                        kind: "budget".into(),
                        id: id.clone(),
                        outcome: Outcome::Blocked,
                        digest: None,
                        detail: e.to_string(),
                    });
                    store.save(
                        run,
                        "budget_blocked",
                        &json!({"task":id,"reason":e.to_string()}),
                    )?;
                    continue;
                }
            };
            let memory = run.memory["repos"][&task.repo]["context"].to_string();
            let (command, plan) = engine::task_command(c, &task, &memory)?;
            if plan.as_ref().is_some_and(|p| p.status == "blocked") {
                let s = run.tasks.get_mut(&id).unwrap();
                s.status = "blocked".into();
                s.evidence.push(Evidence {
                    kind: "execution".into(),
                    id: id.clone(),
                    outcome: Outcome::Blocked,
                    digest: None,
                    detail: json!({"context_plan":plan}).to_string(),
                });
                store.save(run, "context_blocked", &json!({"task":id}))?;
                continue;
            }
            let root = c.roots[&task.repo].clone();
            let before = revision(&root)?;
            let s = run.tasks.get_mut(&id).unwrap();
            s.attempts += 1;
            s.status = "running".into();
            s.evidence.clear();
            let attempt = s.attempts;
            let generation = run.duration.as_ref().map_or(0, |d| d.generation);
            let call = engine::client_name(&task).map(|client| TokenCall {
                id: format!("{generation}:{id}:{attempt}:execution"),
                task_id: id.clone(),
                attempt,
                phase: "execution".into(),
                repo_scope: digest(root.as_os_str().as_encoded_bytes()),
                client: client.into(),
                model: match &task.executor {
                    Executor::Codex { model, .. } | Executor::Claude { model, .. } => model.clone(),
                    _ => None,
                },
                outcome: "running".into(),
                elapsed_ms: None,
                observation: Default::default(),
            });
            if let Some(d) = &mut run.duration {
                let m = forge_domain::duration::Measurement {
                    sequence: d.next_sequence,
                    context: forge_domain::duration::context_key(
                        &c.manifest,
                        &c.roots,
                        &id,
                        "execution",
                        None,
                    ),
                    task_id: id.clone(),
                    phase: "execution".into(),
                    gate_id: None,
                    attempt,
                    final_evaluation: false,
                    started_at_epoch_ms: duration::now_ms(),
                    elapsed_ms: None,
                    outcome: "incomplete".into(),
                    owner_pid: std::process::id(),
                    timeout_seconds: timeout,
                };
                d.next_sequence += 1;
                d.parallel_intervals.insert(id.clone(), m);
                d.evaluated.remove(&id);
            }
            run.last_token_call = call.clone();
            store.save(run,"task_started",&json!({"task":id,"attempt":attempt,"parallel":true,"context_plan":plan,"owner_pid":std::process::id()}))?;
            let tx = spawn_tx.clone();
            let rx = worker_cancel.clone();
            workers.spawn(async move {
                let started = Instant::now();
                let result = process::run_observed(&command, &root, timeout, rx, |pid| {
                    tx.try_send((id, pid))
                        .map_err(|e| anyhow::anyhow!("coordinator spawn queue: {e}"))
                })
                .await;
                Completed {
                    task,
                    before,
                    result,
                    call,
                    started,
                }
            });
        }
        drop(spawn_tx);
        while !workers.is_empty() {
            tokio::select! {
                Some((id,pid)) = spawns.recv() => {
                    run.tasks.get_mut(&id).unwrap().pid=Some(pid);
                    store.save(run,"process_spawned",&json!({"task":id,"pid":pid,"parallel":true}))?;
                }
                result = workers.join_next() => {
                    let done=result.expect("worker exists")?;
                    // Drain spawn notifications first; never restore a finished PID later.
                    while let Ok((id,pid))=spawns.try_recv() {
                        run.tasks.get_mut(&id).unwrap().pid=Some(pid);
                        store.save(run,"process_spawned",&json!({"task":id,"pid":pid,"parallel":true}))?;
                    }
                    if let Some(task)=record(c,store,run,done,session,previous_ms)? {evaluate.push(task);}

                }
            }
        }
        drop(watcher);
        if *cancel.borrow() {
            run.status = "cancelled".into();
            return Ok(());
        }
        if engine::apply_control(store, run)? {
            return Ok(());
        }
        if let Err(error) = integration(c) {
            run.status = "blocked".into();
            store.save(
                run,
                "integration_conflict",
                &json!({"preview":error.to_string(),"files":"preserved"}),
            )?;
            return Ok(());
        }
        // No writer runs while gates inspect the integrated set of selected roots.
        run.revisions = revisions(&c.roots)?;
        for task in evaluate {
            reviewed_waits.insert(task.id.clone());
            engine::evaluate(c, store, run, &task, cancel.clone()).await?;
            if *cancel.borrow() {
                run.status = "cancelled".into();
                return Ok(());
            }
            if engine::apply_control(store, run)? {
                return Ok(());
            }
            let s = &run.tasks[&task.id];
            if s.status == "blocked"
                && matches!(task.executor, Executor::Command { .. })
                && s.attempts < task.max_attempts
                && s.evidence.iter().any(|e| e.outcome == Outcome::Failed)
                && !s.evidence.iter().any(|e| e.outcome == Outcome::Blocked)
            {
                run.tasks.get_mut(&task.id).unwrap().status = "pending".into();
            }
        }
    }
    run.status = if run.tasks.values().all(|s| s.status == "passed") {
        "running"
    } else if run.tasks.values().any(|s| s.status == "awaiting_approval") {
        "awaiting_approval"
    } else {
        "blocked"
    }
    .into();
    Ok(())
}

fn record(
    c: &Context,
    store: &mut Store,
    run: &mut Run,
    mut done: Completed,
    session: Instant,
    previous_ms: u64,
) -> Result<Option<Task>> {
    let id = &done.task.id;
    run.tasks.get_mut(id).unwrap().pid = None;
    let failure = match &done.result {
        Ok(r) => reliability::assess(engine::client_name(&done.task), r).err(),
        Err(e) => Some(reliability::spawn_failure(e)),
    };
    if let Some(call) = &mut done.call {
        call.outcome = if failure.is_none() {
            "completed"
        } else {
            "failed"
        }
        .into();
        if let Ok(r) = &done.result {
            call.elapsed_ms = Some(r.elapsed_ms.min(u128::from(u64::MAX)) as u64);
            call.observation = usage::observe(
                &call.client,
                &r.stdout,
                &call.id,
                r.reason == "exited" && !r.truncated,
            );
        }
    }
    run.last_token_call = done.call;
    store.save(
        run,
        "client_call_finished",
        &json!({"task":id,"call":run.last_token_call}),
    )?;
    let after = revision(&c.roots[&done.task.repo])?;
    let readonly = matches!(
        done.task.executor,
        Executor::Codex {
            permission: Permission::ReadOnly,
            ..
        } | Executor::Claude {
            permission: Permission::ReadOnly,
            ..
        }
    );
    let violation = readonly && done.before != after;
    let success = failure.is_none() && !violation;
    let retry = !violation
        && failure.is_some_and(|f| f.retryable())
        && run.tasks[id].attempts < done.task.max_attempts;
    let s = run.tasks.get_mut(id).unwrap();
    s.status = if success {
        "evaluating"
    } else if retry {
        "pending"
    } else {
        "blocked"
    }
    .into();
    s.revision = Some(after.clone());
    let detail = match &done.result {
        Ok(r) => {
            engine::write_log(
                c,
                id,
                s.attempts,
                run.duration.as_ref().map_or(0, |d| d.generation),
                r,
            )?;
            json!({"reason":r.reason,"code":r.code,"failure_kind":failure,"readonly_preserved":!violation,"parallel":true})
        }
        Err(e) => json!({"error":process::redact(&e.to_string()),"failure_kind":failure}),
    };
    s.evidence.push(Evidence {
        kind: "execution".into(),
        id: id.clone(),
        outcome: if success {
            Outcome::Passed
        } else if retry {
            Outcome::Failed
        } else {
            Outcome::Blocked
        },
        digest: None,
        detail: detail.to_string(),
    });
    run.revisions.insert(done.task.repo.clone(), after);
    if let Some(d) = &mut run.duration {
        let elapsed = done.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        *d.task_active_ms.entry(id.clone()).or_default() += elapsed;
        if let Some(mut m) = d.parallel_intervals.remove(id) {
            m.elapsed_ms = Some(elapsed);
            m.outcome = if violation {
                "blocked"
            } else {
                match &done.result {
                    Ok(r) if r.passed() => "passed",
                    Ok(r) if r.reason == "exited" => "failed",
                    Ok(r) => r.reason.as_str(),
                    Err(_) => "error",
                }
            }
            .into();
            d.last_measurement = Some(m);
        }
        d.observed_active_ms = previous_ms
            .saturating_add(session.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
    }
    duration::refresh(c, store, run);
    store.save(run, "task_exited", &json!({"task":id,"result":detail}))?;
    Ok(if success { Some(done.task) } else { None })
}
