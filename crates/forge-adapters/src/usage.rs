//! Parse native usage independently of exit status. No text-derived token estimates.
use forge_domain::usage::{TokenUsage, UsageObservation, UsageUnit};
use serde_json::Value;
use std::collections::BTreeMap;

fn count(v: &Value, name: &str) -> Option<u64> {
    v.get(name).and_then(Value::as_u64)
}
fn claude_usage(v: &Value, camel: bool) -> TokenUsage {
    let names = if camel {
        [
            "inputTokens",
            "cacheReadInputTokens",
            "cacheCreationInputTokens",
            "outputTokens",
        ]
    } else {
        [
            "input_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "output_tokens",
        ]
    };
    let read = count(v, names[1]);
    let input = count(v, names[0])
        .zip(read)
        .zip(count(v, names[2]))
        .and_then(|((i, r), w)| i.checked_add(r)?.checked_add(w));
    TokenUsage::new(input, read, count(v, names[3]))
}
fn codex_usage(v: &Value) -> TokenUsage {
    TokenUsage::new(
        count(v, "input_tokens"),
        count(v, "cached_input_tokens"),
        count(v, "output_tokens"),
    )
}
fn put(units: &mut BTreeMap<String, UsageUnit>, unit: UsageUnit, limitations: &mut Vec<String>) {
    if let Some(previous) = units.get(&unit.key) {
        let old = &previous.token_usage;
        let new = &unit.token_usage;
        // Cumulative updates may repeat, but a reset is not an extra spend or a delta.
        if [
            (old.input_tokens, new.input_tokens),
            (old.cached_input_tokens, new.cached_input_tokens),
            (old.output_tokens, new.output_tokens),
        ]
        .iter()
        .any(|(o, n)| o.zip(*n).is_some_and(|(o, n)| n < o))
        {
            limitations.push(
                "usage counter regressed; retained earlier snapshot, coverage incomplete".into(),
            );
            return;
        }
        if !unit.cumulative && old != new {
            limitations.push("conflicting duplicate turn usage; coverage incomplete".into());
            return;
        }
    }
    units.insert(unit.key.clone(), unit);
}

/// `fallback` identifies this invocation when the native protocol omits IDs.
/// Session totals replace turn snapshots, rather than being added to them.
pub fn observe(
    client: &str,
    stdout: &str,
    fallback: &str,
    transport_complete: bool,
) -> UsageObservation {
    let mut units = BTreeMap::new();
    let mut limitations = Vec::new();
    let mut session = fallback.to_string();
    let mut turn = "implicit-0".to_string();
    let mut ordinal = 0u64;
    let mut protocol_complete = true;
    let mut terminal_seen = false;
    let events: Vec<Value> = if client == "claude" {
        match serde_json::from_str(stdout) {
            Ok(v) => vec![v],
            Err(_) => {
                protocol_complete = false;
                vec![]
            }
        }
    } else {
        stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| match serde_json::from_str(line) {
                Ok(v) => Some(v),
                Err(_) => {
                    protocol_complete = false;
                    None
                }
            })
            .collect()
    };
    for e in events {
        terminal_seen |= matches!(
            e["type"].as_str(),
            Some("turn.completed" | "turn.failed" | "error" | "result")
        ) || e["method"] == "turn/completed";
        if client == "claude" {
            if e["type"] != "result" {
                continue;
            }
            session = e["session_id"].as_str().unwrap_or(fallback).into();
            // modelUsage covers the whole tree; result.usage is main-loop-only.
            if let Some(models) = e["modelUsage"].as_object().filter(|m| !m.is_empty()) {
                for (model, usage) in models {
                    let key = serde_json::json!([session, "session_model", model]).to_string();
                    put(
                        &mut units,
                        UsageUnit {
                            key,
                            session: session.clone(),
                            cumulative: true,
                            token_usage: claude_usage(usage, true),
                        },
                        &mut limitations,
                    );
                }
            } else if e["usage"].is_object() {
                let key = serde_json::json!([session, "main_loop"]).to_string();
                put(
                    &mut units,
                    UsageUnit {
                        key,
                        session: session.clone(),
                        cumulative: true,
                        token_usage: claude_usage(&e["usage"], false),
                    },
                    &mut limitations,
                );
                limitations.push("Claude result.usage covers the main loop only; subagent usage unavailable without modelUsage".into());
            }
            continue;
        }
        match e["type"].as_str() {
            Some("thread.started") => {
                session = e["thread_id"].as_str().unwrap_or(fallback).into();
                ordinal = 0;
                turn = "implicit-0".into();
            }
            Some("turn.started") => {
                ordinal += 1;
                turn = e["turn_id"]
                    .as_str()
                    .or_else(|| e["turn"]["id"].as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("turn-{ordinal}"));
            }
            Some("turn.completed" | "turn.failed" | "error") if e["usage"].is_object() => {
                let id = e["turn_id"].as_str().unwrap_or(&turn);
                let key = serde_json::json!([session, "turn", id]).to_string();
                put(
                    &mut units,
                    UsageUnit {
                        key,
                        session: session.clone(),
                        cumulative: false,
                        token_usage: codex_usage(&e["usage"]),
                    },
                    &mut limitations,
                );
            }
            Some("turn.completed" | "turn.failed") => {
                limitations.push("terminal turn omitted usage; full consumption unknown".into());
            }
            _ => {}
        }
        // Recognize cumulative App Server snapshots if captured by a caller. Forge
        // still launches exec; it does not enable an App Server or make extra calls.
        if e["method"] == "thread/tokenUsage/updated" {
            let p = &e["params"];
            let s = p["threadId"].as_str().unwrap_or(&session).to_string();
            let total = &p["tokenUsage"]["total"];
            if total.is_object() {
                let key = serde_json::json!([s, "session"]).to_string();
                let usage = TokenUsage::new(
                    count(total, "inputTokens"),
                    count(total, "cachedInputTokens"),
                    count(total, "outputTokens"),
                );
                put(
                    &mut units,
                    UsageUnit {
                        key,
                        session: s,
                        cumulative: true,
                        token_usage: usage,
                    },
                    &mut limitations,
                );
            }
        }
    }
    let sessions: std::collections::BTreeSet<_> = units
        .values()
        .filter(|u| u.cumulative)
        .map(|u| u.session.clone())
        .collect();
    units.retain(|_, u| u.cumulative || !sessions.contains(&u.session));
    if units.is_empty() {
        limitations
            .push("native client did not report supported usage; consumption unknown".into());
    }
    if !terminal_seen && !units.is_empty() {
        limitations.push("no terminal native result; snapshots are partial observations".into());
    }
    if !transport_complete || !protocol_complete {
        limitations.push("incomplete transport/protocol; observed usage is a lower bound".into());
    }
    let complete =
        transport_complete && protocol_complete && limitations.is_empty() && !units.is_empty();
    UsageObservation {
        units: units.into_values().collect(),
        complete,
        limitations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usage_updates_are_not_double_counted() {
        let event = r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":70,"output_tokens":5,"reasoning_output_tokens":3}}"#;
        let r = observe(
            "codex",
            &format!("{{\"type\":\"thread.started\",\"thread_id\":\"s\"}}\n{event}\n{event}"),
            "call",
            true,
        );
        assert!(r.complete);
        assert_eq!(r.units.len(), 1);
        assert_eq!(r.units[0].token_usage.total_tokens, Some(105));
        let r = observe(
            "codex",
            &format!("{event}\n{{\"type\":\"turn.started\"}}\n{event}"),
            "call",
            true,
        );
        assert_eq!(r.units.len(), 2);
        let updates = [10, 10, 20].map(|n| serde_json::json!({"method":"thread/tokenUsage/updated","params":{"threadId":"s","tokenUsage":{"total":{"inputTokens":n,"cachedInputTokens":0,"outputTokens":2}}}}).to_string()).join("\n");
        let r = observe("codex", &updates, "call", true);
        assert_eq!(r.units.len(), 1);
        assert_eq!(r.units[0].token_usage.total_tokens, Some(22));
    }
    #[test]
    fn missing_malformed_failed_and_cancelled_usage_stays_honest() {
        assert!(observe("codex", "", "call", true).units.is_empty());
        let r = observe(
            "codex",
            "{\"type\":\"turn.failed\",\"usage\":{\"input_tokens\":40,\"output_tokens\":2}}\n{bad",
            "call",
            false,
        );
        assert!(!r.complete);
        assert_eq!(r.units[0].token_usage.cached_input_tokens, None);
        assert_eq!(r.units[0].token_usage.total_tokens, Some(42));
        let r = observe(
            "codex",
            r#"{"type":"turn.completed","usage":{"input_tokens":-1,"cached_input_tokens":3,"output_tokens":"5"}}"#,
            "call",
            true,
        );
        assert_eq!(r.units[0].token_usage.total_tokens, None);
    }
    #[test]
    fn claude_cache_is_included_once_and_model_totals_replace_main_loop() {
        let r = observe(
            "claude",
            r#"{"type":"result","session_id":"c","is_error":true,"usage":{"input_tokens":999,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"output_tokens":999},"modelUsage":{"selected":{"inputTokens":10,"cacheReadInputTokens":30,"cacheCreationInputTokens":2,"outputTokens":5}}}"#,
            "call",
            true,
        );
        assert_eq!(r.units.len(), 1);
        assert_eq!(
            r.units[0].token_usage,
            TokenUsage::new(Some(42), Some(30), Some(5))
        );
    }
}
