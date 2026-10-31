//! Durable usage journal and read-only projections; report windows do not limit accounting.
use crate::{Executor, Run, Store, usage::*};
use anyhow::Result;
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::{Value, json};

/// Old attempts are known to exist, but their token consumption is not known.
/// Insert placeholders once so upgrading/resuming cannot turn partial history into zero.
pub(crate) fn backfill_calls(tx: &Transaction<'_>, run: &Run) -> Result<()> {
    let generation = run.duration.as_ref().map_or(0, |d| d.generation);
    for task in &run.effective_manifest.tasks {
        let (client, model) = match &task.executor {
            Executor::Codex { model, .. } => ("codex", model.clone()),
            Executor::Claude { model, .. } => ("claude", model.clone()),
            Executor::Command { .. } => continue,
        };
        let Some(state) = run.tasks.get(&task.id) else {
            continue;
        };
        for attempt in 1..=state.attempts.min(3) {
            let call = TokenCall {
                id: format!("{generation}:{}:{attempt}:execution", task.id),
                task_id: task.id.clone(),
                attempt,
                phase: "execution".into(),
                repo_scope: "unobserved_legacy_scope".into(),
                client: client.into(),
                model: model.clone(),
                outcome: "unobserved_legacy_attempt".into(),
                elapsed_ms: None,
                observation: UsageObservation {
                    units: vec![],
                    complete: false,
                    limitations: vec!["legacy attempt has no token measurement".into()],
                },
            };
            tx.execute(
                "INSERT OR IGNORE INTO token_calls VALUES(?1,?2,?3,0,0,?4)",
                params![
                    run.release_id,
                    call.id,
                    call.task_id,
                    serde_json::to_string(&call)?
                ],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn save_call(tx: &Transaction<'_>, release: &str, call: &TokenCall) -> Result<()> {
    let mut saved = call.clone();
    let scope = crate::digest(serde_json::to_string(&(&call.repo_scope, &call.client))?.as_bytes());
    for unit in &call.observation.units {
        let previous: Option<[Option<i64>;3]> = tx.query_row(
            "SELECT input_tokens,cached_input_tokens,output_tokens FROM token_units WHERE release_id=?1 AND scope=?2 AND unit_key=?3",
            params![release,scope,unit.key], |r| Ok([r.get(0)?,r.get(1)?,r.get(2)?])).optional()?;
        let mut values = [
            unit.token_usage.input_tokens,
            unit.token_usage.cached_input_tokens,
            unit.token_usage.output_tokens,
        ];
        if let Some(previous) = previous {
            let old = previous.map(|v| v.and_then(|v| u64::try_from(v).ok()));
            let conflict = old.iter().zip(values).any(|(o, n)| {
                o.zip(n)
                    .is_some_and(|(o, n)| if unit.cumulative { n < o } else { n != o })
            });
            if conflict {
                saved.observation.complete = false;
                if !saved.observation.limitations.iter().any(|l| {
                    l == "conflicting persisted usage snapshot; earlier observation retained"
                }) {
                    saved.observation.limitations.push(
                        "conflicting persisted usage snapshot; earlier observation retained".into(),
                    );
                }
                continue;
            }
            for (v, o) in values.iter_mut().zip(old) {
                *v = v.or(o);
            }
        }
        let u = TokenUsage::new(values[0], values[1], values[2]);
        // Values outside SQLite's exact integer range remain unknown, never float estimates.
        let db_count = |n: Option<u64>| n.and_then(|n| i64::try_from(n).ok());
        tx.execute("INSERT INTO token_units VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
            ON CONFLICT(release_id,scope,unit_key) DO UPDATE SET input_tokens=excluded.input_tokens,cached_input_tokens=excluded.cached_input_tokens,output_tokens=excluded.output_tokens,total_tokens=excluded.total_tokens",
            params![release,scope,unit.key,unit.session,unit.cumulative,call.id,call.task_id,
                db_count(u.input_tokens),db_count(u.cached_input_tokens),db_count(u.output_tokens),db_count(u.total_tokens)])?;
    }
    tx.execute("INSERT INTO token_calls VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(release_id,id) DO UPDATE SET complete=excluded.complete,has_usage=excluded.has_usage,body=excluded.body",
        params![release, saved.id, saved.task_id, saved.observation.complete, !saved.observation.units.is_empty(), serde_json::to_string(&saved)?])?;
    Ok(())
}
impl Store {
    /// Preserve legacy unknown attempts before replacing a run during reconciliation.
    pub fn preserve_token_calls(&mut self, run: &Run) -> Result<()> {
        let tx = self.conn.transaction()?;
        backfill_calls(&tx, run)?;
        if let Some(call) = &run.last_token_call {
            save_call(&tx, &run.release_id, call)?;
        }
        tx.commit()?;
        Ok(())
    }
    /// All calls in the release, including previous reconciled generations and failed attempts.
    /// Individual fields stay null if any invocation has incomplete or missing observations.
    pub fn token_usage(&self, release: &str, task: Option<&str>) -> Result<Value> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='token_calls')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(unavailable("legacy state has no token journal"));
        }
        let (calls, incomplete, unknown): (i64,i64,i64) = self.conn.query_row(
            "SELECT count(*),coalesce(sum(complete=0),0),coalesce(sum(has_usage=0),0) FROM token_calls WHERE release_id=?1 AND (?2 IS NULL OR task_id=?2)", params![release,task], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        // SQL performs deduplication and cumulative supersession across the full journal.
        // Sum checked integers in constant memory; overflow stays unknown.
        let mut stmt = self.conn.prepare("SELECT input_tokens,cached_input_tokens,output_tokens,total_tokens FROM token_units u WHERE release_id=?1 AND (?2 IS NULL OR task_id=?2) AND (cumulative=1 OR NOT EXISTS (SELECT 1 FROM token_units s WHERE s.release_id=u.release_id AND s.scope=u.scope AND s.session=u.session AND s.cumulative=1))")?;
        let rows = stmt.query_map(params![release, task], |r| {
            Ok([r.get::<_, Option<i64>>(0)?, r.get(1)?, r.get(2)?, r.get(3)?])
        })?;
        let mut sums = [Some(0u64); 4];
        let mut known = [true; 4];
        let mut has_observation = [false; 4];
        let mut count = 0u64;
        for row in rows {
            count += 1;
            for (i, n) in row?.into_iter().enumerate() {
                let n = n.and_then(|v| u64::try_from(v).ok());
                known[i] &= n.is_some();
                has_observation[i] |= n.is_some();
                if let Some(n) = n {
                    sums[i] = sums[i].and_then(|s| s.checked_add(n));
                }
            }
        }
        let observed: [Option<u64>; 4] =
            std::array::from_fn(|i| if has_observation[i] { sums[i] } else { None });
        let fields: [Option<u64>; 4] = std::array::from_fn(|i| {
            if calls > 0 && incomplete == 0 && count > 0 && known[i] {
                sums[i]
            } else {
                None
            }
        });
        let usage = TokenUsage {
            input_tokens: fields[0],
            cached_input_tokens: fields[1],
            output_tokens: fields[2],
            total_tokens: fields[3],
        };
        let observed = TokenUsage {
            input_tokens: observed[0],
            cached_input_tokens: observed[1],
            output_tokens: observed[2],
            total_tokens: observed[3],
        };
        let mut value = serde_json::to_value(usage)?;
        let coverage = if count == 0 {
            "unavailable"
        } else if incomplete == 0 && fields.iter().all(Option::is_some) {
            "complete"
        } else {
            "partial"
        };
        value["coverage"] = json!(coverage);
        value["observed"] = json!(observed);
        value["calls"] = json!(calls);
        value["incomplete_calls"] = json!(incomplete);
        value["unknown_calls"] = json!(unknown);
        value["units"] = json!(count);
        value["basis"] =
            json!("native_reported; cache included in input; no price or subscription conversion");
        value["scope"] = json!(
            "all journaled calls for this release, including retries, failures, cancellations and reconciled generations; running/orphaned calls are incomplete"
        );
        Ok(value)
    }
    /// Journal a non-engine native invocation using the same accounting and deduplication.
    pub fn record_token_call(&mut self, release: &str, call: &TokenCall) -> Result<()> {
        let tx = self.conn.transaction()?;
        save_call(&tx, release, call)?;
        tx.commit()?;
        Ok(())
    }
    pub fn budget_attempts(&self, release: &str, task: Option<&str>) -> Result<u64> {
        let executions: i64=self.conn.query_row("SELECT count(*) FROM events WHERE release_id=?1 AND kind='task_started' AND (?2 IS NULL OR json_extract(body,'$.task')=?2)",params![release,task],|r|r.get(0))?;
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='token_calls')",
            [],
            |r| r.get(0),
        )?;
        let other: i64 = if exists {
            self.conn.query_row("SELECT count(*) FROM token_calls WHERE release_id=?1 AND json_extract(body,'$.phase')!='execution' AND (?2 IS NULL OR task_id=?2)",params![release,task],|r|r.get(0))?
        } else {
            0
        };
        Ok((executions as u64).saturating_add(other as u64))
    }
    pub fn token_calls(&self, release: &str, limit: u32, offset: u64) -> Result<Vec<Value>> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='token_calls')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(vec![]);
        }
        let mut stmt = self.conn.prepare("SELECT body FROM token_calls WHERE release_id=?1 ORDER BY rowid DESC LIMIT ?2 OFFSET ?3")?;
        let rows = stmt.query_map(
            params![release, limit.clamp(1, 1000), i64::try_from(offset)?],
            |r| r.get::<_, String>(0),
        )?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
}
pub fn unavailable(reason: &str) -> Value {
    let mut v = serde_json::to_value(TokenUsage::default()).expect("serializable usage");
    v["coverage"] = json!("unavailable");
    v["reason"] = json!(reason);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Manifest, Run};
    use std::collections::BTreeMap;
    fn run() -> Run {
        let m: Manifest=serde_json::from_value(json!({"schema_version":1,"release_id":"r","repos":{"app":{"path":"."}},"tasks":[{"id":"t","repo":"app","description":"test","executor":{"kind":"command","argv":["true"]},"acceptance":[{"kind":"human","id":"review"}]}]})).unwrap();
        Run::new(m, "manifest".into(), BTreeMap::new())
    }
    fn call(id: &str, phase: &str, outcome: &str, session: &str, turn: &str, n: u64) -> TokenCall {
        TokenCall {
            id: id.into(),
            task_id: "t".into(),
            attempt: 1,
            phase: phase.into(),
            repo_scope: "repo-a".into(),
            client: "codex".into(),
            model: None,
            outcome: outcome.into(),
            elapsed_ms: Some(10),
            observation: UsageObservation {
                complete: true,
                limitations: vec![],
                units: vec![UsageUnit {
                    key: format!("{session}:{turn}"),
                    session: session.into(),
                    cumulative: false,
                    token_usage: TokenUsage::new(Some(n), Some(2), Some(3)),
                }],
            },
        }
    }
    #[test]
    fn economy_counts_failed_and_planning_calls() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("state.sqlite");
        let mut store = Store::open(&db, false).unwrap();
        let mut r = run();
        for (id, phase, outcome, n) in [
            ("1", "planning", "completed", 10),
            ("2", "execution", "failed", 20),
            ("3", "compaction", "cancelled", 30),
        ] {
            r.last_token_call = Some(call(id, phase, outcome, id, "turn", n));
            store.save(&r, "call_finished", &json!({"id":id})).unwrap();
        }
        drop(store);
        let store = Store::open(&db, true).unwrap();
        let usage = store.token_usage("r", None).unwrap();
        assert_eq!(usage["input_tokens"], 60);
        assert_eq!(usage["cached_input_tokens"], 6);
        assert_eq!(usage["total_tokens"], 69);
        assert_eq!(usage["calls"], 3);
        assert_eq!(
            store.token_calls("r", 1, 0).unwrap()[0]["outcome"],
            "cancelled"
        );
        assert_eq!(
            store.token_calls("r", 1, 2).unwrap()[0]["phase"],
            "planning"
        );
        assert_eq!(store.events("r").unwrap().len(), 3);
    }
    #[test]
    fn durable_dedup_scope_and_missing_calls_are_not_zero() {
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(&d.path().join("db"), false).unwrap();
        let mut r = run();
        r.last_token_call = Some(call("1", "execution", "failed", "s", "t", 10));
        store.save(&r, "finished", &json!({})).unwrap();
        store.save(&r, "stopped", &json!({})).unwrap();
        r.last_token_call.as_mut().unwrap().id = "2".into();
        store.save(&r, "finished", &json!({})).unwrap();
        assert_eq!(store.token_usage("r", None).unwrap()["total_tokens"], 13);
        r.last_token_call.as_mut().unwrap().id = "3".into();
        r.last_token_call.as_mut().unwrap().repo_scope = "different-repo".into();
        store.save(&r, "finished", &json!({})).unwrap();
        assert_eq!(store.token_usage("r", None).unwrap()["total_tokens"], 26);
        r.last_token_call.as_mut().unwrap().id = "4".into();
        r.last_token_call.as_mut().unwrap().observation = Default::default();
        store.save(&r, "call_started", &json!({})).unwrap();
        let usage = store.token_usage("r", None).unwrap();
        assert!(usage["total_tokens"].is_null());
        assert_eq!(usage["observed"]["total_tokens"], 26);
        assert_eq!(usage["unknown_calls"], 1);
        assert_eq!(usage["coverage"], "partial");
        assert!(store.token_usage("another-release", None).unwrap()["total_tokens"].is_null());
    }
    #[test]
    fn cumulative_session_snapshot_replaces_turns_and_refuses_regression() {
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(&d.path().join("db"), false).unwrap();
        let mut r = run();
        r.last_token_call = Some(call("1", "execution", "completed", "s", "turn", 10));
        store.save(&r, "finished", &json!({})).unwrap();
        let mut c = call("2", "execution", "completed", "s", "session", 20);
        c.observation.units[0].cumulative = true;
        r.last_token_call = Some(c.clone());
        store.save(&r, "finished", &json!({})).unwrap();
        assert_eq!(store.token_usage("r", None).unwrap()["total_tokens"], 23);
        c.id = "3".into();
        c.observation.units[0].token_usage = TokenUsage::new(Some(5), Some(0), Some(1));
        r.last_token_call = Some(c);
        store.save(&r, "finished", &json!({})).unwrap();
        let usage = store.token_usage("r", None).unwrap();
        assert!(usage["total_tokens"].is_null());
        assert_eq!(usage["observed"]["total_tokens"], 23);
        assert_eq!(
            store.token_calls("r", 1, 0).unwrap()[0]["observation"]["complete"],
            false
        );
    }
    #[test]
    fn legacy_inspection_does_not_migrate_or_fabricate_usage() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("db");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "PRAGMA user_version=1; CREATE TABLE runs(id TEXT PRIMARY KEY,body TEXT NOT NULL);",
        )
        .unwrap();
        let mut legacy = serde_json::to_value(run()).unwrap();
        legacy.as_object_mut().unwrap().remove("last_token_call");
        conn.execute("INSERT INTO runs VALUES('r',?1)", [legacy.to_string()])
            .unwrap();
        drop(conn);
        let store = Store::open(&db, true).unwrap();
        assert!(store.load("r").unwrap().unwrap().last_token_call.is_none());
        let usage = store.token_usage("r", None).unwrap();
        assert!(usage["total_tokens"].is_null());
        assert_eq!(usage["coverage"], "unavailable");
        let exists: bool = store
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='token_calls')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!exists);
    }
    #[test]
    fn partial_fields_and_legacy_attempts_stay_unknown_after_upgrade() {
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(&d.path().join("db"), false).unwrap();
        let mut r = run();
        r.effective_manifest.tasks[0].executor = Executor::Codex {
            permission: crate::Permission::ReadOnly,
            model: None,
        };
        r.tasks.get_mut("t").unwrap().attempts = 1;
        store.save(&r, "legacy_resume", &json!({})).unwrap();
        let mut c = call("0:t:2:execution", "execution", "completed", "s", "turn", 10);
        c.observation.units[0].token_usage = TokenUsage::new(Some(10), None, None);
        r.last_token_call = Some(c);
        r.tasks.get_mut("t").unwrap().attempts = 2;
        store.save(&r, "finished", &json!({})).unwrap();
        let usage = store.token_usage("r", None).unwrap();
        assert_eq!(usage["calls"], 2);
        assert_eq!(usage["unknown_calls"], 1);
        assert!(usage["total_tokens"].is_null());
        assert_eq!(usage["observed"]["input_tokens"], 10);
        assert!(usage["observed"]["output_tokens"].is_null());
        assert!(usage["observed"]["cached_input_tokens"].is_null());
        assert!(usage["observed"]["total_tokens"].is_null());
    }
}
