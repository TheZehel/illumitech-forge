//! Typed operational explanations. Suggestions never grant execution authority.
use serde::Serialize;
use serde_json::{Value, json};
use std::fmt;

#[derive(Clone, Debug, Serialize)]
pub struct Action {
    pub reason: String,
    pub argv: Option<Vec<String>>,
    pub requires_review: bool,
}
pub fn action(reason: &str, argv: Option<Vec<String>>, requires_review: bool) -> Action {
    Action {
        reason: reason.into(),
        argv,
        requires_review,
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub schema_version: u32,
    pub code: String,
    pub cause: String,
    pub operation: String,
    pub actions: Vec<Action>,
    pub authority: &'static str,
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.cause)
    }
}
impl std::error::Error for Diagnostic {}
pub fn blocked(code: &str, cause: &str, operation: &str, reason: &str) -> Diagnostic {
    Diagnostic {
        schema_version: 1,
        code: code.into(),
        cause: forge_adapters::process::redact(cause),
        operation: operation.into(),
        actions: vec![action(reason, None, true)],
        authority: "none",
    }
}
pub fn error_value(error: &anyhow::Error, operation: &str) -> Value {
    let diagnostic = error.downcast_ref::<Diagnostic>().cloned().unwrap_or_else(|| blocked(
        "diagnosis_unavailable", "No typed diagnosis is available for this failure", operation,
        "Inspect the redacted error and the command help before deciding the next operation"));
    json!({"schema_version":1,"status":"blocked","error_code":"operation_blocked",
        "error":forge_adapters::process::redact(&format!("{error:#}")),"diagnostic":diagnostic,"replayed":false})
}
pub fn human(value: &Value) -> Option<String> {
    let d = &value["diagnostic"];
    let cause = d["cause"].as_str()?;
    let actions = d["actions"]
        .as_array()?
        .iter()
        .filter_map(|a| a["reason"].as_str())
        .collect::<Vec<_>>()
        .join("; ");
    Some(format!(
        "{}: {cause}\nPróximo passo / Next step: {actions}",
        d["code"].as_str().unwrap_or("blocked")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_uses_typed_origin_not_error_words() {
        let plain = anyhow::anyhow!("source changed; active process; authentication required");
        assert_eq!(
            error_value(&plain, "test")["diagnostic"]["code"],
            "diagnosis_unavailable"
        );
        let typed = anyhow::Error::new(blocked(
            "scope_changed",
            "different contract",
            "test",
            "Review it",
        ))
        .context("outer context");
        assert_eq!(
            error_value(&typed, "test")["diagnostic"]["code"],
            "scope_changed"
        );
    }
}
