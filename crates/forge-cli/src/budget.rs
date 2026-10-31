//! Local admission checks; no balance inference and no implicit CI activation.
use crate::engine::Context;
use anyhow::{Result, bail};
use forge_domain::{Budget, Executor, Run, Store, Task};
use serde_json::{Value, json};

pub fn inspect(c: &Context) -> Result<Value> {
    let read = || -> Result<_> {
        Ok(if c.db.exists() {
            let store = Store::open(&c.db, true)?;
            (
                store.token_usage(&c.manifest.release_id, None)?,
                store.load(&c.manifest.release_id)?,
                store
                    .metadata(&format!("native-active:{}", c.manifest.release_id))?
                    .and_then(|v| v.as_u64()),
                Some(store.budget_attempts(&c.manifest.release_id, None)?),
            )
        } else {
            (
                forge_domain::unavailable_token_usage("no calls observed"),
                None,
                None,
                None,
            )
        })
    };
    let (usage, run, external_ms, attempts) = read().unwrap_or_else(|_| {
        (
            forge_domain::unavailable_token_usage(
                "operational counters unavailable or incompatible; no state modified",
            ),
            None,
            None,
            None,
        )
    });
    Ok(
        json!({"schema_version":1,"policy":c.manifest.budget,"task_policies":c.manifest.tasks.iter().map(|t|json!({"task":t.id,"budget":t.budget,"max_attempts":t.max_attempts,"timeout_seconds":t.timeout_seconds})).collect::<Vec<_>>(),"observed":{"tokens":usage,"active_seconds":run.as_ref().and_then(|r|r.duration.as_ref()).map(|d|d.observed_active_ms.saturating_add(external_ms.unwrap_or(0))/1000).or(external_ms.map(|ms|ms/1000)),"attempts":attempts,"concurrency":run.as_ref().map_or(0,|r|r.tasks.values().filter(|t|t.pid.is_some()).count()),"effective_max_parallel":crate::parallel::limit(c)},"available_balance":null,"cost_micros":null,"actions":c.manifest.budget.actions,"provider_quota":"unverified","state":"not_modified","limitations":["Native limits are checked before each invocation; no hard per-inference token ceiling is supported","Missing usage blocks subsequent budgeted inference, including failed calls","Active time includes local execution, gates and attached native calls; Forge human wait and paused time excluded; native tool approval wait counts toward its call deadline","Commands/tests have time and attempt limits; their token/currency usage is unobservable","Monetary ceilings fail closed for inference because native subscription cost is unknown","Forge never dispatches workflows; remote publication requires actions:on and explicit CI authorization because remote events may activate Actions"]}),
    )
}
fn policy(p: &Budget, tokens: &Value, attempts: u64, active_ms: u64, native: bool) -> Result<()> {
    if p.max_attempts.is_some_and(|n| attempts >= u64::from(n)) {
        bail!("attempt budget exhausted");
    }
    if p.max_active_seconds
        .is_some_and(|n| active_ms >= n.saturating_mul(1000))
    {
        bail!("active time budget exhausted");
    }
    if native {
        if p.max_cost_micros.is_some() {
            bail!("native monetary cost unknown; budget cannot be enforced");
        }
        if let Some(limit) = p.max_tokens {
            if limit == 0 {
                bail!("token budget exhausted before native spawn");
            }
            let calls = tokens["calls"].as_u64().unwrap_or(0);
            if calls > 0 {
                let Some(total) = tokens["total_tokens"].as_u64() else {
                    bail!("token usage unknown; new inference blocked");
                };
                if total >= limit {
                    bail!("token budget exhausted");
                }
            }
        }
    }
    Ok(())
}
pub fn check(c: &Context, store: &Store, run: &Run, task: &Task, active_ms: u64) -> Result<()> {
    if c.manifest.budget.max_active_seconds.is_some()
        && run.duration.as_ref().is_some_and(|d| !d.observed_complete)
    {
        bail!("legacy or interrupted active time unknown");
    }
    let native = !matches!(task.executor, Executor::Command { .. });
    if c.manifest.budget.max_active_seconds.is_some()
        && store.token_usage(&run.release_id, None)?["incomplete_calls"]
            .as_u64()
            .unwrap_or(0)
            > 0
    {
        bail!("prior native active time unknown");
    }
    let external_ms = store
        .metadata(&format!("native-active:{}", run.release_id))?
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    policy(
        &c.manifest.budget,
        &store.token_usage(&run.release_id, None)?,
        store.budget_attempts(&run.release_id, None)?,
        active_ms.saturating_add(external_ms),
        native,
    )?;
    let task_ms = run
        .duration
        .as_ref()
        .map_or(0, |d| d.task_budget_ms(&task.id));
    policy(
        &task.budget,
        &store.token_usage(&run.release_id, Some(&task.id))?,
        store.budget_attempts(&run.release_id, Some(&task.id))?,
        task_ms,
        native,
    )
}
pub fn timeout(c: &Context, run: &Run, task: &Task, requested: u64, active_ms: u64) -> Result<u64> {
    let external_ms = if c.db.exists() {
        Store::open(&c.db, true)?
            .metadata(&format!("native-active:{}", run.release_id))?
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    } else {
        0
    };
    let active_ms = active_ms.saturating_add(external_ms);
    let mut seconds = requested;
    for (p, elapsed) in [
        (&c.manifest.budget, active_ms),
        (
            &task.budget,
            run.duration
                .as_ref()
                .map_or(0, |d| d.task_budget_ms(&task.id)),
        ),
    ] {
        if let Some(max) = p.max_active_seconds {
            let remaining = max.saturating_sub(elapsed.div_ceil(1000));
            if remaining == 0 {
                bail!("active time budget exhausted");
            }
            seconds = seconds.min(remaining);
        }
    }
    Ok(seconds)
}

/// Durable admission record for planning/review/chat calls attached to a release policy.
/// Holding the worktree lease prevents budget races with engine execution.
pub struct Reservation {
    _state_access: forge_domain::StateAccess,
    call: forge_domain::usage::TokenCall,
    db: std::path::PathBuf,
    release: String,
    started: std::time::Instant,
    _locks: forge_domain::WorktreeLocks,
    pub deadline_seconds: Option<u64>,
}
/// Recheck persisted shared counters while the caller holds the worktree lease.
fn admit_native(c: &Context) -> Result<(Store, Option<u64>)> {
    let store = Store::open(&c.db, false)?;
    let saved = store.load(&c.manifest.release_id)?;
    if c.manifest.budget.max_active_seconds.is_some()
        && saved
            .as_ref()
            .and_then(|r| r.duration.as_ref())
            .is_some_and(|d| {
                !d.observed_complete
                    || d.open_interval.is_some()
                    || !d.parallel_intervals.is_empty()
            })
    {
        bail!("prior execution active time unknown");
    }
    let active = saved
        .and_then(|r| r.duration)
        .map_or(0, |d| d.observed_active_ms);
    let prior = store
        .metadata(&format!("native-active:{}", c.manifest.release_id))?
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    policy(
        &c.manifest.budget,
        &store.token_usage(&c.manifest.release_id, None)?,
        store.budget_attempts(&c.manifest.release_id, None)?,
        active.saturating_add(prior),
        true,
    )?;
    let usage = store.token_usage(&c.manifest.release_id, None)?;
    if c.manifest.budget.max_active_seconds.is_some()
        && usage["incomplete_calls"].as_u64().unwrap_or(0) > 0
    {
        bail!("prior native call incomplete; active time unknown");
    }
    let remaining = c
        .manifest
        .budget
        .max_active_seconds
        .map(|n| n.saturating_sub(active.saturating_add(prior).div_ceil(1000)));
    if remaining == Some(0) {
        bail!("active time budget exhausted");
    }
    Ok((store, remaining))
}
impl Reservation {
    pub fn begin(c: &Context, id: &str, phase: &str) -> Result<Self> {
        let locks = forge_domain::WorktreeLocks::acquire(&c.roots)?;
        let (mut store, remaining) = admit_native(c)?;
        let call = forge_domain::usage::TokenCall {
            id: id.into(),
            task_id: format!("_{phase}"),
            attempt: 1,
            phase: phase.into(),
            repo_scope: forge_domain::digest(&serde_json::to_vec(&c.roots)?),
            client: "codex".into(),
            model: None,
            outcome: "running".into(),
            elapsed_ms: None,
            observation: Default::default(),
        };
        store.record_token_call(&c.manifest.release_id, &call)?;
        Ok(Self {
            _state_access: forge_domain::StateAccess::acquire(&c.db, false, false)?,
            call,
            db: c.db.clone(),
            release: c.manifest.release_id.clone(),
            started: std::time::Instant::now(),
            _locks: locks,
            deadline_seconds: remaining,
        })
    }
    /// Another native invocation in the same explicitly requested round, with the
    /// same lease and a fresh admission/usage row. Never hides calls inside a round.
    pub fn advance(&mut self, c: &Context, id: &str) -> Result<()> {
        anyhow::ensure!(
            self.call.outcome != "running",
            "previous native call still open"
        );
        anyhow::ensure!(
            self.release == c.manifest.release_id && self.db == c.db,
            "native reservation scope changed"
        );
        let (mut store, remaining) = admit_native(c)?;
        self.call.id = id.into();
        self.call.outcome = "running".into();
        self.call.elapsed_ms = None;
        self.call.observation = Default::default();
        self.started = std::time::Instant::now();
        self.deadline_seconds = remaining;
        store.record_token_call(&self.release, &self.call)?;
        Ok(())
    }
    pub fn is_running(&self) -> bool {
        self.call.outcome == "running"
    }
    pub fn finish(&mut self, usage: &Value, outcome: &str) -> Result<()> {
        use forge_domain::usage::{TokenUsage, UsageObservation, UsageUnit};
        let measured = TokenUsage::new(
            usage["inputTokens"].as_u64(),
            usage["cachedInputTokens"].as_u64(),
            usage["outputTokens"].as_u64(),
        );
        self.call.outcome = outcome.into();
        self.call.elapsed_ms =
            Some(self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        self.call.observation = UsageObservation {
            complete: outcome != "unknown" && measured.total_tokens.is_some(),
            units: if measured.input_tokens.is_some() || measured.output_tokens.is_some() {
                vec![UsageUnit {
                    key: self.call.id.clone(),
                    session: self.call.id.clone(),
                    cumulative: false,
                    token_usage: measured,
                }]
            } else {
                vec![]
            },
            limitations: vec!["native subscription balance and monetary cost unknown".into()],
        };
        let mut store = Store::open(&self.db, false)?;
        store.record_token_call(&self.release, &self.call)?;
        let key = format!("native-active:{}", self.release);
        let prior = store.metadata(&key)?.and_then(|v| v.as_u64()).unwrap_or(0);
        store.set_metadata(
            &key,
            &json!(prior.saturating_add(self.call.elapsed_ms.unwrap_or(0))),
        )?;
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if self.call.outcome == "running" {
            let _ = self.finish(&Value::Null, "unknown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_unknown_is_not_zero_or_unlimited() {
        let p = Budget {
            max_tokens: Some(10),
            ..Budget::default()
        };
        assert!(policy(&p, &json!({"calls":1,"total_tokens":null}), 0, 0, true).is_err());
        assert!(policy(&p, &json!({"calls":0,"total_tokens":null}), 0, 0, true).is_ok());
        assert!(policy(&p, &json!({"calls":1,"total_tokens":10}), 0, 0, true).is_err());
    }
    #[test]
    fn ci_requires_explicit_budgeted_activation() {
        let p: Budget = serde_json::from_value(json!({"actions":"on"})).unwrap();
        assert!(p.validate().is_err());
        assert!(Budget::default().validate().is_ok());
    }
}
