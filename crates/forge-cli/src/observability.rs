//! Runtime-only wiring; inspect/dry-run never initialize a transport.
use crate::engine::Context;
use anyhow::Result;
use forge_adapters::observability::{Phase, Session, Trace};
use forge_domain::TraceMode;
use serde_json::Value;
use std::future::Future;

pub async fn phase(
    c: &Context,
    dry: bool,
    phase: Phase,
    work: impl Future<Output = Result<Value>>,
) -> Result<Value> {
    if dry {
        return work.await;
    }
    if Trace::active() {
        return nested(phase, work).await;
    }
    let mut session = Session::from_env(
        c.manifest.automation.observability.langsmith == TraceMode::Optional,
        dry,
        &c.manifest.release_id,
    );
    let result = session.trace.scope(nested(phase, work)).await;
    let diagnostics = session.finish().await;
    // Diagnostics cannot turn a task failure into success, or make success fail.
    if c.manifest.automation.observability.langsmith == TraceMode::Optional {
        let _ = save_diagnostics(&c.db, &diagnostics);
    }
    result
}
// Never create/migrate a database or wait for an operational writer for telemetry.
fn save_diagnostics(path: &std::path::Path, diagnostics: &Value) -> rusqlite::Result<()> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.busy_timeout(std::time::Duration::ZERO)?;
    conn.execute("INSERT INTO metadata VALUES('observability',?1) ON CONFLICT(key) DO UPDATE SET body=excluded.body", [diagnostics.to_string()])?;
    Ok(())
}
pub async fn nested(phase: Phase, work: impl Future<Output = Result<Value>>) -> Result<Value> {
    let mut span = Trace::current().span(phase, None, 0);
    let result = span.child.scope(work).await;
    span.finish(
        result
            .as_ref()
            .ok()
            .and_then(|v| {
                v["run"]["status"]
                    .as_str()
                    .or_else(|| v["status"].as_str())
                    .or_else(|| (v["published"] == true).then_some("passed"))
            })
            .unwrap_or("failed"),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn diagnostics_do_not_wait_for_locks_or_create_missing_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.sqlite");
        assert!(save_diagnostics(&path, &json!({})).is_err());
        assert!(!path.exists());
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE metadata(key TEXT PRIMARY KEY, body TEXT); BEGIN EXCLUSIVE;",
        )
        .unwrap();
        let started = std::time::Instant::now();
        assert!(save_diagnostics(&path, &json!({})).is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        conn.execute_batch("ROLLBACK").unwrap();
    }
    #[tokio::test]
    async fn instrumentation_returns_original_errors_and_nonpassing_results() {
        let result = nested(Phase::Task, async {
            Err(anyhow::anyhow!("original failure"))
        })
        .await;
        assert_eq!(result.unwrap_err().to_string(), "original failure");
        let published = json!({"published":true,"url":"fixture"});
        assert_eq!(
            nested(Phase::Publish, async { Ok(published.clone()) })
                .await
                .unwrap(),
            published
        );
        for status in ["blocked", "cancelled", "awaiting_approval", "passed"] {
            let original = json!({"run":{"status":status}});
            assert_eq!(
                nested(Phase::Release, async { Ok(original.clone()) })
                    .await
                    .unwrap(),
                original
            );
        }
    }
}
