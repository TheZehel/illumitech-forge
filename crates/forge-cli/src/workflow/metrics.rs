//! Local planning attempts and a closed, typed export boundary. No transport.
use super::*;
use std::time::Instant;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningMetrics {
    pub schema_version: u32,
    pub attempt_id: String,
    pub status: String,
    pub active_ms: Option<u64>,
    pub tokens: forge_domain::usage::TokenUsage,
    pub coverage: String,
}
pub struct Attempt {
    pub id: String,
    dir: PathBuf,
    started: Instant,
    finished: bool,
}
impl Attempt {
    pub fn begin(ops: &Path, output: &Path, root: &Path) -> Result<Self> {
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|e| anyhow::anyhow!("attempt ID: {e}"))?;
        let id = digest(&nonce);
        let dir = ops.join("attempts").join(&id);
        guided::create(&dir.join("started.json"), &json!({"schema_version":1,"attempt_id":id,"planning_receipt":digest(output.to_string_lossy().as_bytes()),"project":digest(root.to_string_lossy().as_bytes()),"status":"planning","started_ms":crate::duration::now_ms()}).to_string())?;
        Ok(Self {
            id,
            dir,
            started: Instant::now(),
            finished: false,
        })
    }
    pub fn finish(&mut self, passed: bool, observed: &Value) -> Result<()> {
        let usage = forge_domain::usage::TokenUsage::new(
            observed["total"]["inputTokens"].as_u64(),
            observed["total"]["cachedInputTokens"].as_u64(),
            observed["total"]["outputTokens"].as_u64(),
        );
        guided::create(&self.dir.join("finished.json"), &json!({"schema_version":1,"attempt_id":self.id,"status":if passed{"planned"}else{"failed"},"active_ms":self.started.elapsed().as_millis(),"tokens":usage,"coverage":if usage.total_tokens.is_some(){"complete"}else{"unavailable"}}).to_string())?;
        self.finished = true;
        Ok(())
    }
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if !self.finished {
            // A crash may leave only started.json. Neither case implies success or zero tokens.
            let _ = guided::create(&self.dir.join("interrupted.json"), &json!({"schema_version":1,"attempt_id":self.id,"status":"interrupted","active_ms":self.started.elapsed().as_millis(),"tokens":null,"coverage":"unavailable"}).to_string());
        }
    }
}
fn outcome(value: &Value) -> &'static str {
    match value.as_str().unwrap_or("") {
        "passed" => "passed",
        "failed" => "failed",
        "cancelled" => "cancelled",
        "paused" => "paused",
        "blocked" => "blocked",
        "prepared" => "prepared",
        "awaiting_approval" => "awaiting_approval",
        "running" => "running",
        "evaluating" => "evaluating",
        _ => "unknown",
    }
}
/// Every emitted field is constructed here. Never copy arbitrary report subtrees.
fn shareable(id: &str, status: &Value, human_minutes: Option<u64>) -> Value {
    let r = &status["report"];
    let planning = &status["planning_metrics"];
    let tokens = &r["token_usage"];
    let coverage = match tokens["coverage"].as_str() {
        Some("complete") => "complete",
        Some("partial") => "partial",
        _ => "unavailable",
    };
    let attempts = r["run"]["tasks"].as_object().and_then(|tasks| {
        tasks
            .values()
            .try_fold(0u64, |n, t| n.checked_add(t["attempts"].as_u64()?))
    });
    json!({"schema_version":1,"export_version":1,"workflow_pseudonym":digest(id.as_bytes()),"forge_version":env!("CARGO_PKG_VERSION"),"outcome":outcome(status.get("effective_status").unwrap_or(&status["status"])),"evidence_current":r["evidence_current"].as_bool().unwrap_or(false),"metrics":{"execution_attempts":attempts,"active_ms":r["run"]["duration"]["observed_active_ms"].as_u64(),"wait_ms":status["journal_metrics"]["wait_ms"].as_u64(),"completed_wait_ms":status["journal_metrics"]["completed_wait_ms"].as_u64(),"observed_stops":status["journal_metrics"]["observed_stops"].as_u64(),"interruptions":if status["journal_metrics"]["active_or_uncertain"]==false{status["journal_metrics"]["observed_stops"].as_u64()}else{None},"interruption_coverage":if status["journal_metrics"]["active_or_uncertain"]==false{"persisted_journal"}else{"unknown"},"decision_stops":status["journal_metrics"]["decision_stops"].as_u64(),"external_stops":status["journal_metrics"]["external_stops"].as_u64(),"tokens":{"coverage":coverage,"input":tokens["input_tokens"].as_u64(),"output":tokens["output_tokens"].as_u64(),"total":tokens["total_tokens"].as_u64()},"human_minutes":human_minutes,"human_time_source":if human_minutes.is_some(){"manual"}else{"unknown"}},"planning_attempt_pseudonym":planning["attempt_id"].as_str().map(|v|digest(v.as_bytes())),"planning_active_ms":planning["active_ms"].as_u64(),"full_active_ms":planning["active_ms"].as_u64().zip(r["run"]["duration"]["observed_active_ms"].as_u64()).and_then(|(a,b)|a.checked_add(b)),"wait_scope":"execution stops until explicit resume; native approval and pre-execution wait unobserved","measurement_scope":if planning["status"]=="planned"{"planning_and_execution"}else{"execution_only"},"planning_coverage":if planning["coverage"]=="complete"{"complete"}else{"unavailable"},"coverage_scope":"linked planning and execution; separate failed attempts remain visible in the local journal","network_sent":false})
}
pub fn export(
    root: &Path,
    ops: &Path,
    id: &str,
    output: Option<&Path>,
    human_minutes: Option<u64>,
) -> Result<Value> {
    let value = if attempt_id(id) {
        attempt_report(root, ops, id, human_minutes)?
    } else {
        let status = recovery::status(root, ops, id)?;
        shareable(id, &status, human_minutes)
    };
    if let Some(path) = output {
        ensure!(
            !guided::safe_path(path)?.exists(),
            "export already exists; choose a new versioned filename"
        );
        guided::create(path, &serde_json::to_string_pretty(&value)?)?;
    }
    Ok(
        json!({"schema_version":1,"status":if output.is_some(){"written"}else{"preview"},"report":value,"mutation":output.is_some()}),
    )
}
fn attempt_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn attempt_report(root: &Path, ops: &Path, id: &str, human_minutes: Option<u64>) -> Result<Value> {
    ensure!(attempt_id(id), "invalid attempt ID");
    let root = guided::root(root)?;
    let dir = guided::external(ops, &root)?.join("attempts").join(id);
    let started: Value = read_json(&dir.join("started.json"))?;
    ensure!(
        started["project"] == digest(root.to_string_lossy().as_bytes())
            && started["attempt_id"] == id
            && started["schema_version"] == 1,
        "attempt identity mismatch"
    );
    let receipt: Value = if dir.join("finished.json").exists() {
        read_json(&dir.join("finished.json"))?
    } else if dir.join("interrupted.json").exists() {
        read_json(&dir.join("interrupted.json"))?
    } else {
        Value::Null
    };
    ensure!(
        receipt.is_null() || receipt["attempt_id"] == id,
        "attempt receipt mismatch"
    );
    let state = match receipt["status"].as_str() {
        Some("planned") => "planned",
        Some("failed") => "failed",
        Some("interrupted") => "interrupted",
        _ => "unknown",
    };
    Ok(
        json!({"schema_version":1,"export_version":1,"attempt_pseudonym":digest(id.as_bytes()),"forge_version":env!("CARGO_PKG_VERSION"),"outcome":state,"coverage":if receipt["coverage"]=="complete"{"complete"}else{"unavailable"},"active_ms":receipt["active_ms"].as_u64(),"tokens":{"input":receipt["tokens"]["input_tokens"].as_u64(),"output":receipt["tokens"]["output_tokens"].as_u64(),"total":receipt["tokens"]["total_tokens"].as_u64()},"human_minutes":human_minutes,"human_time_source":if human_minutes.is_some(){"manual"}else{"unknown"},"workflow_link":"use the same attempt pseudonym in workflow exports; do not count both as separate attempts","network_sent":false}),
    )
}
pub fn attempts(root: &Path, ops: &Path) -> Result<Value> {
    let root = guided::root(root)?;
    let ops = guided::external(ops, &root)?;
    let directory = guided::safe_path(&ops.join("attempts"))?;
    let mut rows = vec![];
    if directory.exists() {
        for entry in std::fs::read_dir(directory)?.take(1000) {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type()?.is_dir() && attempt_id(&id) {
                rows.push(json!({"id":id,"report":attempt_report(&root,&ops,&id,None)?}));
            }
        }
    }
    rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(
        json!({"schema_version":1,"attempts":rows,"limit":1000,"mutation":false,"native_inference_calls":0}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_drops_arbitrary_sensitive_strings_and_unknown_is_not_zero() {
        let raw = json!({"status":"SECRET","prompt":"SECRET","report":{"token_usage":{"coverage":"SECRET","total_tokens":"SECRET"},"run":{"tasks":{"SECRET":{"attempts":1,"logs":"SECRET"}}},"remote":"SECRET"}});
        let value = shareable("SECRET", &raw, Some(12));
        assert!(!value.to_string().contains("SECRET"));
        assert!(value["metrics"]["tokens"]["total"].is_null());
        assert_eq!(value["metrics"]["human_time_source"], "manual");
    }
    #[test]
    fn failed_and_dropped_planning_attempts_are_preserved() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().canonicalize().unwrap();
        let mut a = Attempt::begin(&base, &base.join("proposal.json"), &base).unwrap();
        a.finish(false, &Value::Null).unwrap();
        let b = Attempt::begin(&base, &base.join("proposal.json"), &base).unwrap();
        let path = b.dir.clone();
        drop(b);
        assert!(a.dir.join("finished.json").exists());
        assert!(path.join("interrupted.json").exists());
        assert_ne!(a.dir, path);
    }
    #[test]
    fn failed_attempt_without_workflow_can_be_previewed_and_exported() {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q"]).unwrap();
        let ops = base.join("ops");
        let mut a = Attempt::begin(&ops, &ops.join("proposal.json"), &root).unwrap();
        a.finish(false, &Value::Null).unwrap();
        let preview = export(&root, &ops, &a.id, None, None).unwrap();
        assert_eq!(preview["status"], "preview");
        assert_eq!(preview["report"]["outcome"], "failed");
        assert!(preview["report"]["tokens"]["total"].is_null());
        assert!(!preview.to_string().contains(root.to_str().unwrap()));
        let output = base.join("attempt-v1.json");
        export(&root, &ops, &a.id, Some(&output), Some(3)).unwrap();
        assert!(output.exists());
        assert_eq!(
            attempts(&root, &ops).unwrap()["attempts"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let other = base.join("other");
        std::fs::create_dir(&other).unwrap();
        git(&other, &["init", "-q"]).unwrap();
        assert!(export(&other, &ops, &a.id, None, None).is_err());
    }
}
