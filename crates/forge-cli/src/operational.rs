//! Project-scoped UI operations delegate all execution/evidence authority to the engine.
use crate::{
    engine, guided,
    workbench::{App, sanitize},
};
use anyhow::{Context as _, Result, ensure};
use forge_domain::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgeAction {
    pub operation_id: String,
    pub revision: Option<String>,
    pub task: Option<String>,
    pub gate: Option<String>,
    pub reviewer: Option<String>,
    pub fingerprint: Option<String>,
    pub reconcile: Option<String>,
}
impl App {
    pub(crate) fn inspection_context(&self) -> Result<engine::Context> {
        let path = self
            .manifest
            .as_deref()
            .context("no explicitly supplied manifest")?;
        guided::external(path, &self.root)?;
        let c = engine::Context::load(path, &self.state)?;
        ensure!(
            c.roots.values().all(|r| self.allowed_roots.contains(r))
                && c.roots.values().any(|r| r == &self.root)
                && Some(&c.manifest.release_id) == self.release_id.as_ref(),
            "manifest project/release scope changed"
        );
        Ok(c)
    }
    pub(crate) fn forge_context(&self) -> Result<engine::Context> {
        let c = self.inspection_context()?;
        ensure!(
            !c.manifest.automation.enabled && c.manifest.automation.publish.is_none(),
            "UI execution does not authorize automation/publication"
        );
        Ok(c)
    }
    pub(crate) fn control_revision(&self, c: &engine::Context) -> Result<String> {
        Ok(digest(&serde_json::to_vec(&(
            &c.hash,
            revisions(&c.roots)?,
        ))?))
    }
    pub(crate) fn review(&self, c: &engine::Context) -> Result<Value> {
        let status = engine::status(c, false)?;
        let diff = git(
            &self.root,
            &["diff", "HEAD", "--no-ext-diff", "--no-textconv"],
        )?;
        let untracked = git(&self.root, &["ls-files", "--others", "--exclude-standard"])?;
        let scopes = c.roots.iter().map(|(id,root)| Ok(json!({"repo":id,"root":root,
            "diff":forge_adapters::process::redact(&git(root,&["diff","HEAD","--no-ext-diff","--no-textconv"])?).chars().take(32768).collect::<String>(),
            "untracked_files":git(root,&["ls-files","--others","--exclude-standard"])?}))).collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"scopes":scopes,"control_revision":self.control_revision(c)?,"revisions":revisions(&c.roots)?,"diff":forge_adapters::process::redact(&diff),"untracked_files":untracked,"untracked_review":"fingerprint includes file contents; inspect untracked files locally before approval","tasks":status["run"]["tasks"],"evidence_current":status["evidence_current"],"human_approval":"explicit task/gate/reviewer/evaluated fingerprint required; denial grants nothing"}),
        )
    }
    fn journal(&self, id: &str, route: &str, status: &str, detail: Value) -> Result<()> {
        let db =
            rusqlite::Connection::open(guided::safe_path(&self.state.join("workbench.sqlite"))?)?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS control_journal(seq INTEGER PRIMARY KEY AUTOINCREMENT, root TEXT NOT NULL, operation_id TEXT NOT NULL, route TEXT NOT NULL, status TEXT NOT NULL, detail TEXT NOT NULL, at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));")?;
        db.execute("INSERT INTO control_journal(root,operation_id,route,status,detail) VALUES(?1,?2,?3,?4,?5)",rusqlite::params![self.root.to_string_lossy(),id,route,status,sanitize(detail).to_string()])?;
        Ok(())
    }
    pub(crate) fn control_journal(&self) -> Result<Value> {
        let db = rusqlite::Connection::open_with_flags(
            self.state.join("workbench.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='control_journal')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(json!([]));
        }
        let mut stmt=db.prepare("SELECT operation_id,route,status,detail,at FROM control_journal WHERE root=?1 ORDER BY seq DESC LIMIT 100")?;
        let rows=stmt.query_map([self.root.to_string_lossy()],|r|Ok(json!({"operation_id":r.get::<_,String>(0)?,"route":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"detail":r.get::<_,String>(3)?,"at":r.get::<_,String>(4)?})))?;
        Ok(json!(rows.collect::<std::result::Result<Vec<_>, _>>()?))
    }
    pub async fn forge_action(self: &Arc<Self>, route: &str, a: ForgeAction) -> Result<Value> {
        ensure!(
            [
                "forge-start",
                "forge-pause",
                "forge-cancel",
                "forge-resume",
                "forge-approve",
                "forge-deny",
                "forge-review-plan",
                "forge-review-run"
            ]
            .contains(&route),
            "unknown control"
        );
        ensure!(
            route == "forge-approve"
                || route == "forge-deny"
                || (a.task.is_none()
                    && a.gate.is_none()
                    && a.reviewer.is_none()
                    && a.fingerprint.is_none()),
            "approval fields are invalid for this control"
        );
        ensure!(
            route == "forge-resume" || a.reconcile.is_none(),
            "reconciliation only valid on resume"
        );
        let _guard = self.forge_control.lock().await;
        self.reserve(&a.operation_id)?;
        self.journal(
            &a.operation_id,
            route,
            "reserved",
            json!({"revision":a.revision,"task":a.task,"gate":a.gate,"fingerprint":a.fingerprint}),
        )?;
        let result = self.forge_action_inner(route, &a).await;
        self.journal(
            &a.operation_id,
            route,
            if result.is_ok() {
                "accepted"
            } else {
                "rejected"
            },
            match &result {
                Ok(v) => v.clone(),
                Err(e) => json!({"error":e.to_string()}),
            },
        )?;
        result
    }
    async fn forge_action_inner(self: &Arc<Self>, route: &str, a: &ForgeAction) -> Result<Value> {
        if route == "forge-pause" || route == "forge-cancel" {
            // Content may be changing in an authorized writer. Use pinned startup scope,
            // not revision/evaluator preconditions, for emergency controls.
            let id = self
                .release_id
                .as_deref()
                .context("manifest scope required")?;
            let mut store = Store::open(&self.state.join("state.sqlite"), false)?;
            store.set_control(
                id,
                if route == "forge-pause" {
                    "pause"
                } else {
                    "cancel"
                },
            )?;
            return Ok(
                json!({"requested":route,"pause_semantics":"parallel pause/cancel terminates all owned active process groups; serial pause remains cooperative; interrupted tasks require explicit reconciliation"}),
            );
        }
        {
            let v = self.view.lock().await;
            ensure!(
                v.connected && v.account["authenticated"] == true,
                "authenticated connected native session required"
            );
        }
        let c = self.forge_context()?;
        ensure!(
            a.revision.as_deref() == Some(self.control_revision(&c)?.as_str()),
            "revision changed; refresh diff and fingerprint"
        );
        if route == "forge-approve" || route == "forge-deny" {
            let task = a.task.as_deref().context("task required")?;
            let gate = a.gate.as_deref().context("gate required")?;
            let reviewer = a.reviewer.as_deref().context("reviewer required")?;
            let fingerprint = a
                .fingerprint
                .as_deref()
                .context("evaluated fingerprint required")?;
            ensure!(
                reviewer.len() <= 200 && !reviewer.trim().is_empty(),
                "reviewer required and bounded"
            );
            if route == "forge-deny" {
                let _locks = WorktreeLocks::acquire(&c.roots)?;
                let mut store = Store::open(&c.db, false)?;
                let run = store
                    .load(&c.manifest.release_id)?
                    .context("release has not run")?;
                ensure!(
                    run.manifest_hash == c.hash && run.revisions == revisions(&c.roots)?,
                    "reviewed revision changed"
                );
                store.deny(&run, task, gate, reviewer, fingerprint)?;
                return Ok(
                    json!({"approved":false,"task":task,"gate":gate,"fingerprint":fingerprint,"reviewer":reviewer,"execution":"unchanged"}),
                );
            }
            return engine::approve(&c, task, gate, reviewer, fingerprint);
        }
        if route == "forge-review-plan" {
            return crate::review::plan(&c);
        }
        let mut running = self.forge_run.lock().await;
        ensure!(
            running.as_ref().is_none_or(|h| h.is_finished()),
            "release operation already running"
        );
        let independent_review = route == "forge-review-run";
        let resume = route == "forge-resume";
        if !independent_review && c.db.exists() {
            let saved = Store::open(&c.db, true)?.load(&c.manifest.release_id)?;
            ensure!(
                resume == saved.is_some(),
                "use start for new runs and resume for existing runs"
            );
        } else if !independent_review {
            ensure!(!resume, "cannot resume unknown release");
        }
        let app = Arc::downgrade(self);
        let operation = a.operation_id.clone();
        let route_owned = route.to_owned();
        let reconcile = a.reconcile.clone();
        let executable = self.executable.clone();
        *running = Some(tokio::spawn(async move {
            let result = if independent_review {
                crate::review::run(&c, executable.as_deref()).await
            } else {
                engine::execute(c, resume, reconcile).await
            };
            if let Some(app) = app.upgrade() {
                let _ = app.journal(
                    &operation,
                    &route_owned,
                    "finished",
                    match result {
                        Ok(v) => v,
                        Err(e) => json!({"error":e.to_string(),"status":"blocked"}),
                    },
                );
            }
        }));
        drop(running);
        // Give the engine a bounded opportunity to journal preparation before offering controls.
        for _ in 0..50 {
            if self.state.join("state.sqlite").exists()
                && Store::open(&self.state.join("state.sqlite"), true)
                    .ok()
                    .and_then(|s| {
                        s.load(self.release_id.as_deref().unwrap_or(""))
                            .ok()
                            .flatten()
                    })
                    .is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok(
            json!({"status":"submitted","operation_id":a.operation_id,"asynchronous":true,"replayed":false}),
        )
    }
    pub(crate) async fn stop_forge(&self) {
        if let Some(handle) = self.forge_run.lock().await.take()
            && !handle.is_finished()
        {
            if let Some(id) = &self.release_id
                && let Ok(mut store) = Store::open(&self.state.join("state.sqlite"), false)
            {
                let _ = store.set_control(id, "cancel");
            }
            let mut handle = handle;
            if tokio::time::timeout(std::time::Duration::from_secs(5), &mut handle)
                .await
                .is_err()
            {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, process::Command, time::Duration};
    pub struct Fixture {
        pub _dir: tempfile::TempDir,
        pub app: Arc<App>,
        pub repo: PathBuf,
        pub manifest: PathBuf,
    }
    impl Fixture {
        pub fn new(human: bool, command: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let base = dir.path().canonicalize().unwrap();
            let repo = base.join("project");
            std::fs::create_dir(&repo).unwrap();
            for args in [
                vec!["init", "-q"],
                vec![
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "fixture",
                ],
            ] {
                assert!(
                    Command::new("git")
                        .args(args)
                        .current_dir(&repo)
                        .status()
                        .unwrap()
                        .success()
                );
            }
            let gate = base.join("gate.py");
            std::fs::write(&gate, "pass\n").unwrap();
            let mut acceptance = vec![json!({"kind":"gate","id":"proof"})];
            if human {
                acceptance.push(json!({"kind":"human","id":"review"}));
            }
            let m = json!({"schema_version":1,"release_id":"controls","repos":{"app":{"path":repo}},"tasks":[{"id":"task","repo":"app","description":"Local disposable control test","executor":{"kind":"command","argv":["python3","-c",command]},"acceptance":acceptance}],"gates":[{"id":"proof","argv":["python3",gate],"inputs":{gate.to_str().unwrap():digest(b"pass\n")}}]});
            let manifest = base.join("release.yaml");
            std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
            let app = App::new(&repo, &base.join("state"), None, Some(manifest.clone())).unwrap();
            Self {
                _dir: dir,
                app,
                repo,
                manifest,
            }
        }
        pub async fn authorized(&self) {
            let mut v = self.app.view.lock().await;
            v.connected = true;
            v.account = json!({"authenticated":true});
        }
        pub fn action(&self, id: &str) -> ForgeAction {
            serde_json::from_value(
                json!({"operation_id":id,"revision":self.app.forge().unwrap()["control_revision"]}),
            )
            .unwrap()
        }
        pub async fn wait(&self, status: &str) -> Value {
            for _ in 0..300 {
                let v = self.app.forge().unwrap();
                if v["operational_state"]["run"]["status"] == status
                    && self
                        .app
                        .forge_run
                        .lock()
                        .await
                        .as_ref()
                        .is_none_or(|h| h.is_finished())
                {
                    return v;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("expected {status}: {}", self.app.forge().unwrap());
        }
    }
    #[tokio::test]
    async fn ui_controls_reuse_engine_authority() {
        let f = Fixture::new(
            false,
            "from pathlib import Path;Path('result').write_text('real command')",
        );
        f.authorized().await;
        let a = f.action("start");
        assert_eq!(
            f.app.forge_action("forge-start", a).await.unwrap()["asynchronous"],
            true
        );
        let v = f.wait("passed").await;
        assert_eq!(
            std::fs::read_to_string(f.repo.join("result")).unwrap(),
            "real command"
        );
        assert_eq!(v["operational_state"]["evidence_current"], true);
    }
    #[tokio::test]
    async fn ui_stale_revision_cannot_be_approved() {
        let f = Fixture::new(true, "pass");
        f.authorized().await;
        f.app
            .forge_action("forge-start", f.action("start"))
            .await
            .unwrap();
        let v = f.wait("awaiting_approval").await;
        let approval = json!({"operation_id":"stale","revision":v["control_revision"],"task":"task","gate":"review","reviewer":"Fixture","fingerprint":v["operational_state"]["run"]["tasks"]["task"]["revision"]["fingerprint"]});
        std::fs::write(f.repo.join("changed"), "new source").unwrap();
        assert!(
            f.app
                .forge_action("forge-approve", serde_json::from_value(approval).unwrap())
                .await
                .is_err()
        );
        assert!(
            f.app
                .forge_action("forge-start", f.action("cannot-start-existing"))
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn ui_duplicate_actions_are_idempotent() {
        let f = Fixture::new(false, "pass");
        f.authorized().await;
        let a = f.action("same");
        f.app.forge_action("forge-start", a).await.unwrap();
        f.wait("passed").await;
        assert!(
            f.app
                .forge_action("forge-start", f.action("same"))
                .await
                .is_err()
        );
        assert!(
            f.app
                .control_journal()
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["status"] == "finished")
        );
    }
    #[tokio::test]
    async fn cancel_does_not_require_stable_source_or_manifest() {
        let f = Fixture::new(false, "import time;time.sleep(30)");
        f.authorized().await;
        f.app
            .forge_action("forge-start", f.action("start"))
            .await
            .unwrap();
        let action = f.action("cancel");
        std::fs::write(&f.manifest, "invalid edited contract").unwrap();
        std::fs::write(f.repo.join("writer-changed"), "changed").unwrap();
        assert!(f.app.forge_action("forge-cancel", action).await.is_ok());
        f.app.stop_forge().await;
        let store = Store::open(&f.app.state.join("state.sqlite"), true).unwrap();
        assert_eq!(store.load("controls").unwrap().unwrap().status, "cancelled");
    }
    #[tokio::test]
    async fn denial_revokes_prior_approval_and_disconnected_ui_is_blocked() {
        let f = Fixture::new(true, "pass");
        assert!(
            f.app
                .forge_action("forge-start", f.action("disconnected"))
                .await
                .is_err()
        );
        f.authorized().await;
        f.app
            .forge_action("forge-start", f.action("start"))
            .await
            .unwrap();
        let v = f.wait("awaiting_approval").await;
        let mut fields = json!({"operation_id":"approve","revision":v["control_revision"],"task":"task","gate":"review","reviewer":"Fixture","fingerprint":v["operational_state"]["run"]["tasks"]["task"]["revision"]["fingerprint"]});
        f.app
            .forge_action(
                "forge-approve",
                serde_json::from_value(fields.clone()).unwrap(),
            )
            .await
            .unwrap();
        fields["operation_id"] = json!("deny");
        f.app
            .forge_action("forge-deny", serde_json::from_value(fields).unwrap())
            .await
            .unwrap();
        f.app
            .forge_action("forge-resume", f.action("resume"))
            .await
            .unwrap();
        f.wait("awaiting_approval").await;
    }
    #[tokio::test]
    async fn pause_is_cooperative_and_resume_requires_current_revision() {
        let f = Fixture::new(true, "import time;time.sleep(.3)");
        f.authorized().await;
        f.app
            .forge_action("forge-start", f.action("start"))
            .await
            .unwrap();
        f.app
            .forge_action("forge-pause", f.action("pause"))
            .await
            .unwrap();
        f.wait("paused").await;
        f.app
            .forge_action("forge-resume", f.action("resume"))
            .await
            .unwrap();
        f.wait("awaiting_approval").await;
    }
    #[test]
    fn controls_reject_unknown_fields() {
        assert!(
            serde_json::from_value::<ForgeAction>(
                json!({"operation_id":"bad","permission":"unsafe"})
            )
            .is_err()
        );
    }
    #[test]
    fn multi_worktree_authorization_is_pinned_at_startup() {
        let f = Fixture::new(false, "pass");
        let other = f.repo.parent().unwrap().join("other");
        std::fs::create_dir(&other).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&other)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
        m["repos"]["other"] = json!({"path":other});
        std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        assert!(
            f.app.forge_context().is_err(),
            "later manifests cannot widen startup roots"
        );
        let app = App::new(&f.repo, &f.app.state, None, Some(f.manifest.clone())).unwrap();
        assert!(
            app.forge_context().is_ok(),
            "explicit restart selects both roots"
        );
        assert_eq!(app.root, f.repo, "native chat root stays selected");
        assert_eq!(app.allowed_roots.len(), 2);
    }
}
