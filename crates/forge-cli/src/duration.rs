//! Read-only projections and local clocks for the pure domain estimator.
use crate::engine::Context;
use forge_adapters::process;
use forge_domain::{Run, Store, Task, duration::*};
use serde_json::{Value, json};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}
pub fn project(c: &Context, run: Option<&Run>, store: Option<&Store>) -> DurationEstimate {
    let (history, reason) = store
        .map(Store::duration_history)
        .unwrap_or_else(|| (vec![], Some("no local duration history".into())));
    let alive = run
        .and_then(|r| r.duration.as_ref())
        .and_then(|d| d.open_interval.as_ref())
        .is_none_or(|m| process::pid_alive(m.owner_pid))
        && run.and_then(|r| r.duration.as_ref()).is_none_or(|d| {
            d.parallel_intervals
                .values()
                .all(|m| process::pid_alive(m.owner_pid))
        });
    estimate(
        &c.manifest,
        &c.hash,
        &c.roots,
        run,
        &history,
        reason,
        now_ms(),
        alive,
    )
}
pub fn inspect(c: &Context) -> DurationEstimate {
    if !c.db.exists() {
        return project(c, None, None);
    }
    match Store::open(&c.db, true) {
        Ok(store) => project(c, None, Some(&store)),
        Err(_) => {
            let mut e = project(c, None, None);
            e.history.reason = Some("state is unreadable or incompatible; heuristic only".into());
            e
        }
    }
}
pub fn refresh(c: &Context, store: &Store, run: &mut Run) {
    let e = project(c, Some(run), Some(store));
    if let Some(d) = &mut run.duration {
        d.current_estimate = e;
    }
}
#[allow(clippy::too_many_arguments)]
pub fn start(
    c: &Context,
    store: &Store,
    run: &mut Run,
    task: &Task,
    phase: &str,
    gate: Option<&str>,
    timeout: u64,
) -> Instant {
    let attempt = run.tasks[&task.id].attempts;
    if let Some(d) = &mut run.duration {
        d.start(Measurement {
            sequence: 0,
            context: context_key(&c.manifest, &c.roots, &task.id, phase, gate),
            task_id: task.id.clone(),
            phase: phase.into(),
            gate_id: gate.map(str::to_owned),
            attempt,
            final_evaluation: d.final_evaluation,
            started_at_epoch_ms: now_ms(),
            elapsed_ms: None,
            outcome: "incomplete".into(),
            owner_pid: std::process::id(),
            timeout_seconds: timeout,
        });
    }
    refresh(c, store, run);
    Instant::now()
}
pub fn finish(run: &mut Run, started: Instant, outcome: &str) {
    if let Some(d) = &mut run.duration {
        d.finish(
            started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            outcome,
        );
    }
}
pub fn comparison(run: &Run) -> Value {
    let Some(d) = &run.duration else {
        return json!({"status":"unavailable","reason":"legacy run has no initial forecast"});
    };
    let Some(initial_estimate) = &d.initial_estimate else {
        return json!({"status":"unavailable","reason":"legacy run has no initial forecast","completed_intervals_ms":d.observed_active_ms});
    };
    let actual = d.observed_active_ms as f64 / 1000.0;
    let initial = initial_estimate.remaining_seconds;
    let known = d.observed_complete && d.open_interval.is_none();
    let complete = known && run.status == "passed";
    json!({"status":if complete {"complete"} else {"partial"},"outcome":run.status,
        "initial_estimate":d.initial_estimate,"observed_active_seconds":if known {Some(actual)} else {None},
        "completed_intervals_seconds":actual,"coverage":if complete {initial.map(|r|actual>=r.lower&&actual<=r.upper)} else {None},
        "error_seconds":if complete {initial.map(|r|actual-(r.lower+r.upper)/2.0)} else {None},
        "metrics_scope":"active execution through the last checkpoint, including measured local engine overhead; idle and human waiting excluded",
        "accuracy_target":null})
}
pub fn human(value: &Value) -> Option<String> {
    let e = &value["duration_estimate"];
    if e.is_null() {
        return None;
    }
    if let (Some(lo), Some(hi)) = (
        e["remaining_seconds"]["lower"].as_f64(),
        e["remaining_seconds"]["upper"].as_f64(),
    ) {
        Some(format!(
            "Estimated active work remaining: {lo:.1}–{hi:.1} seconds; confidence: {}. {}{}",
            e["confidence"].as_str().unwrap_or("low"),
            if e["conditional"].as_bool() == Some(true) {
                "Conditional on approval/resumption; waiting duration unknown. "
            } else {
                ""
            },
            "Range is uncertain, not a deadline; concurrency follows the inspected execution policy."
        ))
    } else {
        Some(format!(
            "Duration unavailable: {}",
            e["unavailable_reason"]
                .as_str()
                .unwrap_or("unknown measurements")
        ))
    }
}

/// Account for active local preparation and bookkeeping once per execute invocation.
/// Child intervals are already included; subtract them before adding overhead.
pub fn finish_session(c: &Context, run: &mut Run, started: Instant, previous_ms: u64) {
    let total_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    if let Some(d) = &mut run.duration
        && d.open_interval.is_none()
    {
        let overhead = total_ms.saturating_sub(d.observed_active_ms.saturating_sub(previous_ms));
        let task = &c.manifest.tasks[0];
        d.start(Measurement {
            sequence: 0,
            context: context_key(&c.manifest, &c.roots, &task.id, "engine_overhead", None),
            task_id: task.id.clone(),
            phase: "engine_overhead".into(),
            gate_id: None,
            attempt: 0,
            final_evaluation: false,
            started_at_epoch_ms: now_ms().saturating_sub(total_ms),
            elapsed_ms: None,
            outcome: "incomplete".into(),
            owner_pid: std::process::id(),
            timeout_seconds: 0,
        });
        d.finish(overhead, "passed");
    }
}
