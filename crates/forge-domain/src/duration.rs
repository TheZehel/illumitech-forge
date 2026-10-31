//! Deterministic DAG/resource-aware active-work forecasts. No client, network or clock reads.
use crate::{Acceptance, Executor, Manifest, Run, digest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

pub const HISTORY_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct SecondsRange {
    pub lower: f64,
    pub upper: f64,
}
impl SecondsRange {
    fn add(&mut self, other: Self) {
        self.lower += other.lower;
        self.upper += other.upper;
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Counts {
    pub specs: usize,
    pub tasks: usize,
    pub gates: usize,
    /// References, including the final re-evaluation, before retry risk.
    pub gate_invocations: usize,
    pub artifact_checks: usize,
    pub human_gates: usize,
    pub retry_budget: u64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    pub available: bool,
    pub reason: Option<String>,
    pub considered: usize,
    pub compatible: usize,
    pub successful: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub timed_out: usize,
    pub unknown: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationEstimate {
    pub schema_version: u32,
    pub status: String,
    pub basis: String,
    pub confidence: String,
    pub manifest_hash: String,
    pub execution_context_hash: String,
    pub configuration_observability: String,
    pub phase: String,
    pub generated_at_epoch_ms: u64,
    pub remaining_seconds: Option<SecondsRange>,
    pub unavailable_reason: Option<String>,
    pub observed_active_seconds: Option<f64>,
    pub observed_active_ms: u64,
    pub in_progress_elapsed_seconds: Option<f64>,
    pub conditional: bool,
    pub waiting_for: Option<String>,
    /// Calendar availability and external waiting are deliberately not predicted.
    pub wall_clock_eta: Option<String>,
    pub assumptions: Vec<String>,
    pub counts: Counts,
    pub history: History,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub sequence: u64,
    pub context: String,
    pub task_id: String,
    pub phase: String,
    pub gate_id: Option<String>,
    pub attempt: u32,
    pub final_evaluation: bool,
    pub started_at_epoch_ms: u64,
    pub elapsed_ms: Option<u64>,
    pub outcome: String,
    pub owner_pid: u32,
    pub timeout_seconds: u64,
}
impl Measurement {
    pub fn valid(&self) -> bool {
        crate::valid_hash(&self.context)
            && crate::valid_id(&self.task_id)
            && self.owner_pid > 0
            && matches!(
                self.phase.as_str(),
                "execution" | "gate" | "artifact" | "human_check" | "engine_overhead"
            )
            && match self.outcome.as_str() {
                "incomplete" => self.elapsed_ms.is_none(),
                "passed" | "failed" | "blocked" | "error" | "output_limit" | "timeout"
                | "cancelled" => self.elapsed_ms.is_some(),
                _ => false,
            }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationState {
    pub generation: u64,
    pub initial_estimate: Option<DurationEstimate>,
    pub current_estimate: DurationEstimate,
    pub observed_active_ms: u64,
    #[serde(default)]
    pub task_active_ms: BTreeMap<String, u64>,
    pub observed_complete: bool,
    pub open_interval: Option<Measurement>,
    #[serde(default)]
    pub parallel_intervals: BTreeMap<String, Measurement>,
    pub last_measurement: Option<Measurement>,
    pub next_sequence: u64,
    pub final_evaluation: bool,
    pub final_completed: BTreeMap<String, usize>,
    pub evaluated: BTreeMap<String, usize>,
}
impl DurationState {
    pub fn new(initial: DurationEstimate, generation: u64) -> Self {
        Self {
            generation,
            initial_estimate: Some(initial.clone()),
            current_estimate: initial,
            observed_active_ms: 0,
            task_active_ms: BTreeMap::new(),
            observed_complete: true,
            open_interval: None,
            parallel_intervals: BTreeMap::new(),
            last_measurement: None,
            next_sequence: 0,
            final_evaluation: false,
            final_completed: BTreeMap::new(),
            evaluated: BTreeMap::new(),
        }
    }
    pub fn task_budget_ms(&self, task: &str) -> u64 {
        self.task_active_ms.get(task).copied().unwrap_or(0)
    }
    pub fn start(&mut self, mut measurement: Measurement) {
        // An interrupted interval is never converted into a completed measurement.
        if self.open_interval.is_some() {
            self.observed_complete = false;
        }
        measurement.sequence = self.next_sequence;
        self.next_sequence += 1;
        self.open_interval = Some(measurement);
    }
    pub fn finish(&mut self, elapsed_ms: u64, outcome: &str) {
        if let Some(mut m) = self.open_interval.take() {
            m.elapsed_ms = Some(elapsed_ms);
            m.outcome = outcome.into();
            self.observed_active_ms = self.observed_active_ms.saturating_add(elapsed_ms);
            let active = self.task_active_ms.entry(m.task_id.clone()).or_default();
            *active = active.saturating_add(elapsed_ms);
            self.last_measurement = Some(m);
        }
    }
}

/// Hash roots rather than persisting paths in measurement contexts. Include the whole
/// task contract and pinned gate inputs: IDs alone never establish comparability.
pub fn context_key(
    manifest: &Manifest,
    roots: &BTreeMap<String, PathBuf>,
    task_id: &str,
    phase: &str,
    gate_id: Option<&str>,
) -> String {
    let task = manifest.tasks.iter().find(|t| t.id == task_id).unwrap();
    let gate = gate_id.and_then(|id| manifest.gates.iter().find(|g| g.id == id));
    digest(
        &serde_json::to_vec(&(
            1,
            roots.get(&task.repo),
            task,
            &manifest.repos[&task.repo].specs,
            task.context.as_ref().unwrap_or(&manifest.context),
            phase,
            gate,
            std::env::consts::OS,
            std::env::consts::ARCH,
            env!("CARGO_PKG_VERSION"),
        ))
        .expect("serializable duration context"),
    )
}

fn unit_range(key: &str, fallback: SecondsRange, samples: &[Measurement]) -> (SecondsRange, usize) {
    let values: Vec<_> = samples
        .iter()
        .take(HISTORY_LIMIT)
        .filter(|m| m.context == key && m.outcome == "passed")
        .filter_map(|m| m.elapsed_ms.map(|ms| ms as f64 / 1000.0))
        .collect();
    if values.len() < 3 {
        return (fallback, values.len());
    }
    // An empirical envelope, not a statistical confidence interval. Retain a
    // minimum width for cached/near-instant operations and unseen variation.
    let lower = values.iter().copied().fold(f64::INFINITY, f64::min) * 0.5;
    let upper = (values.iter().copied().fold(0.0, f64::max) * 2.0).max(lower + 1.0);
    (SecondsRange { lower, upper }, values.len())
}

/// `now_ms` and liveness are injected. Only a live owner's currently open interval
/// can contribute provisional elapsed time; heartbeat and approval waits cannot.
#[allow(clippy::too_many_arguments)]
pub fn estimate(
    manifest: &Manifest,
    hash: &str,
    roots: &BTreeMap<String, PathBuf>,
    run: Option<&Run>,
    history: &[Measurement],
    history_reason: Option<String>,
    now_ms: u64,
    owner_alive: bool,
) -> DurationEstimate {
    let state = run.and_then(|r| r.duration.as_ref());
    let mut counts = Counts {
        specs: manifest.repos.values().map(|r| r.specs.len()).sum(),
        tasks: manifest.tasks.len(),
        gates: manifest.gates.len(),
        ..Counts::default()
    };
    let mut range = SecondsRange::default();
    let mut parallel_units = BTreeMap::new();
    let mut keys = std::collections::BTreeSet::new();
    let mut units = 0;
    let mut calibrated = 0;
    let mut native = false;
    let open = state.and_then(|s| s.open_interval.as_ref());
    let mut current_unit = None;
    let completed = run.is_some_and(|r| r.status == "passed");
    let stopped = run.is_some_and(|r| {
        matches!(
            r.status.as_str(),
            "paused" | "blocked" | "failed" | "awaiting_approval" | "cancelled"
        )
    });
    let mut exhausted = false;
    for task in &manifest.tasks {
        let ts = run.and_then(|r| r.tasks.get(&task.id));
        let passed = ts.is_some_and(|s| s.status == "passed");
        let evaluating =
            ts.is_some_and(|s| matches!(s.status.as_str(), "evaluating" | "awaiting_approval"));
        let final_mode = !stopped && state.is_some_and(|s| s.final_evaluation);
        exhausted |= !completed
            && !passed
            && !evaluating
            && ts.is_some_and(|s| s.status != "running" && s.attempts >= task.max_attempts);
        let command = matches!(task.executor, Executor::Command { .. });
        native |= !command;
        counts.retry_budget += u64::from(task.max_attempts.saturating_sub(1));
        let key = context_key(manifest, roots, &task.id, "execution", None);
        keys.insert(key.clone());
        let (execution, n) = unit_range(
            &key,
            if command {
                SecondsRange {
                    lower: 5.0,
                    upper: 120.0,
                }
            } else {
                SecondsRange {
                    lower: 60.0,
                    upper: 900.0,
                }
            },
            history,
        );
        if open.is_some_and(|m| m.context == key && m.phase == "execution") {
            current_unit = Some(execution);
        }
        units += 1;
        calibrated += usize::from(n >= 3);
        let mut retry_upper = execution.upper;
        let execute_pending = !completed && !passed && !evaluating && !final_mode;
        if execute_pending {
            parallel_units.insert(task.id.clone(), execution);
            range.add(execution);
            range.upper += 10.0; // Per-task local engine/revision overhead.
        }
        for (index, a) in task.acceptance.iter().enumerate() {
            let (phase, gate, default) = match a {
                Acceptance::Gate { id } => {
                    counts.gate_invocations += 2;
                    (
                        "gate",
                        Some(id.as_str()),
                        SecondsRange {
                            lower: 1.0,
                            upper: 120.0,
                        },
                    )
                }
                Acceptance::Artifact { .. } => {
                    counts.artifact_checks += 2;
                    (
                        "artifact",
                        None,
                        SecondsRange {
                            lower: 0.0,
                            upper: 2.0,
                        },
                    )
                }
                Acceptance::Human { .. } => {
                    counts.human_gates += 1;
                    continue;
                }
            };
            let key = context_key(manifest, roots, &task.id, phase, gate);
            keys.insert(key.clone());
            let (unit, n) = unit_range(&key, default, history);
            units += 1;
            calibrated += usize::from(n >= 3);
            if open.is_some_and(|m| m.context == key && m.phase == phase) {
                current_unit = Some(unit);
            }
            retry_upper += unit.upper;
            let evaluated =
                if execute_pending || ts.is_some_and(|s| s.status == "awaiting_approval") {
                    0
                } else {
                    state
                        .and_then(|s| s.evaluated.get(&task.id))
                        .copied()
                        .unwrap_or(0)
                };
            let final_done = if stopped {
                0
            } else {
                state
                    .and_then(|s| s.final_completed.get(&task.id))
                    .copied()
                    .unwrap_or(0)
            };
            if !completed && !passed && !final_mode && index >= evaluated {
                range.add(unit);
            }
            if !completed && index >= final_done {
                range.add(unit);
            }
        }
        if !completed && !passed && !final_mode {
            // Gate failure can still cause execution retries. A pending next
            // attempt is already in the primary range; do not count it twice.
            let attempts = ts.map_or(0, |t| t.attempts);
            let included = if execute_pending && ts.is_none_or(|t| t.status != "running") {
                attempts.saturating_add(1)
            } else {
                attempts.max(1)
            };
            let future = task.max_attempts.saturating_sub(included);
            range.upper += retry_upper * f64::from(future);
        }
    }
    if manifest.execution.max_parallel > 1 {
        let serial = parallel_units
            .values()
            .fold(SecondsRange::default(), |mut s, r| {
                s.add(*r);
                s
            });
        let parallel = wave_range(manifest, &parallel_units);
        range.lower = (range.lower - serial.lower + parallel.lower).max(0.0);
        range.upper = (range.upper - serial.upper + parallel.upper).max(range.lower);
    }
    if run.is_none() || run.is_some_and(|r| r.status == "not_run") {
        range.add(SecondsRange {
            lower: 2.0,
            upper: 30.0,
        });
    }
    let mut observed_ms = state.map_or(0, |s| s.observed_active_ms);
    let mut provisional = None;
    let mut unavailable = None;
    if let Some(open) = state.and_then(|s| s.open_interval.as_ref()) {
        let elapsed = now_ms.checked_sub(open.started_at_epoch_ms);
        if owner_alive
            && elapsed.is_some_and(|ms| {
                ms <= open.timeout_seconds.saturating_add(10).saturating_mul(1000)
            })
        {
            let ms = elapsed.unwrap();
            provisional = Some(ms as f64 / 1000.0);
            observed_ms = observed_ms.saturating_add(ms);
            let seconds = ms as f64 / 1000.0;
            if let Some(unit) = current_unit {
                // Elapsed time belongs only to this unit; downstream gates and
                // retry risk never disappear when a subprocess overruns its range.
                range.lower = (range.lower - seconds.min(unit.lower)).max(0.0);
                range.upper = (range.upper - seconds.min(unit.upper)).max(range.lower);
                if seconds >= unit.upper && !completed {
                    range.upper += (unit.upper * 0.5).max(1.0);
                }
            }
        } else {
            unavailable = Some("open interval has no verifiable live owner or valid clock; reconciliation required".into());
        }
    }
    if let Some(d) = state
        && !d.parallel_intervals.is_empty()
    {
        if !owner_alive
            || d.parallel_intervals.values().any(|m| {
                now_ms
                    .checked_sub(m.started_at_epoch_ms)
                    .is_none_or(|ms| ms > m.timeout_seconds.saturating_add(10).saturating_mul(1000))
            })
        {
            unavailable = Some(
                "parallel interval has no verifiable owner/clock; reconciliation required".into(),
            );
        } else {
            provisional = d
                .parallel_intervals
                .values()
                .map(|m| (now_ms - m.started_at_epoch_ms) as f64 / 1000.0)
                .max_by(f64::total_cmp);
        }
    }
    if exhausted {
        unavailable =
            Some("attempt budget exhausted; remaining execution requires reconciliation".into());
    }
    if run.is_some_and(|r| r.status == "cancelled") {
        unavailable = Some(
            "cancelled run requires reconciliation before remaining scope can be forecast".into(),
        );
    }
    if run.is_some() && state.is_none() {
        unavailable = Some("legacy run has no duration measurements or initial forecast".into());
    }
    if run.is_some_and(|r| r.manifest_hash != hash) {
        unavailable = Some("manifest changed; reconcile before forecasting this plan".into());
    }
    let waiting = run.and_then(|r| match r.status.as_str() {
        "paused" => Some("paused"),
        "awaiting_approval" => Some("human_approval"),
        "blocked" | "failed" => Some("operator_intervention"),
        "cancelled" => Some("cancelled; reconciliation required"),
        _ => None,
    });
    let mut h = History {
        available: history_reason.is_none(),
        reason: history_reason,
        considered: history.len().min(HISTORY_LIMIT),
        ..History::default()
    };
    for m in history
        .iter()
        .take(HISTORY_LIMIT)
        .filter(|m| keys.contains(&m.context))
    {
        h.compatible += 1;
        match m.outcome.as_str() {
            "passed" => h.successful += 1,
            "failed" | "blocked" | "error" | "output_limit" => h.failed += 1,
            "cancelled" => h.cancelled += 1,
            "timeout" => h.timed_out += 1,
            _ => h.unknown += 1,
        }
    }
    let all_calibrated = calibrated == units && units > 0;
    DurationEstimate {
        schema_version: 1,
        status: if unavailable.is_some() { "unavailable" } else if all_calibrated { "estimated" } else { "preliminary" }.into(),
        basis: if calibrated > 0 { "local_history_and_heuristics" } else { "heuristic" }.into(),
        confidence: if all_calibrated && !native { "medium" } else { "low" }.into(),
        manifest_hash: hash.into(),
        execution_context_hash: digest(&serde_json::to_vec(&(roots,manifest,std::env::consts::OS,std::env::consts::ARCH,env!("CARGO_PKG_VERSION"))).expect("serializable execution context")),
        configuration_observability: if native { "declared_native_configuration_only" } else { "command_contract_only" }.into(),
        phase: state.and_then(|s| s.open_interval.as_ref()).map(|m| if m.final_evaluation { "final_evaluation" } else { &m.phase }).unwrap_or_else(|| run.map_or("planning", |r| r.status.as_str())).into(),
        generated_at_epoch_ms: now_ms,
        remaining_seconds: if unavailable.is_some() { None } else { Some(range) },
        unavailable_reason: unavailable,
        observed_active_seconds: state.filter(|s| s.observed_complete).filter(|_| state.is_none_or(|s| s.open_interval.is_none()) || provisional.is_some()).map(|_| observed_ms as f64 / 1000.0),
        observed_active_ms: observed_ms,
        in_progress_elapsed_seconds: provisional,
        conditional: waiting.is_some() || (!completed && counts.human_gates > 0),
        waiting_for: waiting.map(str::to_string),
        wall_clock_eta: None,
        assumptions: vec![
            if manifest.execution.max_parallel == 1 { "Serial execution; dependencies change order, not concurrency.".into() }
            else { format!("DAG worktree waves, max_parallel={}, max_concurrency={:?}; one writer per root, gates/final integration serial; retry risk remains conservative.",manifest.execution.max_parallel,manifest.budget.max_concurrency) },
            "Active work only; human approval, pause and external waiting have unknown duration.".into(),
            "Timeouts are execution limits, never expected durations; upper includes permitted retry risk and final gates.".into(),
            "Local setup and engine overhead use heuristic allowances; measured invocation overhead is recorded at stop; idle time is excluded.".into(),
            if native { "Native CLI effective model/configuration, version, environment and cache are not fully observable; matching declared configuration gives partial compatibility only." }
            else { "Command contract, canonical root and platform match; environment, tool versions and cache remain unobserved." }.into(),
            "History is a bounded empirical envelope, not a calibrated probability or accuracy guarantee.".into(),
        ],
        counts,
        history: h,
    }
}

/// Simulate the actual deterministic wave policy, rather than dividing a serial
/// estimate by N. Dependency chains, shared roots and admission limits constrain it.
fn wave_range(manifest: &Manifest, units: &BTreeMap<String, SecondsRange>) -> SecondsRange {
    let mut done: std::collections::BTreeSet<String> = manifest
        .tasks
        .iter()
        .filter(|t| !units.contains_key(&t.id))
        .map(|t| t.id.clone())
        .collect();
    let mut total = SecondsRange::default();
    let order = manifest.validate().unwrap_or_default();
    let limit = manifest
        .execution
        .max_parallel
        .min(manifest.budget.max_concurrency.unwrap_or(32)) as usize;
    while done.len() < manifest.tasks.len() {
        let mut repos = std::collections::BTreeSet::new();
        let mut wave = Vec::new();
        let mut native = false;
        for id in &order {
            let t = manifest.tasks.iter().find(|t| &t.id == id).unwrap();
            if done.contains(id)
                || t.dependencies.iter().any(|d| !done.contains(d))
                || repos.contains(&t.repo)
            {
                continue;
            }
            let is_native = !matches!(t.executor, Executor::Command { .. });
            if native && is_native && manifest.budget.max_tokens.is_some() {
                continue;
            }
            native |= is_native;
            repos.insert(t.repo.clone());
            wave.push(id.clone());
            if wave.len() >= limit {
                break;
            }
        }
        if wave.is_empty() {
            break;
        }
        let mut longest = SecondsRange::default();
        for id in wave {
            if let Some(r) = units.get(&id) {
                longest.lower = longest.lower.max(r.lower);
                longest.upper = longest.upper.max(r.upper);
            }
            done.insert(id);
        }
        total.add(longest);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> (Manifest, BTreeMap<String, PathBuf>) {
        let m: Manifest = serde_json::from_value(json!({
            "schema_version":1,"release_id":"duration",
            "repos":{"app":{"path":"project","specs":["020-duration"]}},
            "tasks":[{"id":"work","repo":"app","description":"implement", "executor":{"kind":"command","argv":["true"]},
            "acceptance":[{"kind":"gate","id":"check"}],"max_attempts":3}],
            "gates":[{"id":"check","argv":["true"],"inputs":{"external-check":"a".repeat(64)}}]
        })).unwrap();
        (
            m,
            BTreeMap::from([("app".into(), PathBuf::from("project"))]),
        )
    }
    fn forecast(
        m: &Manifest,
        roots: &BTreeMap<String, PathBuf>,
        run: Option<&Run>,
        history: &[Measurement],
        now: u64,
        alive: bool,
    ) -> DurationEstimate {
        estimate(m, "plan", roots, run, history, None, now, alive)
    }
    fn run(m: &Manifest, roots: &BTreeMap<String, PathBuf>) -> Run {
        let initial = forecast(m, roots, None, &[], 1000, true);
        let mut r = Run::new(m.clone(), "plan".into(), BTreeMap::new());
        r.duration = Some(DurationState::new(initial, 0));
        r
    }
    fn sample(
        m: &Manifest,
        roots: &BTreeMap<String, PathBuf>,
        phase: &str,
        outcome: &str,
    ) -> Measurement {
        Measurement {
            sequence: 0,
            context: context_key(
                m,
                roots,
                "work",
                phase,
                if phase == "gate" { Some("check") } else { None },
            ),
            task_id: "work".into(),
            phase: phase.into(),
            gate_id: None,
            attempt: 1,
            final_evaluation: false,
            started_at_epoch_ms: 1000,
            elapsed_ms: Some(10_000),
            outcome: outcome.into(),
            owner_pid: 1,
            timeout_seconds: 1800,
        }
    }
    #[test]
    fn heuristic_is_deterministic_serial_and_not_timeout() {
        let (mut m, roots) = fixture();
        let a = forecast(&m, &roots, None, &[], 1000, true);
        assert_eq!(a.counts.specs, 1);
        assert_eq!(a.counts.gate_invocations, 2);
        assert_eq!(a.counts.retry_budget, 2);
        assert_eq!(a.confidence, "low");
        assert_eq!(a.status, "preliminary");
        assert_eq!(
            a.remaining_seconds,
            Some(SecondsRange {
                lower: 9.0,
                upper: 880.0
            })
        );
        m.tasks[0].timeout_seconds = 60;
        m.gates[0].timeout_seconds = 1;
        let b = forecast(&m, &roots, None, &[], 1000, true);
        assert_eq!(a.remaining_seconds, b.remaining_seconds);
        let mut second = m.tasks[0].clone();
        second.id = "dependent".into();
        second.dependencies = vec!["work".into()];
        m.tasks.push(second);
        let c = forecast(&m, &roots, None, &[], 1000, true);
        assert_eq!(c.counts.gate_invocations, 4);
        assert_eq!(c.remaining_seconds.unwrap().lower, 16.0);
        assert_eq!(c.remaining_seconds.unwrap().upper, 1730.0);
    }
    #[test]
    fn history_uses_minimum_samples_and_exact_context_not_ids() {
        let (mut m, mut roots) = fixture();
        let mut samples = vec![sample(&m, &roots, "execution", "passed"); 3];
        samples.extend(vec![sample(&m, &roots, "gate", "passed"); 3]);
        samples.push(sample(&m, &roots, "execution", "timeout"));
        samples.push(sample(&m, &roots, "execution", "cancelled"));
        let e = forecast(&m, &roots, None, &samples, 1000, true);
        assert_eq!(e.status, "estimated");
        assert_eq!(e.confidence, "medium");
        assert_eq!(e.history.successful, 6);
        assert_eq!(e.history.timed_out, 1);
        assert_eq!(e.history.cancelled, 1);
        assert!(e.remaining_seconds.unwrap().upper < 880.0);
        roots.insert("app".into(), PathBuf::from("other-project"));
        assert_eq!(
            forecast(&m, &roots, None, &samples, 1000, true)
                .history
                .compatible,
            0
        );
        roots.insert("app".into(), PathBuf::from("project"));
        m.tasks[0].description = "different scope".into();
        assert_eq!(
            forecast(&m, &roots, None, &samples, 1000, true)
                .history
                .compatible,
            0
        );
        let (mut m, roots) = fixture();
        m.tasks[0].executor = Executor::Codex {
            permission: crate::Permission::ReadOnly,
            model: Some("declared-model".into()),
        };
        assert_eq!(
            forecast(&m, &roots, None, &samples, 1000, true)
                .history
                .compatible,
            0
        );
        let mut native = vec![sample(&m, &roots, "execution", "passed"); 3];
        native.extend(vec![sample(&m, &roots, "gate", "passed"); 3]);
        let e = forecast(&m, &roots, None, &native, 1000, true);
        assert_eq!(e.confidence, "low");
        assert!(
            e.assumptions
                .iter()
                .any(|a| a.contains("not fully observable"))
        );
        m.tasks[0].executor = Executor::Codex {
            permission: crate::Permission::ReadOnly,
            model: Some("different-model".into()),
        };
        assert_eq!(
            forecast(&m, &roots, None, &native, 1000, true)
                .history
                .compatible,
            0
        );
    }
    #[test]
    fn waits_do_not_consume_active_time_and_legacy_is_unknown() {
        let (m, roots) = fixture();
        let mut r = run(&m, &roots);
        r.duration.as_mut().unwrap().observed_active_ms = 7000;
        for (status, wait) in [
            ("paused", "paused"),
            ("awaiting_approval", "human_approval"),
            ("blocked", "operator_intervention"),
        ] {
            r.status = status.into();
            let a = forecast(&m, &roots, Some(&r), &[], 2000, true);
            let b = forecast(&m, &roots, Some(&r), &[], 2_000_000, true);
            assert_eq!(a.remaining_seconds, b.remaining_seconds);
            assert_eq!(b.observed_active_seconds, Some(7.0));
            assert_eq!(b.waiting_for.as_deref(), Some(wait));
            assert!(b.wall_clock_eta.is_none());
        }
        r.duration = None;
        let e = forecast(&m, &roots, Some(&r), &[], 2_000_000, true);
        assert!(e.remaining_seconds.is_none());
        assert!(e.observed_active_seconds.is_none());
        assert!(e.unavailable_reason.unwrap().contains("legacy"));
    }
    #[test]
    fn live_projection_saturates_but_orphan_and_clock_reversal_stay_unknown() {
        let (m, roots) = fixture();
        let mut r = run(&m, &roots);
        let mut interval = sample(&m, &roots, "execution", "incomplete");
        interval.elapsed_ms = None;
        r.status = "running".into();
        r.duration.as_mut().unwrap().start(interval);
        let e = forecast(&m, &roots, Some(&r), &[], 1_000_000, true);
        assert!(e.remaining_seconds.unwrap().lower >= 2.0);
        assert!(e.remaining_seconds.unwrap().upper >= 1.0);
        assert_eq!(e.in_progress_elapsed_seconds, Some(999.0));
        for (now, alive) in [(3000, false), (999, true), (3_000_000, true)] {
            let e = forecast(&m, &roots, Some(&r), &[], now, alive);
            assert!(e.remaining_seconds.is_none());
            assert!(e.observed_active_seconds.is_none());
        }
        r.manifest_hash = "changed".into();
        assert!(
            forecast(&m, &roots, Some(&r), &[], 2000, true)
                .unavailable_reason
                .unwrap()
                .contains("manifest changed")
        );
    }
    #[test]
    fn measurements_accumulate_retries_and_final_gate_progress() {
        let (m, roots) = fixture();
        let mut r = run(&m, &roots);
        let d = r.duration.as_mut().unwrap();
        let initial = d.initial_estimate.clone();
        for outcome in ["failed", "timeout", "cancelled", "passed"] {
            let mut m = sample(&m, &roots, "execution", "incomplete");
            m.elapsed_ms = None;
            d.start(m);
            d.finish(1200, outcome);
        }
        assert_eq!(d.observed_active_ms, 4800);
        assert_eq!(d.next_sequence, 4);
        assert_eq!(
            serde_json::to_value(initial).unwrap(),
            serde_json::to_value(&d.initial_estimate).unwrap()
        );
        r.tasks.get_mut("work").unwrap().status = "passed".into();
        r.status = "running".into();
        r.duration.as_mut().unwrap().final_evaluation = true;
        assert_eq!(
            forecast(&m, &roots, Some(&r), &[], 1000, true)
                .remaining_seconds
                .unwrap(),
            SecondsRange {
                lower: 1.0,
                upper: 120.0
            }
        );
        r.duration
            .as_mut()
            .unwrap()
            .final_completed
            .insert("work".into(), 1);
        assert_eq!(
            forecast(&m, &roots, Some(&r), &[], 1000, true)
                .remaining_seconds
                .unwrap(),
            SecondsRange::default()
        );
        r.status = "passed".into();
        assert_eq!(
            forecast(&m, &roots, Some(&r), &[], 1000, true)
                .remaining_seconds
                .unwrap(),
            SecondsRange::default()
        );
    }
    #[test]
    fn failures_are_not_successful_timing_samples() {
        let (m, roots) = fixture();
        let mut samples = vec![sample(&m, &roots, "execution", "failed"); HISTORY_LIMIT + 10];
        samples.extend(vec![sample(&m, &roots, "execution", "passed"); 3]);
        let e = forecast(&m, &roots, None, &samples, 0, true);
        assert_eq!(e.basis, "heuristic");
        assert_eq!(e.history.considered, HISTORY_LIMIT);
        assert_eq!(e.history.failed, HISTORY_LIMIT);
        assert_eq!(e.history.successful, 0);
    }
    #[test]
    fn retries_remain_risk_during_gates_and_consumed_attempts_are_not_recounted() {
        let (m, roots) = fixture();
        let mut r = run(&m, &roots);
        r.status = "running".into();
        let task = r.tasks.get_mut("work").unwrap();
        task.status = "evaluating".into();
        task.attempts = 1;
        let e = forecast(&m, &roots, Some(&r), &[], 1000, true);
        assert_eq!(
            e.remaining_seconds.unwrap(),
            SecondsRange {
                lower: 2.0,
                upper: 720.0
            }
        );
        let task = r.tasks.get_mut("work").unwrap();
        task.status = "failed".into();
        r.duration
            .as_mut()
            .unwrap()
            .evaluated
            .insert("work".into(), 1);
        let e = forecast(&m, &roots, Some(&r), &[], 1000, true);
        assert_eq!(
            e.remaining_seconds.unwrap(),
            SecondsRange {
                lower: 7.0,
                upper: 610.0
            }
        );
        r.tasks.get_mut("work").unwrap().attempts = 3;
        assert!(
            forecast(&m, &roots, Some(&r), &[], 1000, true)
                .remaining_seconds
                .is_none()
        );
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn critical_path_respects_dependencies_roots_and_resource_limits() {
        let mut m:Manifest=serde_json::from_value(json!({"schema_version":1,"release_id":"parallel","execution":{"max_parallel":2},"repos":{"a":{"path":"a"},"b":{"path":"b"}},"tasks":[
            {"id":"a","repo":"a","description":"a","executor":{"kind":"command","argv":["true"]},"acceptance":[{"kind":"human","id":"check"}]},
            {"id":"b","repo":"b","description":"b","executor":{"kind":"command","argv":["true"]},"acceptance":[{"kind":"human","id":"check"}]},
            {"id":"c","repo":"b","description":"c","dependencies":["a","b"],"executor":{"kind":"command","argv":["true"]},"acceptance":[{"kind":"human","id":"check"}]}]})).unwrap();
        let units: BTreeMap<_, _> = [("a", 10.0), ("b", 15.0), ("c", 20.0)]
            .into_iter()
            .map(|(id, n)| (id.into(), SecondsRange { lower: n, upper: n }))
            .collect();
        assert_eq!(wave_range(&m, &units).upper, 35.0);
        m.budget.max_concurrency = Some(1);
        assert_eq!(wave_range(&m, &units).upper, 45.0);
        m.budget.max_concurrency = None;
        m.tasks[1].repo = "a".into();
        assert_eq!(wave_range(&m, &units).upper, 45.0);
    }
}
