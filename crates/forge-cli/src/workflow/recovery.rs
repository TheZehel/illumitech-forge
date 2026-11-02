use super::*;
pub fn status(root: &Path, ops: &Path, id: &str) -> Result<Value> {
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
            json!({"schema_version":1,"workflow_id":id,"effective_status":"blocked","scope":scope,"last_error":store.metadata("workflow-error")?,"baseline":store.metadata("workflow-baseline")?,"next_action":{"reason":"Preparation or baseline was interrupted before a safe execution checkpoint. Inspect retained files/journal and prepare a fresh reviewed workflow; no command replayed."},"mutation":false,"replayed":false}),
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
        json!({"schema_version":1,"workflow_id":id,"effective_status":effective,"scope_digest":consent_digest(&c)?,"scope":scope,"last_error":store.metadata("workflow-error")?,"baseline":store.metadata("workflow-baseline")?,"tasks":tasks,"report":report,"planning_call":c.planning_call,"usage_scope":"Execution plus linked successful native planning receipt; failed/unlinked planning and account-wide activity remain outside this journal","next_action":{"reason":action},"mutation":false,"replayed":false}),
    )
}
