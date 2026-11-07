//! Read-only handoff projection. All authority stays in the engine and durable journal.
use crate::diagnostics::{action, blocked};
use serde_json::{Value, json};

pub struct Context<'a> {
    pub kind: &'a str,
    pub id: &'a str,
    pub objective: &'a str,
    pub inspect_argv: Vec<String>,
    pub references: Value,
}
pub fn attach(value: &mut Value, context: Context<'_>) {
    let state = value["effective_status"]
        .as_str()
        .or(value["status"].as_str())
        .unwrap_or("unknown")
        .to_owned();
    let active = value["tasks"]
        .as_array()
        .is_some_and(|t| t.iter().any(|t| t["process_alive"] == true));
    let stale = value["source_current"] == false
        || value["scope_current"] == false
        || (!value["report"]["run"].is_null() && value["report"]["evidence_current"] == false);
    let (code, reason, guidance) = if active || matches!(state.as_str(), "running" | "evaluating") {
        (
            "process_active",
            "Execution is active or its outcome is not yet established",
            "Inspect the process and retained observations before considering another execution",
        )
    } else if stale {
        (
            "evidence_stale",
            "Source, scope or evaluator differs from the recorded work",
            "Inspect changes and prepare a new reviewed scope or explicitly reconcile where supported",
        )
    } else {
        match state.as_str() {
            "passed" => (
                "completed",
                "Recorded checks passed for the observed scope",
                "Review current evidence and remaining human gates",
            ),
            "prepared" => (
                "scope_approval_required",
                "A candidate is prepared; execution has not been authorized",
                "Review the exact scope digest and commands before choosing start",
            ),
            "awaiting_approval" => (
                "human_approval_required",
                "A declared human acceptance gate is pending",
                "Review the exact evaluated revision; initial scope consent is not acceptance",
            ),
            "paused" | "cancelled" => (
                "decision_required",
                "Work stopped with retained evidence",
                "Inspect retained evidence before explicitly resuming the existing workflow",
            ),
            "failed" => (
                "validation_failed",
                "A recorded check or task failed",
                "Inspect the failed task or check and its logs before deciding on correction or a new scope",
            ),
            "degraded" => (
                "coverage_incomplete",
                "The requested QA coverage is incomplete",
                "Review unavailable or unexecuted checks and prepare a fresh reviewed QA scope when needed",
            ),
            _ => (
                "outcome_uncertain",
                "The outcome is blocked or unknown",
                "Inspect the durable journal, processes and evidence; do not replay uncertain operations",
            ),
        }
    };
    let rows = value["tasks"]
        .as_array()
        .or_else(|| value["checks"].as_array())
        .cloned()
        .unwrap_or_default();
    let rows = if rows.is_empty() {
        value["report"]["run"]["tasks"]
            .as_object()
            .map(|t| {
                t.iter()
                    .map(|(id, t)| json!({"id":id,"status":t["status"],"evidence":t["evidence"]}))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        rows
    };
    let completed = rows.iter().filter(|r| r["status"] == "passed").count();
    let gaps = value["coverage"]
        .as_array()
        .map(|r| {
            r.iter()
                .filter(|r| !matches!(r["status"].as_str(), Some("passed" | "not_requested")))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let diagnostic = blocked(code, reason, context.kind, guidance);
    let actions = vec![
        action(
            "Inspect the current durable state",
            Some(context.inspect_argv),
            false,
        ),
        action(guidance, None, true),
    ];
    value["continuity"] = json!({"schema_version":1,"kind":context.kind,"id":context.id,"objective":context.objective,
        "status":state,"stop_reason":code,"progress":{"completed":completed,"total":rows.len(),"items":rows},
        "coverage_gaps":gaps,"evidence_current":value["evidence_current"].as_bool().or(value["report"]["evidence_current"].as_bool()),
        "references":context.references,"diagnostic":diagnostic,"next_actions":actions,"authority":"none","mutation":false,"replayed":false});
    value["diagnostic"] = value["continuity"]["diagnostic"].clone();
}
pub fn human(value: &Value) -> Option<String> {
    let c = &value["continuity"];
    Some(format!(
        "{} · {}\n{}\n{}/{}",
        c["id"].as_str()?,
        c["status"].as_str()?,
        c["objective"].as_str()?,
        c["progress"]["completed"],
        c["progress"]["total"]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_unknown_and_stale_never_suggest_replay() {
        for (state, extra, expected) in [
            (
                "failed",
                json!({"tasks":[{"id":"one","status":"running","process_alive":true}]}),
                "process_active",
            ),
            ("blocked", json!({}), "outcome_uncertain"),
            ("passed", json!({"scope_current":false}), "evidence_stale"),
        ] {
            let mut v = json!({"status":state});
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            attach(
                &mut v,
                Context {
                    kind: "workflow",
                    id: "001-test",
                    objective: "Test",
                    inspect_argv: vec!["forge".into(), "workflow".into(), "status".into()],
                    references: json!({}),
                },
            );
            assert_eq!(v["diagnostic"]["code"], expected);
            assert_eq!(v["continuity"]["authority"], "none");
            assert!(v["continuity"]["next_actions"][1]["argv"].is_null());
        }
    }
}
