use super::*;
pub fn status(root: &Path, ops: &Path, id: &str) -> Result<Value> {
    let mut value = project_status(root, ops, id)?;
    let state = value
        .get("effective_status")
        .or(value.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let reason = match state {
        "passed" => "completed",
        "prepared" | "awaiting_approval" | "paused" | "cancelled" => "decision_required",
        "running" | "evaluating" => "active",
        "failed" => "external_blocker",
        _ => "continuity_failure",
    };
    value["stop_reason"] = json!(reason);
    let digest = value["scope_digest"].clone();
    value["authorization"] = json!({"scope_granted":!value["scope"].is_null(),"digest_current":!digest.is_null() && value["scope"]["digest"]==digest,"phase":value["scope"]["phase"],"acceptance_granted":false});
    Ok(value)
}
fn project_status(root: &Path, ops: &Path, id: &str) -> Result<Value> {
    let (dir, c) = load_projection(root, ops, id)?;
    let state = dir.join("state/state.sqlite");
    if !state.exists() {
        c.manifest
            .verify_evaluators(&dir, &BTreeMap::from([("project".into(), c.root.clone())]))?;
        return Ok(
            json!({"schema_version":1,"workflow_id":id,"status":"prepared","scope_digest":consent_digest(&c)?,"source_current":revision(&c.root)?==c.source,"next_action":{"command":"workflow start","reason":"Review and explicitly confirm the scope digest"},"evidence_current":false,"mutation":false}),
        );
    }
    let store = Store::open(&state, true)?;
    let scope = store.metadata("workflow-scope")?;
    if store.load(&c.manifest.release_id)?.is_none()
        && scope
            .as_ref()
            .is_none_or(|s| s["phase"] != "execution_authorized")
    {
        return Ok(
            json!({"schema_version":1,"workflow_id":id,"effective_status":"blocked","scope_digest":consent_digest(&c)?,"scope":scope,"planning_metrics":c.planning_metrics,"journal_metrics":store.workflow_observations(&c.manifest.release_id)?,"report":{"schema_version":1,"token_usage":store.token_usage(&c.manifest.release_id,None)?,"evidence_current":false},"last_error":store.metadata("workflow-error")?,"baseline":store.metadata("workflow-baseline")?,"next_action":{"reason":"Inspect the preparation journal and processes, then explicitly workflow resume. Only unchanged preparation and proven completed checkpoints are reusable; uncertain or failed baselines need a fresh reviewed workflow."},"mutation":false,"replayed":false}),
        );
    }
    let report = engine::status(
        &engine::Context::load(&dir.join("release.json"), &dir.join("state"))?,
        true,
    )?;
    let early_control = store.metadata("workflow-control")?;
    let effective = if report["status"] == "not_run" && early_control == Some(json!("pause")) {
        "paused"
    } else if report["status"] == "not_run" && early_control == Some(json!("cancel")) {
        "cancelled"
    } else {
        report["effective_status"]
            .as_str()
            .or(report["status"].as_str())
            .unwrap_or("unknown")
    };
    let action = match effective {
        "passed" => "review report; no execution needed",
        "running" | "evaluating" => "inspect active process; do not start again",
        "awaiting_approval" => {
            "review the exact task/gate revision; scope consent cannot approve it"
        }
        "blocked" => "inspect source/evaluator changes; explicitly reconcile only after review",
        "paused" | "failed" | "cancelled" => {
            "workflow resume after inspecting the retained evidence"
        }
        _ => "inspect preparation journal before any resume",
    };
    let tasks=report["run"]["tasks"].as_object().map(|t|t.iter().map(|(id,t)|json!({"id":id,"status":t["status"],"pid":t["pid"],"process_alive":t["pid"].as_u64().map(|pid|forge_adapters::process::pid_alive(pid as u32)),"evidence":t["evidence"]})).collect::<Vec<_>>()).unwrap_or_default();
    Ok(
        json!({"schema_version":1,"workflow_id":id,"effective_status":effective,"scope_digest":consent_digest(&c)?,"scope":scope,"last_error":store.metadata("workflow-error")?,"baseline":store.metadata("workflow-baseline")?,"tasks":tasks,"report":report,"journal_metrics":store.workflow_observations(&c.manifest.release_id)?,"planning_call":c.planning_call,"planning_metrics":c.planning_metrics,"measurement_scope":if c.planning_metrics.is_some(){"planning_and_execution"}else{"execution_only"},"planning_coverage":c.planning_metrics.as_ref().map(|m|m.coverage.as_str()),"usage_scope":"Execution plus linked successful native planning receipt; failed/unlinked planning and account-wide activity remain outside this journal","next_action":{"reason":action},"mutation":false,"replayed":false}),
    )
}

/// During preparation only the reviewed spec files may be materialized.
/// The index, HEAD, other files, modes and links remain bound to the initial grant.
pub(super) fn preparation_guard(c: &Candidate) -> Result<Value> {
    let paths = git(
        &c.root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    let mut entries = BTreeMap::new();
    for name in paths.split('\0').filter(|p| !p.is_empty()) {
        let path = c.root.join(name);
        if let Some(body) = c.documents.get(&path) {
            guided::check_file(&path, body)?;
            continue;
        }
        let entry = match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => json!({"link":std::fs::read_link(&path)?}),
            Ok(meta) if meta.is_file() => {
                ensure!(
                    meta.len() <= 64 * 1024 * 1024,
                    "file exceeds preparation fingerprint bound"
                );
                #[cfg(unix)]
                let mode = {
                    use std::os::unix::fs::PermissionsExt;
                    meta.permissions().mode()
                };
                #[cfg(not(unix))]
                let mode = meta.permissions().readonly() as u32;
                json!({"sha256":digest(&std::fs::read(&path)?),"mode":mode})
            }
            Ok(_) => json!("non-file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!("deleted"),
            Err(e) => return Err(e.into()),
        };
        entries.insert(name, entry);
    }
    Ok(json!(digest(&serde_json::to_vec(
        &json!({"head":git(&c.root,&["rev-parse","HEAD"] )?,"index":git(&c.root,&["diff","--cached","--binary"] )?,"files":entries})
    )?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn candidate() -> (tempfile::TempDir, PathBuf, Candidate) {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let root = base.join("repo");
        std::fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q"]).unwrap();
        git(
            &root,
            &[
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
        )
        .unwrap();
        std::fs::write(root.join("source.txt"), "original").unwrap();
        let r:Request=serde_json::from_value(json!({"schema_version":1,"id":"059-recover","title":"Recover","request":"Verify","recipe":"bug","files":{"verify.py":"raise SystemExit(42)"},"checks":[{"id":"bug","argv":["python3","{checks}/verify.py"],"before_exit":42}]})).unwrap();
        let ops = base.join("ops");
        let v = prepare::prepare(r, &root, &ops, true).unwrap();
        let c: Candidate = serde_json::from_value(v["candidate"].clone()).unwrap();
        let dir = directory(&root, &ops, "059-recover").unwrap();
        (tmp, dir, c)
    }
    #[test]
    fn partial_materialization_is_idempotent_but_other_source_changes_block() {
        let (_tmp, _dir, c) = candidate();
        let original = preparation_guard(&c).unwrap();
        for (path, body) in &c.documents {
            guided::create(path, body).unwrap();
            assert_eq!(preparation_guard(&c).unwrap(), original);
        }
        std::fs::write(c.root.join("source.txt"), "changed").unwrap();
        assert_ne!(preparation_guard(&c).unwrap(), original);
        let (path, _) = c.documents.first_key_value().unwrap();
        std::fs::write(path, "unexpected scope").unwrap();
        assert!(preparation_guard(&c).is_err());
    }
    #[tokio::test]
    async fn uncertain_baseline_is_never_replayed_on_resume() {
        let (_tmp, dir, c) = candidate();
        let approval = consent_digest(&c).unwrap();
        let mut store = Store::open(&dir.join("state/state.sqlite"), false).unwrap();
        store.journal_metadata(&c.manifest.release_id,"workflow-scope",&json!({"digest":approval,"phase":"preparing","preparation_guard":preparation_guard(&c).unwrap()})).unwrap();
        store
            .journal_metadata(
                &c.manifest.release_id,
                "workflow-baseline:bug",
                &json!({"status":"running"}),
            )
            .unwrap();
        drop(store);
        let error = super::super::continue_preparation(dir, c, &approval, true)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("uncertain"), "{error}");
    }
}
