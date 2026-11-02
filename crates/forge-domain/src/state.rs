use crate::{Manifest, Revision};
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    NotRun,
    Blocked,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: String,
    pub id: String,
    pub outcome: Outcome,
    pub digest: Option<String>,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub status: String,
    pub attempts: u32,
    pub evidence: Vec<Evidence>,
    pub revision: Option<Revision>,
    pub pid: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub schema_version: u32,
    pub release_id: String,
    pub manifest_hash: String,
    pub status: String,
    pub revisions: BTreeMap<String, Revision>,
    pub tasks: BTreeMap<String, TaskState>,
    pub effective_manifest: Manifest,
    pub memory: Value,
    #[serde(default)]
    pub duration: Option<crate::duration::DurationState>,
    #[serde(default)]
    pub last_token_call: Option<crate::usage::TokenCall>,
}
pub struct Store {
    pub(crate) conn: Connection,
    // Cooperative maintenance barrier, independent of SQLite's WAL byte locks.
    // Kept after `conn` so it is released only after the connection closes.
    _access: StateAccess,
}
/// Shared leases cover all Forge database users; maintenance requires exclusive ownership.
pub struct StateAccess(Option<std::fs::File>);
impl StateAccess {
    pub fn acquire(path: &Path, exclusive: bool, create: bool) -> Result<Self> {
        use fs2::FileExt;
        ensure!(
            !path
                .parent()
                .is_some_and(|p| p.join(".forge-restore-incomplete").exists()),
            "incomplete state restore; use a fresh verified restore target"
        );
        // Never flock the database: some SQLite VFS implementations use flock too.
        let lock = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("state parent required"))?
            .join(".forge-state-access.lock");
        if let Ok(m) = std::fs::symlink_metadata(&lock) {
            ensure!(
                m.is_file() && !m.file_type().is_symlink(),
                "invalid state maintenance lock"
            );
        }
        // Legacy read-only readers create nothing. All writers establish the barrier.
        if !create && !exclusive && !lock.exists() {
            return Ok(Self(None));
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(create || exclusive)
            .create(create || exclusive)
            .truncate(false)
            .open(lock)?;
        let result = if exclusive {
            file.try_lock_exclusive()
        } else {
            FileExt::try_lock_shared(&file)
        };
        result.map_err(|_| {
            anyhow::anyhow!("state in use; maintenance requires idle exclusive ownership")
        })?;
        Ok(Self(Some(file)))
    }
}
impl Drop for StateAccess {
    fn drop(&mut self) {
        if let Some(file) = &self.0 {
            let _ = fs2::FileExt::unlock(file);
        }
    }
}
impl Store {
    pub fn open(path: &Path, readonly: bool) -> Result<Self> {
        if !readonly && let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let access = StateAccess::acquire(path, false, !readonly)?;
        let flags = if readonly {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        };
        let conn = Connection::open_with_flags(path, flags)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        ensure!(version == 0 || version == 1, "unsupported state schema");
        if !readonly {
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY AUTOINCREMENT, release_id TEXT NOT NULL, at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), kind TEXT NOT NULL, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS controls (release_id TEXT PRIMARY KEY, action TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS approvals (release_id TEXT NOT NULL, task_id TEXT NOT NULL, gate_id TEXT NOT NULL, manifest_hash TEXT NOT NULL, fingerprint TEXT NOT NULL, reviewer TEXT NOT NULL, PRIMARY KEY(release_id,task_id,gate_id));
            CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, body TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS duration_measurements (release_id TEXT NOT NULL, generation INTEGER NOT NULL, sequence INTEGER NOT NULL, context TEXT NOT NULL, body TEXT NOT NULL, PRIMARY KEY(release_id,generation,sequence));
            CREATE INDEX IF NOT EXISTS duration_context ON duration_measurements(context);
            CREATE TABLE IF NOT EXISTS duration_forecasts (seq INTEGER PRIMARY KEY AUTOINCREMENT, release_id TEXT NOT NULL, generation INTEGER NOT NULL, reason TEXT NOT NULL, body TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS duration_forecast_release ON duration_forecasts(release_id,seq);
            CREATE TABLE IF NOT EXISTS token_calls (release_id TEXT NOT NULL, id TEXT NOT NULL, task_id TEXT NOT NULL, complete INTEGER NOT NULL, has_usage INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY(release_id,id));
            CREATE TABLE IF NOT EXISTS token_units (release_id TEXT NOT NULL, scope TEXT NOT NULL, unit_key TEXT NOT NULL, session TEXT NOT NULL, cumulative INTEGER NOT NULL, call_id TEXT NOT NULL, task_id TEXT NOT NULL, input_tokens INTEGER, cached_input_tokens INTEGER, output_tokens INTEGER, total_tokens INTEGER, PRIMARY KEY(release_id,scope,unit_key));
            CREATE INDEX IF NOT EXISTS token_unit_session ON token_units(release_id,scope,session,cumulative);
            PRAGMA user_version=1;")?;
        }
        ensure!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? == 1,
            "unsupported state schema"
        );
        Ok(Self {
            conn,
            _access: access,
        })
    }
    pub fn load(&self, id: &str) -> Result<Option<Run>> {
        let body: Option<String> = self
            .conn
            .query_row("SELECT body FROM runs WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        body.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn metadata(&self, key: &str) -> Result<Option<Value>> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='metadata')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(None);
        }
        let body: Option<String> = self
            .conn
            .query_row("SELECT body FROM metadata WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        body.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn set_metadata(&self, key: &str, body: &Value) -> Result<()> {
        self.conn.execute(
            "INSERT INTO metadata VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET body=excluded.body",
            params![key, body.to_string()],
        )?;
        Ok(())
    }
    /// Persist an operational transition and its audit event in the same transaction.
    pub fn journal_metadata(&mut self, release: &str, key: &str, body: &Value) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO metadata VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET body=excluded.body",
            params![key, body.to_string()],
        )?;
        tx.execute(
            "INSERT INTO events(release_id,kind,body) VALUES(?1,?2,?3)",
            params![release, key, body.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn remove_metadata(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM metadata WHERE key=?1", [key])?;
        Ok(())
    }
    pub fn save(&mut self, run: &Run, kind: &str, detail: &Value) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO runs(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![run.release_id,serde_json::to_string(run)?])?;
        tx.execute(
            "INSERT INTO events(release_id,kind,body) VALUES(?1,?2,?3)",
            params![run.release_id, kind, detail.to_string()],
        )?;
        crate::usage_store::backfill_calls(&tx, run)?;
        if let Some(call) = &run.last_token_call {
            crate::usage_store::save_call(&tx, &run.release_id, call)?;
        }
        if let Some(d) = &run.duration {
            for m in [d.last_measurement.as_ref(), d.open_interval.as_ref()]
                .into_iter()
                .flatten()
                .chain(d.parallel_intervals.values())
            {
                tx.execute("INSERT INTO duration_measurements VALUES(?1,?2,?3,?4,?5) ON CONFLICT(release_id,generation,sequence) DO UPDATE SET body=excluded.body WHERE json_extract(duration_measurements.body,'$.elapsed_ms') IS NULL",
                    params![run.release_id,i64::try_from(d.generation)?,i64::try_from(m.sequence)?,m.context,serde_json::to_string(m)?])?;
            }
            tx.execute("INSERT INTO duration_forecasts(release_id,generation,reason,body) VALUES(?1,?2,?3,?4)",params![run.release_id,i64::try_from(d.generation)?,kind,serde_json::to_string(&d.current_estimate)?])?;
        }
        tx.commit()?;
        Ok(())
    }
    /// At most 200 small records; legacy and malformed histories remain distinguishable.
    pub fn duration_history(&self) -> (Vec<crate::duration::Measurement>, Option<String>) {
        let read = || -> Result<Vec<crate::duration::Measurement>> {
            let mut stmt = self.conn.prepare("SELECT CASE WHEN length(CAST(body AS BLOB))<=65536 THEN body ELSE 'oversized' END FROM duration_measurements ORDER BY rowid DESC LIMIT 200")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.map(|r| {
                let m: crate::duration::Measurement = serde_json::from_str(&r?)?;
                ensure!(m.valid(), "invalid duration measurement");
                Ok(m)
            })
            .collect()
        };
        match read() {
            Ok(v) => (v, None),
            Err(_) => (
                vec![],
                Some("duration history missing, corrupt or incompatible".into()),
            ),
        }
    }
    pub fn duration_forecasts(&self, id: &str) -> Result<Vec<Value>> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='duration_forecasts')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(vec![]);
        }
        let mut stmt = self.conn.prepare("SELECT generation,reason,CASE WHEN length(CAST(body AS BLOB))<=65536 THEN body ELSE 'oversized' END FROM duration_forecasts WHERE release_id=?1 ORDER BY seq DESC LIMIT 100")?;
        let rows = stmt.query_map([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        rows.map(|r| { let (generation,reason,body)=r?; Ok(serde_json::json!({"generation":generation,"reason":reason,"estimate":serde_json::from_str::<Value>(&body)?})) }).collect()
    }
    pub fn control(&self, id: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT action FROM controls WHERE release_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn set_control(&mut self, id: &str, action: &str) -> Result<()> {
        ensure!(
            self.load(id)?.is_some()
                || self
                    .metadata(&format!("independent-review:{id}"))?
                    .is_some_and(|v| v["status"] == "running"),
            "unknown release or active review"
        );
        ensure!(
            ["pause", "resume", "cancel"].contains(&action),
            "invalid control action"
        );
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO controls VALUES(?1,?2) ON CONFLICT(release_id) DO UPDATE SET action=excluded.action",params![id,action])?;
        tx.execute(
            "INSERT INTO events(release_id,kind,body) VALUES(?1,'control',?2)",
            params![id, serde_json::json!({"action":action}).to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn approve(
        &mut self,
        run: &Run,
        task: &str,
        gate: &str,
        reviewer: &str,
        fingerprint: &str,
    ) -> Result<()> {
        ensure!(!reviewer.trim().is_empty(), "reviewer is required");
        let t = run
            .tasks
            .get(task)
            .ok_or_else(|| anyhow::anyhow!("unknown task"))?;
        ensure!(
            t.status == "awaiting_approval",
            "task has not reached human review"
        );
        let r = t
            .revision
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no evaluated revision"))?;
        ensure!(
            r.fingerprint == fingerprint,
            "approval does not match evaluated revision"
        );
        let contract = run
            .effective_manifest
            .tasks
            .iter()
            .find(|t| t.id == task)
            .unwrap();
        ensure!(
            contract
                .acceptance
                .iter()
                .any(|a| matches!(a,crate::Acceptance::Human{id} if id==gate)),
            "unknown human gate"
        );
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO approvals VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                run.release_id,
                task,
                gate,
                run.manifest_hash,
                fingerprint,
                reviewer
            ],
        )?;
        tx.execute("INSERT INTO events(release_id,kind,body) VALUES(?1,'approval',?2)",params![run.release_id,serde_json::json!({"task":task,"gate":gate,"reviewer":reviewer,"fingerprint":fingerprint}).to_string()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn deny(
        &mut self,
        run: &Run,
        task: &str,
        gate: &str,
        reviewer: &str,
        fingerprint: &str,
    ) -> Result<()> {
        ensure!(!reviewer.trim().is_empty(), "reviewer required");
        let t = run
            .tasks
            .get(task)
            .ok_or_else(|| anyhow::anyhow!("unknown task"))?;
        ensure!(
            t.status == "awaiting_approval"
                && t.revision
                    .as_ref()
                    .is_some_and(|r| r.fingerprint == fingerprint),
            "review revision/status mismatch"
        );
        ensure!(
            run.effective_manifest
                .tasks
                .iter()
                .find(|t| t.id == task)
                .is_some_and(|t| t
                    .acceptance
                    .iter()
                    .any(|a| matches!(a,crate::Acceptance::Human{id} if id==gate))),
            "unknown human gate"
        );
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM approvals WHERE release_id=?1 AND task_id=?2 AND gate_id=?3",
            params![run.release_id, task, gate],
        )?;
        tx.execute("INSERT INTO events(release_id,kind,body) VALUES(?1,'denial',?2)",params![run.release_id,serde_json::json!({"task":task,"gate":gate,"reviewer":reviewer,"fingerprint":fingerprint}).to_string()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn approved(&self, run: &Run, task: &str, gate: &str, fingerprint: &str) -> Result<bool> {
        Ok(self.conn.query_row("SELECT 1 FROM approvals WHERE release_id=?1 AND task_id=?2 AND gate_id=?3 AND manifest_hash=?4 AND fingerprint=?5",params![run.release_id,task,gate,run.manifest_hash,fingerprint],|r|r.get::<_,u32>(0)).optional()?.is_some())
    }
    pub fn clear_approvals(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM approvals WHERE release_id=?1", [id])?;
        Ok(())
    }
    pub fn events(&self, id: &str) -> Result<Vec<Value>> {
        self.event_rows(id, false)
    }
    /// A bounded report projection must never truncate the journal used for recovery.
    pub fn recent_events(&self, id: &str) -> Result<Vec<Value>> {
        self.event_rows(id, true)
    }
    /// Stable bounded page of the append-only recovery journal (newest first).
    pub fn events_page(&self, id: &str, limit: u32, offset: u64) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare("SELECT seq,at,kind,body FROM events WHERE release_id=?1 ORDER BY seq DESC LIMIT ?2 OFFSET ?3")?;
        let rows = stmt.query_map(
            params![id, limit.clamp(1, 1000), i64::try_from(offset)?],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )?;
        rows.map(|r| {let (seq,at,kind,body)=r?;Ok(serde_json::json!({"seq":seq,"at":at,"kind":kind,"body":serde_json::from_str::<Value>(&body)?}))}).collect()
    }
    fn event_rows(&self, id: &str, recent: bool) -> Result<Vec<Value>> {
        let query = if recent {
            "SELECT seq,at,kind,body FROM (SELECT seq,at,kind,body FROM events WHERE release_id=?1 ORDER BY seq DESC LIMIT 1000) ORDER BY seq"
        } else {
            "SELECT seq,at,kind,body FROM events WHERE release_id=?1 ORDER BY seq"
        };
        let mut stmt = self.conn.prepare(query)?;
        let rows = stmt.query_map([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        rows.map(|r|{let(s,a,k,b)=r?;Ok(serde_json::json!({"seq":s,"at":a,"kind":k,"body":serde_json::from_str::<Value>(&b)?}))}).collect()
    }
}
impl Run {
    pub fn new(manifest: Manifest, hash: String, revisions: BTreeMap<String, Revision>) -> Self {
        Self {
            schema_version: 1,
            release_id: manifest.release_id.clone(),
            manifest_hash: hash,
            status: "not_run".into(),
            revisions,
            tasks: manifest
                .tasks
                .iter()
                .map(|t| {
                    (
                        t.id.clone(),
                        TaskState {
                            status: "not_run".into(),
                            attempts: 0,
                            evidence: vec![],
                            revision: None,
                            pid: None,
                        },
                    )
                })
                .collect(),
            effective_manifest: manifest,
            memory: Value::Null,
            duration: None,
            last_token_call: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_does_not_create_database() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("absent/db");
        assert!(Store::open(&p, true).is_err());
        assert!(!p.parent().unwrap().exists());
    }
    #[test]
    fn report_window_preserves_the_complete_recovery_journal() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.sqlite"), false).unwrap();
        for i in 0..1005 {
            store
                .conn
                .execute(
                    "INSERT INTO events(release_id,kind,body) VALUES('release','journal',?1)",
                    [serde_json::json!({"number": i}).to_string()],
                )
                .unwrap();
        }
        let full = store.events("release").unwrap();
        let recent = store.recent_events("release").unwrap();
        assert_eq!(full.len(), 1005);
        assert_eq!(full[0]["body"]["number"], 0);
        assert_eq!(recent.len(), 1000);
        assert_eq!(recent[0]["body"]["number"], 5);
        assert_eq!(recent.last().unwrap()["body"]["number"], 1004);
    }
    #[test]
    fn rejects_unknown_controls() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(&d.path().join("state"), false).unwrap();
        assert!(s.set_control("missing", "cancel").is_err());
    }
}
