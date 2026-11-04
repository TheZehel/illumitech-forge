//! Read-only comparison of recorded deliveries, not token-price or subscription inference.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::path::Path;
fn report(v: &Value) -> &Value {
    v.get("report").unwrap_or(v)
}
fn outcome(v: &Value) -> &str {
    v["effective_status"]
        .as_str()
        .or(v["run"]["status"].as_str())
        .or(v["status"].as_str())
        .unwrap_or("unknown")
}
fn summary(v: &Value) -> Value {
    let r = report(v);
    let accepted = outcome(r) == "passed" && r["evidence_current"] == true;
    let tokens = &r["token_usage"];
    let total = if tokens["coverage"] == "complete" {
        tokens["total_tokens"].as_u64()
    } else {
        None
    };
    let attempts = r["run"]["tasks"].as_object().map(|m| {
        m.values()
            .filter_map(|t| t["attempts"].as_u64())
            .sum::<u64>()
    });
    let execution_active = r["run"]["duration"]["observed_active_ms"].as_u64();
    let planning_active = v["planning_metrics"]["active_ms"].as_u64();
    let full_active = execution_active
        .zip(planning_active)
        .and_then(|(a, b)| a.checked_add(b));
    json!({"outcome":outcome(r),"accepted":accepted,"token_usage":tokens,"observed_total_tokens":total,"tokens_per_accepted_delivery":if accepted{total}else{None},"attempts":attempts,"active_ms":full_active,"execution_active_ms":execution_active,"planning_active_ms":planning_active,"planning_attempts":if v["planning_metrics"]["attempt_id"].is_string(){Some(1u64)}else{None},"wait_ms":v["journal_metrics"]["wait_ms"].as_u64(),"interruptions":if v["journal_metrics"]["active_or_uncertain"]==false{v["journal_metrics"]["observed_stops"].as_u64()}else{None},"duration":r["duration_comparison"],"failed_or_cancelled_included":true})
}
pub fn compare(a: &Value, b: &Value) -> Result<Value> {
    ensure!(
        report(a)["schema_version"] == 1 && report(b)["schema_version"] == 1,
        "schema-version-1 reports required"
    );
    let mut reasons = vec![];
    // Context is an operator declaration, explicitly separated from measured counters.
    for key in [
        "case",
        "model",
        "client_version",
        "source_commit",
        "verification_sha256",
        "configuration_sha256",
    ] {
        let av = a["evaluation_context"][key].as_str();
        let bv = b["evaluation_context"][key].as_str();
        if av.is_none_or(str::is_empty) || av != bv {
            reasons.push(format!("missing or different {key}"));
        }
    }
    let sa = summary(a);
    let sb = summary(b);
    let a_scope = a["measurement_scope"].as_str();
    let b_scope = b["measurement_scope"].as_str();
    if a_scope != Some("planning_and_execution") || a_scope != b_scope {
        reasons.push("missing or different full-workflow measurement scope".into());
    }
    if a["planning_coverage"] != "complete" || b["planning_coverage"] != "complete" {
        reasons.push("planning coverage incomplete or unknown".into());
    }
    let compatible = reasons.is_empty();
    let difference = if compatible && sa["accepted"] == true && sb["accepted"] == true {
        sa["observed_total_tokens"].as_u64().zip(sb["observed_total_tokens"].as_u64()).map(|(x,y)|json!({"baseline":x,"candidate":y,"candidate_minus_baseline":i128::from(y)-i128::from(x),"observed_reduction_percent":if x>0{Some((x as f64-y as f64)*100.0/x as f64)}else{None}}))
    } else {
        None
    };
    Ok(
        json!({"schema_version":1,"comparison_schema":1,"status":"compared","compatible":compatible,"incompatibilities":reasons,"comparability_basis":"operator-declared experiment context; not independently authenticated by this reader","baseline":sa,"candidate":sb,"observed_difference":difference,"currency":null,"subscription_balance":null,"mutation":false,"native_inference_calls":0,"limitations":["Exploratory observations, not generalized savings or quality equivalence","Failed, blocked, cancelled, stale and incomplete samples remain visible; no successful-delivery ratio is fabricated","Cached input is part of input; missing coverage is unknown","Native invocations may contain several provider requests; hooks/background activity can be outside this journal"]}),
    )
}
pub fn files(a: &Path, b: &Path) -> Result<Value> {
    compare(
        &crate::workflow::read_json(a)?,
        &crate::workflow::read_json(b)?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Value {
        json!({"schema_version":1,"effective_status":"passed","evidence_current":true,"token_usage":{"coverage":"complete","total_tokens":100},"evaluation_context":{"case":"bug","model":"same","client_version":"same","source_commit":"same","verification_sha256":"same","configuration_sha256":"same"}})
    }
    #[test]
    fn missing_is_not_zero_and_mismatch_cannot_claim_savings() {
        let a = sample();
        let mut b = sample();
        b["evaluation_context"]["model"] = json!("other");
        let c = compare(&a, &b).unwrap();
        assert_eq!(c["compatible"], false);
        assert!(c["observed_difference"].is_null());
        b["evaluation_context"] = a["evaluation_context"].clone();
        b["token_usage"]["total_tokens"] = Value::Null;
        assert!(compare(&a, &b).unwrap()["candidate"]["tokens_per_accepted_delivery"].is_null());
    }
    #[test]
    fn failures_and_stale_evidence_are_not_accepted() {
        let a = sample();
        let mut b = a.clone();
        b["evidence_current"] = json!(false);
        assert_eq!(compare(&a, &b).unwrap()["candidate"]["accepted"], false);
        b["effective_status"] = json!("failed");
        assert!(compare(&a, &b).unwrap()["observed_difference"].is_null());
    }
    #[test]
    fn savings_require_full_planning_coverage_and_matching_conditions() {
        let mut a = sample();
        let mut b = sample();
        b["token_usage"]["total_tokens"] = json!(80);
        assert!(compare(&a, &b).unwrap()["observed_difference"].is_null());
        for v in [&mut a, &mut b] {
            v["measurement_scope"] = json!("planning_and_execution");
            v["planning_coverage"] = json!("complete");
        }
        assert_eq!(
            compare(&a, &b).unwrap()["observed_difference"]["observed_reduction_percent"],
            20.0
        );
        b["token_usage"]["coverage"] = json!("partial");
        assert!(compare(&a, &b).unwrap()["observed_difference"].is_null());
    }
}
