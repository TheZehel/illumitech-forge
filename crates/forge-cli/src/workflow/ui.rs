//! Both frontends share the scope service; browsers pass IDs, not manifest paths.
use super::*;
use crate::workbench::App;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub operation_id: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub request: Option<Request>,
    #[serde(default)]
    pub scope_digest: Option<String>,
    #[serde(default)]
    pub reconcile: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub gate: Option<String>,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub sarif: Option<Value>,
    #[serde(default)]
    pub baseline: Option<Value>,
    #[serde(default)]
    pub candidate: Option<Value>,
}
impl App {
    pub fn workflow_ops(&self) -> Result<PathBuf> {
        guided::external(&self.state.join("workflow-operations"), &self.root)
    }
    fn prepare_workflow(&self, r: Request, apply: bool) -> Result<Value> {
        let call = if let Some(id) = &r.planning_operation {
            ensure!(
                valid_id(id) && id.len() <= 150,
                "invalid planning receipt ID"
            );
            let value: Value = read_json(
                &self
                    .state
                    .join("workbench-candidates/plans")
                    .join(format!("{id}.json")),
            )?;
            ensure!(
                serde_json::to_value(
                    r.proposal
                        .as_ref()
                        .context("planning receipt requires its proposal")?
                )? == value["proposal"],
                "planning receipt/proposal mismatch"
            );
            ensure!(
                value["manifest"]["repos"]["project"]["path"]
                    == self.root.to_string_lossy().as_ref(),
                "planning receipt belongs to another project"
            );
            let usage: forge_domain::usage::TokenUsage =
                serde_json::from_value(value["token_usage"]["observed"].clone())?;
            Some(forge_domain::usage::TokenCall{id:format!("workflow-planning:{id}"),task_id:"workflow-planning".into(),attempt:1,phase:"planning".into(),repo_scope:self.root.to_string_lossy().into_owned(),client:"codex".into(),model:None,outcome:"passed".into(),elapsed_ms:None,observation:forge_domain::usage::UsageObservation{units:vec![forge_domain::usage::UsageUnit{key:id.clone(),session:id.clone(),cumulative:true,token_usage:usage}],complete:true,limitations:vec!["Observed planning receipt from the owning local server; model and planning duration not observed here".into()]}})
        } else {
            None
        };
        prepare::prepare_with_receipt(r, &self.root, &self.workflow_ops()?, apply, call)
    }
    pub async fn workflow_action(self: &Arc<Self>, route: &str, action: Action) -> Result<Value> {
        ensure!(
            [
                "workflow-preview",
                "workflow-prepare",
                "workflow-start",
                "workflow-status",
                "workflow-resume",
                "workflow-pause",
                "workflow-cancel",
                "workflow-recipes",
                "workflow-findings",
                "workflow-compare",
                "workflow-list",
                "workflow-open",
                "workflow-approve",
                "workflow-review"
            ]
            .contains(&route),
            "unknown workflow action"
        );
        let _guard = self.forge_control.lock().await;
        if route == "workflow-list" {
            return list(&self.root, &self.workflow_ops()?);
        }
        if route == "workflow-open" {
            let (_, c) = load(
                &self.root,
                &self.workflow_ops()?,
                action.id.as_deref().context("workflow ID required")?,
            )?;
            return Ok(
                json!({"schema_version":1,"workflow_id":c.request.id,"status":"prepared","scope_digest":consent_digest(&c)?,"candidate":c,"executed":false}),
            );
        }
        if route == "workflow-findings" {
            return crate::findings::parse(&action.sarif.context("SARIF required")?, &self.root);
        }
        if route == "workflow-compare" {
            return crate::comparison::compare(
                &action.baseline.context("baseline report required")?,
                &action.candidate.context("candidate report required")?,
            );
        }
        if route == "workflow-recipes" {
            return Ok(
                json!({"schema_version":1,"recipes":["feature","bug","dependencies","pr"],"checks":prepare::suggested(&self.root)?,"scope":"One reviewed plan; no implicit publication, gate acceptance or native permission bypass"}),
            );
        }
        if route == "workflow-status" {
            return recovery::status(
                &self.root,
                &self.workflow_ops()?,
                action.id.as_deref().context("workflow ID required")?,
            );
        }
        if route == "workflow-preview" {
            return self
                .prepare_workflow(action.request.context("workflow request required")?, false);
        }
        if route == "workflow-review" {
            let diff = git(
                &self.root,
                &["diff", "HEAD", "--no-ext-diff", "--no-textconv"],
            )?;
            let untracked = git(&self.root, &["ls-files", "--others", "--exclude-standard"])?;
            return Ok(
                json!({"schema_version":1,"diff":forge_adapters::process::redact(&diff).chars().take(32768).collect::<String>(),"truncated":diff.chars().count()>32768,"untracked_files":untracked.chars().take(16384).collect::<String>(),"review":"Inspect untracked files locally; scope consent does not grant acceptance","mutation":false}),
            );
        }
        self.reserve(&action.operation_id)?;
        if route == "workflow-preview" || route == "workflow-prepare" {
            let request = action.request.context("workflow request required")?;
            return self.prepare_workflow(request, route == "workflow-prepare");
        }
        let id = action.id.context("workflow ID required")?;
        let (dir, _) = load(&self.root, &self.workflow_ops()?, &id)?;
        if route == "workflow-approve" {
            return engine::approve(
                &engine::Context::load(&dir.join("release.json"), &dir.join("state"))?,
                action.task.as_deref().context("task required")?,
                action.gate.as_deref().context("gate required")?,
                action.reviewer.as_deref().context("reviewer required")?,
                action
                    .fingerprint
                    .as_deref()
                    .context("evaluated fingerprint required")?,
            );
        }
        if route == "workflow-pause" || route == "workflow-cancel" {
            let state = dir.join("state/state.sqlite");
            if state.exists() && Store::open(&state, true)?.load(&id)?.is_none() {
                if route == "workflow-cancel"
                    && let Some(h) = self.forge_run.lock().await.take()
                {
                    h.abort();
                    let _ = h.await;
                }
                let mut store = Store::open(&state, false)?;
                store.journal_metadata(
                    &id,
                    "workflow-control",
                    &json!(if route == "workflow-pause" {
                        "pause"
                    } else {
                        "cancel"
                    }),
                )?;
                return Ok(
                    json!({"schema_version":1,"status":if route=="workflow-pause"{"pause_requested"}else{"cancelled"},"replayed":false}),
                );
            }
            return engine::control(
                &engine::Context::load(&dir.join("release.json"), &dir.join("state"))?,
                if route.ends_with("pause") {
                    "pause"
                } else {
                    "cancel"
                },
            );
        }
        let mut running = self.forge_run.lock().await;
        ensure!(
            running.as_ref().is_none_or(|h| h.is_finished()),
            "another release operation is running"
        );
        if route == "workflow-start" {
            let v = self.view.lock().await;
            ensure!(
                v.connected && v.account["authenticated"] == true,
                "connect an authenticated native client before execution"
            );
            let (candidate_dir, c) = load(&self.root, &self.workflow_ops()?, &id)?;
            let candidate_state = candidate_dir.join("state/state.sqlite");
            ensure!(
                !candidate_state.exists()
                    || Store::open(&candidate_state, true)?
                        .metadata("workflow-scope")?
                        .is_none(),
                "scope already submitted; inspect status or resume instead of repeating start"
            );
            ensure!(
                revision(&self.root)? == c.source,
                "project changed after preparation; review a fresh candidate before execution"
            );
            ensure!(
                action.scope_digest.as_deref() == Some(consent_digest(&c)?.as_str()),
                "scope changed; review again"
            );
        }
        let ops = self.workflow_ops()?;
        let app = Arc::clone(self);
        let workflow_id = id.clone();
        let digest = action.scope_digest;
        let reconcile = action.reconcile;
        let resume_action = route == "workflow-resume";
        *self.workflow_active.lock().await = Some((dir.join("state/state.sqlite"), id.clone()));
        *running = Some(tokio::spawn(async move {
            let result = if resume_action {
                resume(&app.root, &ops, &workflow_id, reconcile).await
            } else {
                start(
                    &app.root,
                    &ops,
                    &workflow_id,
                    digest.as_deref().unwrap_or(""),
                )
                .await
            };
            app.workflow_active.lock().await.take();
            if let Err(e) = result
                && let Ok((dir, _)) = load(&app.root, &ops, &workflow_id)
                && let Ok(store) = Store::open(&dir.join("state/state.sqlite"), false)
            {
                let _=store.set_metadata("workflow-error",&json!({"error":forge_adapters::process::redact(&e.to_string()),"replayed":false}));
            }
        }));
        Ok(
            json!({"schema_version":1,"status":"scheduled","workflow_id":id,"replayed":false,"acceptance":"not_evaluated"}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planning_receipt_is_project_bound_and_keeps_observed_usage() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir(&root).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--allow-empty",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(&root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let state = base.join("state");
        let app = App::new(&root, &state, None, None).unwrap();
        let proposal = json!({"schema_version":1,"id":"001-fixture","title":"Fixture","summary":"Verify fixture","tasks":[{"id":"T01","title":"Fixture","dependencies":[],"acceptance":[{"id":"AC1","description":"Observable check","command":["true"]}]}]});
        let r:Request=serde_json::from_value(json!({"schema_version":1,"id":"001-fixture","title":"Fixture","request":"Verify fixture","recipe":"feature","checks":[{"id":"verify","argv":["true"]}],"proposal":proposal,"planning_operation":"fixture-plan"})).unwrap();
        let path = state.join("workbench-candidates/plans/fixture-plan.json");
        let mut receipt = json!({"proposal":proposal,"manifest":{"repos":{"project":{"path":root}}},"token_usage":{"coverage":"complete","observed":{"input_tokens":10,"cached_input_tokens":0,"output_tokens":1,"total_tokens":11}}});
        guided::create(&path, &receipt.to_string()).unwrap();
        let preview = app.prepare_workflow(r.clone(), false).unwrap();
        assert_eq!(
            preview["candidate"]["planning_call"]["observation"]["units"][0]["token_usage"]["total_tokens"],
            11
        );
        assert!(!root.join("specs").exists());
        assert!(prepare::prepare(r.clone(), &root, &base.join("ops"), false).is_err());
        receipt["manifest"]["repos"]["project"]["path"] = json!(base.join("other-project"));
        std::fs::write(path, receipt.to_string()).unwrap();
        assert!(app.prepare_workflow(r, false).is_err());
    }
}
