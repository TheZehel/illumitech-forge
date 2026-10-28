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
}
pub struct Store {
    conn: Connection,
}
impl Store {
    pub fn open(path: &Path, readonly: bool) -> Result<Self> {
        if !readonly && let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
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
            PRAGMA user_version=1;")?;
        }
        ensure!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? == 1,
            "unsupported state schema"
        );
        Ok(Self { conn })
    }
    pub fn load(&self, id: &str) -> Result<Option<Run>> {
        let body: Option<String> = self
            .conn
            .query_row("SELECT body FROM runs WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        body.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn save(&mut self, run: &Run, kind: &str, detail: &Value) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO runs(id,body) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![run.release_id,serde_json::to_string(run)?])?;
        tx.execute(
            "INSERT INTO events(release_id,kind,body) VALUES(?1,?2,?3)",
            params![run.release_id, kind, detail.to_string()],
        )?;
        tx.commit()?;
        Ok(())
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
        ensure!(self.load(id)?.is_some(), "unknown release");
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
    pub fn approved(&self, run: &Run, task: &str, gate: &str, fingerprint: &str) -> Result<bool> {
        Ok(self.conn.query_row("SELECT 1 FROM approvals WHERE release_id=?1 AND task_id=?2 AND gate_id=?3 AND manifest_hash=?4 AND fingerprint=?5",params![run.release_id,task,gate,run.manifest_hash,fingerprint],|r|r.get::<_,u32>(0)).optional()?.is_some())
    }
    pub fn clear_approvals(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM approvals WHERE release_id=?1", [id])?;
        Ok(())
    }
    pub fn events(&self, id: &str) -> Result<Vec<Value>> {
        let mut stmt = self
            .conn
            .prepare("SELECT seq,at,kind,body FROM events WHERE release_id=?1 ORDER BY seq")?;
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
    fn rejects_unknown_controls() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(&d.path().join("state"), false).unwrap();
        assert!(s.set_control("missing", "cancel").is_err());
    }
}
