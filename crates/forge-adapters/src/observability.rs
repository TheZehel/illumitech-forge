//! Opt-in, content-free LangSmith transport. Telemetry is never execution evidence.
use chrono::Utc;
use reqwest::{
    Client, Url,
    header::{HeaderMap, HeaderValue},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

const LIMIT: usize = 128;
const TIMEOUT: Duration = Duration::from_secs(2);
#[derive(Clone, Default)]
pub struct Trace {
    sender: Option<mpsc::Sender<(bool, Value)>>,
    dropped: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
    parent: Option<String>,
    execution: String,
    release: String,
}
tokio::task_local! { static CURRENT: Trace; }

/// Owns the worker; dropping a cancelled session aborts pending I/O.
pub struct Session {
    pub trace: Trace,
    worker: Option<JoinHandle<()>>,
}
impl Session {
    pub fn from_env(optional: bool, dry: bool, release: &str) -> Self {
        if !optional || dry {
            return Self::disabled();
        }
        Self::configured(
            optional,
            dry,
            release,
            &std::env::var("LANGSMITH_TRACING").unwrap_or_default(),
            &std::env::var("LANGSMITH_API_KEY").unwrap_or_default(),
            &std::env::var("LANGSMITH_ENDPOINT")
                .unwrap_or_else(|_| "https://api.smith.langchain.com".into()),
            std::env::var("LANGSMITH_WORKSPACE_ID").ok().as_deref(),
        )
    }
    fn disabled() -> Self {
        Self {
            trace: Trace::default(),
            worker: None,
        }
    }
    fn configured(
        optional: bool,
        dry: bool,
        release: &str,
        enabled: &str,
        key: &str,
        endpoint: &str,
        tenant: Option<&str>,
    ) -> Self {
        if !optional || dry || enabled != "true" || key.trim().is_empty() {
            return Self::disabled();
        }
        let Some((client, endpoint)) = transport(endpoint, key, tenant) else {
            return Self::disabled();
        };
        let (tx, mut rx) = mpsc::channel::<(bool, Value)>(LIMIT);
        let trace = Trace {
            sender: Some(tx),
            execution: Uuid::new_v4().to_string(),
            release: opaque(release),
            ..Trace::default()
        };
        let failed = trace.failed.clone();
        let worker = tokio::spawn(async move {
            while let Some((update, body)) = rx.recv().await {
                let request = if update {
                    client.patch(format!(
                        "{endpoint}/runs/{}",
                        body["id"].as_str().unwrap_or_default()
                    ))
                } else {
                    client.post(format!("{endpoint}/runs"))
                };
                let result = request.json(&body).send().await;
                if !result.is_ok_and(|r| r.status().is_success()) {
                    failed.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        Self {
            trace,
            worker: Some(worker),
        }
    }
    /// One bounded drain for the entire session, never per span. No error propagates.
    pub async fn finish(&mut self) -> Value {
        self.trace.sender.take();
        if let Some(mut worker) = self.worker.take()
            && tokio::time::timeout(TIMEOUT, &mut worker).await.is_err()
        {
            worker.abort();
            self.trace.failed.fetch_add(1, Ordering::Relaxed);
        }
        self.trace.diagnostics()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(w) = &self.worker {
            w.abort();
        }
    }
}

fn transport(endpoint: &str, key: &str, tenant: Option<&str>) -> Option<(Client, String)> {
    let url = Url::parse(endpoint).ok()?;
    if url.scheme() != "https"
        && !(cfg!(test)
            && url.scheme() == "http"
            && url.host_str().is_some_and(|h| {
                h.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            }))
    {
        return None;
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let mut headers = HeaderMap::new();
    let mut secret = HeaderValue::from_str(key).ok()?;
    secret.set_sensitive(true);
    headers.insert("x-api-key", secret);
    if let Some(tenant) = tenant {
        headers.insert("x-tenant-id", HeaderValue::from_str(tenant).ok()?);
    }
    let client = Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT)
        .build()
        .ok()?;
    Some((client, url.as_str().trim_end_matches('/').into()))
}
// Hash all operator-supplied IDs: even a syntactically valid ID can be a secret.
fn opaque(value: &str) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(value.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
impl Trace {
    pub fn current() -> Self {
        CURRENT.try_with(Clone::clone).unwrap_or_default()
    }
    pub fn active() -> bool {
        CURRENT.try_with(|_| ()).is_ok()
    }
    pub async fn scope<T>(&self, future: impl Future<Output = T>) -> T {
        CURRENT.scope(self.clone(), future).await
    }
    pub fn diagnostics(&self) -> Value {
        json!({"dropped":self.dropped.load(Ordering::Relaxed), "failed":self.failed.load(Ordering::Relaxed)})
    }
    pub fn span(&self, phase: Phase, task: Option<&str>, attempt: u32) -> Span {
        let id = Uuid::new_v4().to_string();
        let mut child = self.clone();
        child.parent = Some(id.clone());
        let metadata = json!({"execution_id":self.execution,"release_id":self.release,"task_id":task.map(opaque),"attempt":attempt,"dry_run":false,"phase":phase.name()});
        let body = json!({"id":id,"name":phase.name(),"run_type":"chain","start_time":Utc::now().to_rfc3339(),"parent_run_id":self.parent,"inputs":{},"extra":{"metadata":metadata}});
        let queued = self
            .sender
            .as_ref()
            .is_some_and(|s| s.try_send((false, body.clone())).is_ok());
        if self.sender.is_some() && !queued {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        Span {
            queued,
            trace: self.clone(),
            child,
            body,
            started: Instant::now(),
            outcome: "cancelled",
        }
    }
}
#[derive(Clone, Copy)]
pub enum Phase {
    DaemonCycle,
    Release,
    Task,
    ValidationGates,
    Verification,
    Publish,
}
impl Phase {
    fn name(self) -> &'static str {
        match self {
            Self::DaemonCycle => "forge.daemon_cycle",
            Self::Release => "forge.release",
            Self::Task => "forge.task",
            Self::ValidationGates => "forge.validation_gates",
            Self::Verification => "forge.verification",
            Self::Publish => "forge.publish",
        }
    }
}
pub struct Span {
    queued: bool,
    trace: Trace,
    pub child: Trace,
    body: Value,
    started: Instant,
    outcome: &'static str,
}
impl Span {
    pub fn finish(&mut self, status: &str) {
        self.outcome = match status {
            "passed" | "evaluating" => "passed",
            "cancelled" => "cancelled",
            "paused" => "paused",
            "awaiting_approval" => "awaiting_approval",
            _ => "failed",
        };
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        if !self.queued {
            return;
        }
        let Some(sender) = &self.trace.sender else {
            return;
        };
        self.body["end_time"] = json!(Utc::now().to_rfc3339());
        let metadata = &mut self.body["extra"]["metadata"];
        metadata["duration_ms"] =
            json!(self.started.elapsed().as_millis().min(u64::MAX as u128) as u64);
        metadata["outcome"] = json!(self.outcome);
        if self.outcome != "passed" {
            self.body["error"] = json!(self.outcome);
        }
        // Fixed typed metadata has no arbitrary key/value path; defend the total limit too.
        if self.body["extra"]["metadata"].to_string().len() > 4096
            || sender.try_send((true, self.body.take())).is_err()
        {
            self.trace.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn fake(response: String) -> (String, mpsc::Receiver<(String, Value)>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel(256);
        let worker = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let split = loop {
                    let byte = socket.read_u8().await.unwrap();
                    bytes.push(byte);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break bytes.len();
                    }
                };
                let headers = String::from_utf8(bytes.clone()).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                bytes.resize(split + length, 0);
                socket.read_exact(&mut bytes[split..]).await.unwrap();
                tx.send((
                    headers.lines().next().unwrap().into(),
                    serde_json::from_slice(&bytes[split..]).unwrap(),
                ))
                .await
                .unwrap();
                if response.is_empty() {
                    std::future::pending::<()>().await;
                }
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (endpoint, rx, worker)
    }
    fn fake_session(endpoint: &str) -> Session {
        Session::configured(
            true,
            false,
            "release-private",
            "true",
            "fixture-key",
            endpoint,
            None,
        )
    }
    fn response(code: &str) -> String {
        format!("HTTP/1.1 {code}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    }

    #[tokio::test]
    async fn hierarchy_attempts_redaction_and_failure_are_preserved() {
        let (endpoint, mut received, server) = fake(response("200 OK")).await;
        let mut session = fake_session(&endpoint);
        let mut root = session.trace.span(Phase::Release, None, 0);
        let root_id = root.body["id"].clone();
        let execution = session.trace.execution.clone();
        let result: Result<(), &'static str> = root
            .child
            .scope(async {
                for attempt in 1..=2 {
                    let mut task = Trace::current().span(
                        Phase::Task,
                        Some("token=secret /private/path"),
                        attempt,
                    );
                    task.finish("failed");
                }
                Err("private task error")
            })
            .await;
        assert_eq!(result, Err("private task error"));
        root.finish("failed");
        drop(root);
        assert_eq!(session.finish().await, json!({"dropped":0,"failed":0}));
        let mut events = Vec::new();
        while let Ok(event) = received.try_recv() {
            events.push(event);
        }
        assert_eq!(events.len(), 6);
        assert!(events[0].0.starts_with("POST /runs "));
        assert!(events[5].0.starts_with("PATCH /runs/"));
        assert_eq!(events[1].1["parent_run_id"], root_id);
        assert_eq!(events[3].1["parent_run_id"], root_id);
        assert_ne!(events[1].1["id"], events[3].1["id"]);
        assert_eq!(events[2].1["extra"]["metadata"]["attempt"], 1);
        assert_eq!(events[4].1["extra"]["metadata"]["attempt"], 2);
        for (_, event) in &events {
            let metadata = &event["extra"]["metadata"];
            assert_eq!(metadata["execution_id"], execution);
            assert!(metadata.to_string().len() < 4096);
            for value in metadata.as_object().unwrap().values() {
                if let Some(s) = value.as_str() {
                    assert!(s.len() <= 256);
                }
            }
            assert_eq!(event["inputs"], json!({}));
            for forbidden in ["secret", "private", "fixture-key", "stdout", "prompt"] {
                assert!(!event.to_string().contains(forbidden));
            }
        }
        assert_eq!(events[5].1["error"], "failed");
        server.abort();
    }

    #[tokio::test]
    async fn six_phases_share_context_and_new_sessions_do_not_reuse_ids() {
        let (endpoint, mut received, server) = fake(response("200 OK")).await;
        let mut session = fake_session(&endpoint);
        let mut cycle = session.trace.span(Phase::DaemonCycle, None, 0);
        let cycle_id = cycle.body["id"].clone();
        let release_id = cycle
            .child
            .scope(async {
                let mut release = Trace::current().span(Phase::Release, None, 0);
                let id = release.body["id"].clone();
                release
                    .child
                    .scope(async {
                        for phase in [
                            Phase::Task,
                            Phase::ValidationGates,
                            Phase::Verification,
                            Phase::Publish,
                        ] {
                            let mut span = Trace::current().span(phase, Some("task"), 1);
                            span.finish("passed");
                        }
                    })
                    .await;
                release.finish("passed");
                id
            })
            .await;
        cycle.finish("passed");
        drop(cycle);
        assert_eq!(session.finish().await["failed"], 0);
        let mut posts = Vec::new();
        while let Ok((method, body)) = received.try_recv() {
            if method.starts_with("POST") {
                posts.push(body);
            }
        }
        assert_eq!(
            posts
                .iter()
                .map(|v| v["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "forge.daemon_cycle",
                "forge.release",
                "forge.task",
                "forge.validation_gates",
                "forge.verification",
                "forge.publish"
            ]
        );
        assert_eq!(posts[1]["parent_run_id"], cycle_id);
        assert!(posts[2..].iter().all(|v| v["parent_run_id"] == release_id));
        let second = fake_session(&endpoint);
        assert_ne!(session.trace.execution, second.trace.execution);
        assert!(!Trace::active());
        server.abort();
    }

    #[tokio::test]
    async fn off_dry_missing_credentials_and_false_never_connect() {
        let (endpoint, mut received, server) = fake(response("200 OK")).await;
        for (opt, dry, enabled, key) in [
            (false, false, "true", "key"),
            (true, true, "true", "key"),
            (true, false, "true", ""),
            (true, false, "false", "key"),
        ] {
            let mut session = Session::configured(opt, dry, "r", enabled, key, &endpoint, None);
            assert!(session.worker.is_none());
            drop(session.trace.span(Phase::Task, None, 1));
            session.finish().await;
        }
        assert!(received.try_recv().is_err());
        server.abort();
    }
    #[tokio::test]
    async fn errors_and_redirects_do_not_retry_or_forward_credentials() {
        let (target, mut target_rx, target_server) = fake(response("200 OK")).await;
        for code in [
            response("500 Broken"),
            format!(
                "HTTP/1.1 307 Temporary Redirect\r\nLocation: {target}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
        ] {
            let (endpoint, mut received, server) = fake(code).await;
            let mut session = fake_session(&endpoint);
            let mut span = session.trace.span(Phase::Task, None, 1);
            span.finish("failed");
            drop(span);
            assert_eq!(session.finish().await["failed"], 2);
            assert!(received.try_recv().is_ok());
            assert!(received.try_recv().is_ok());
            assert!(received.try_recv().is_err());
            server.abort();
        }
        assert!(target_rx.try_recv().is_err());
        target_server.abort();
    }
    #[tokio::test]
    async fn saturation_timeout_and_cancel_are_bounded() {
        let (endpoint, mut received, server) = fake(String::new()).await;
        let mut session = fake_session(&endpoint);
        let span = session.trace.span(Phase::Task, None, 1);
        received.recv().await.unwrap(); // Request is in flight, no scheduling assumptions.
        drop(span); // Unfinished operation is cancelled, never successful.
        for _ in 0..300 {
            drop(session.trace.span(Phase::Task, None, 1));
        }
        assert!(session.trace.diagnostics()["dropped"].as_u64().unwrap() > 0);
        let start = Instant::now();
        let diagnostics = session.finish().await;
        assert!(start.elapsed() < Duration::from_secs(3));
        assert!(diagnostics["failed"].as_u64().unwrap() > 0);
        server.abort();
    }
    #[tokio::test]
    async fn aborted_operation_emits_cancelled_and_drop_aborts_worker() {
        let (endpoint, mut received, server) = fake(response("200 OK")).await;
        let mut session = fake_session(&endpoint);
        let trace = session.trace.clone();
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _span = trace.span(Phase::Task, None, 1);
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        task.abort();
        let _ = task.await;
        session.finish().await;
        received.recv().await.unwrap();
        assert_eq!(received.recv().await.unwrap().1["error"], "cancelled");
        server.abort();
        let (endpoint, mut received, server) = fake(String::new()).await;
        let session = fake_session(&endpoint);
        let handle = session.worker.as_ref().unwrap().abort_handle();
        let _span = session.trace.span(Phase::Task, None, 1);
        received.recv().await.unwrap();
        drop(session);
        tokio::task::yield_now().await;
        assert!(handle.is_finished());
        server.abort();
    }
    #[test]
    fn transport_rejects_insecure_and_credential_bearing_urls() {
        for url in [
            "http://example.invalid",
            "ftp://127.0.0.1",
            "https://user:secret@example.invalid",
            "https://example.invalid?token=secret",
            "https://example.invalid/#fragment",
        ] {
            assert!(transport(url, "key", None).is_none());
        }
        assert!(transport("https://example.invalid", "bad\nkey", None).is_none());
    }
}
