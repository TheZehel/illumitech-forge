//! Local, project-authorized workbench. Native approvals never become Forge acceptance.
use crate::{guided, native::Native};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::{Mutex, RwLock};

#[derive(Clone, Default, Serialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub events: VecDeque<Value>,
    pub active_turn: Option<String>,
    pub usage: Value,
    pub effective: Value,
    pub loaded: bool,
    #[serde(skip)]
    pub budget_call: Option<Arc<Mutex<crate::budget::Reservation>>>,
    #[serde(skip)]
    pub budget_operation: Option<String>,
}
impl Session {
    fn push(&mut self, event: Value) {
        let event = display_event(event);
        self.events.push_back(event);
        while self.events.len() > 256
            || self
                .events
                .iter()
                .map(|e| e.to_string().len())
                .sum::<usize>()
                > 256 * 1024
        {
            self.events.pop_front();
        }
    }
}
// Keep tool identity/status when a large payload exceeds the bridge display budget.
// This projection never changes the native request or approval payload.
fn display_event(mut event: Value) -> Value {
    if serde_json::to_vec(&event).is_ok_and(|b| b.len() <= 65536) {
        return event;
    }
    fn compact(value: &Value, budget: &mut usize, depth: usize) -> Value {
        if *budget == 0 || depth > 8 {
            return json!("… Conteúdo omitido pelo limite de exibição.");
        }
        match value {
            Value::String(s) => {
                let end = s.floor_char_boundary((*budget).min(8000).min(s.len()));
                *budget = budget.saturating_sub(end);
                if end < s.len() {
                    json!(format!("{}\n… Conteúdo limitado.", &s[..end]))
                } else {
                    value.clone()
                }
            }
            Value::Array(a) => {
                let mut result = Vec::new();
                for v in a.iter().take(32) {
                    if *budget == 0 {
                        break;
                    }
                    *budget = budget.saturating_sub(16);
                    result.push(compact(v, budget, depth + 1));
                }
                if result.len() < a.len() {
                    result.push(json!("… Itens omitidos pelo limite de exibição."));
                }
                result.into()
            }
            Value::Object(o) => {
                let mut result = serde_json::Map::new();
                for (k, v) in o.iter().take(32) {
                    if *budget < k.len() + 16 {
                        break;
                    }
                    *budget -= k.len() + 16;
                    result.insert(k.clone(), compact(v, budget, depth + 1));
                }
                Value::Object(result)
            }
            _ => {
                *budget = budget.saturating_sub(16);
                value.clone()
            }
        }
    }
    // The content budget must not decide whether an error or file identity exists.
    // Restore these fields from the original, even if compact() returned a marker.
    fn preserve_fields(target: &mut Value, original: &Value, fields: &[&str]) {
        for field in fields {
            if let Some(value) = original.get(*field) {
                if !target.is_object() {
                    *target = json!({"forgeContentOmitted":target.take()});
                }
                target[*field] = value.clone();
            }
        }
    }
    fn project(original: &Value, mut budget: usize) -> Value {
        let mut item = original.clone();
        // Reserve file identity independently of all output/diff allocation. Keep
        // the native order and original count; omitted files are not fake rows.
        let files = original["changes"].as_array().map(|files| {
            files
                .iter()
                .filter(|file| file.is_object())
                .take(32)
                .map(|file| {
                    let mut metadata = json!({});
                    preserve_fields(&mut metadata, file, &["id", "path", "kind"]);
                    (metadata, file)
                })
                .collect::<Vec<_>>()
        });
        for key in [
            "aggregatedOutput",
            "result",
            "arguments",
            "contentItems",
            "text",
            "error",
        ] {
            if !original[key].is_null() {
                item[key] = compact(&original[key], &mut budget, 0);
            }
        }
        preserve_fields(&mut item["result"], &original["result"], &["isError"]);
        preserve_fields(&mut item["error"], &original["error"], &["code", "type"]);
        if let Some(files) = files {
            item["forgeOriginalFileCount"] = original["changes"].as_array().unwrap().len().into();
            item["changes"] = files
                .into_iter()
                .map(|(mut metadata, file)| {
                    if !file["diff"].is_null() {
                        metadata["diff"] = compact(&file["diff"], &mut budget, 0);
                    }
                    metadata
                })
                .collect();
        }
        item["forgeDisplayTruncated"] = true.into();
        item
    }
    if event["params"]["item"].is_object() {
        let original = event["params"]["item"].take();
        // JSON escaping can exceed the byte limit despite the text budget. In
        // that case retain metadata, rather than losing it to a content fallback.
        for budget in [20000, 0] {
            event["params"]["item"] = project(&original, budget);
            if serde_json::to_vec(&event).is_ok_and(|b| b.len() <= 65536) {
                return event;
            }
        }
    }
    json!({"method":"bridge/truncated","params":{"reason":"Native event exceeds display limit; inspect locally"}})
}
#[derive(Default)]
pub struct View {
    pub sessions: BTreeMap<String, Session>,
    pub approvals: BTreeMap<String, Value>,
    pub connected: bool,
    pub error: Option<String>,
    pub account: Value,
    pub handshake: Value,
}
pub struct App {
    _state_access: forge_domain::StateAccess,
    pub root: PathBuf,
    pub state: PathBuf,
    pub(crate) executable: Option<PathBuf>,
    pub(crate) manifest: Option<PathBuf>,
    pub native: RwLock<Option<Arc<Native>>>,
    pub view: Mutex<View>,
    pub control: Mutex<()>,
    epoch: AtomicU64,
    pub(crate) forge_control: Mutex<()>,
    pub(crate) forge_run: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub(crate) workflow_active: Mutex<Option<(PathBuf, String)>>,
    pub(crate) release_id: Option<String>,
    pub(crate) allowed_roots: std::collections::BTreeSet<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub operation_id: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub request_id: Option<Value>,
    #[serde(default)]
    pub decision: Option<Value>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub request: Option<String>,
}
impl App {
    pub fn new(
        root: &Path,
        state: &Path,
        executable: Option<PathBuf>,
        manifest: Option<PathBuf>,
    ) -> Result<Arc<Self>> {
        let root = guided::root(root)?;
        let state = guided::external(state, &root)?;
        let mut release_id = None;
        let mut allowed_roots = std::collections::BTreeSet::from([root.clone()]);
        if let Some(path) = &manifest {
            let path = guided::external(path, &root)?;
            let c = crate::engine::Context::load(&path, &state)?;
            release_id = Some(c.manifest.release_id.clone());
            ensure!(
                c.roots.values().any(|r| r == &root),
                "selected root must belong to the operator-selected manifest"
            );
            allowed_roots.extend(c.roots.values().cloned());
        }
        std::fs::create_dir_all(&state)?;
        let state_db = state.join("state.sqlite");
        guided::safe_path(&state_db)?;
        let _ = forge_domain::Store::open(&state_db, false)?;
        let state_access = forge_domain::StateAccess::acquire(&state_db, false, false)?;
        let dbpath = state.join("workbench.sqlite");
        guided::safe_path(&dbpath)?;
        let db = rusqlite::Connection::open(dbpath)?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS sessions(root TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(root,id)); CREATE TABLE IF NOT EXISTS operations(root TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(root,id));")?;
        // Additive migration: never persist prompts, native history or credentials as names.
        let has_title = db
            .prepare("PRAGMA table_info(sessions)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|c| c == "title");
        if !has_title {
            db.execute_batch("ALTER TABLE sessions ADD COLUMN title TEXT NOT NULL DEFAULT 'Conversa recuperada';")?;
        }
        let mut sessions = BTreeMap::new();
        let mut stmt =
            db.prepare("SELECT id,title FROM sessions WHERE root=?1 ORDER BY rowid LIMIT 200")?;
        for row in stmt.query_map([root.to_string_lossy()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (id, title) = row?;
            sessions.insert(
                id.clone(),
                Session {
                    id: id.clone(),
                    title,
                    ..Session::default()
                },
            );
        }
        Ok(Arc::new(Self {
            _state_access: state_access,
            root,
            state,
            executable,
            manifest,
            native: RwLock::new(None),
            view: Mutex::new(View {
                sessions,
                ..View::default()
            }),
            control: Mutex::new(()),
            epoch: AtomicU64::new(0),
            forge_control: Mutex::new(()),
            forge_run: Mutex::new(None),
            workflow_active: Mutex::new(None),
            release_id,
            allowed_roots,
        }))
    }
    pub async fn connect(self: &Arc<Self>) -> Result<()> {
        let epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut view = self.view.lock().await;
            view.connected = false;
            view.approvals.clear();
            for s in view.sessions.values_mut() {
                s.loaded = false;
                s.budget_call = None;
            }
        }
        if let Some(old) = self.native.write().await.take() {
            old.shutdown().await;
        }
        let native = match Native::connect(self.executable.as_deref(), &self.root).await {
            Ok(n) => Arc::new(n),
            Err(e) => {
                self.view.lock().await.error = Some(e.to_string());
                return Err(e);
            }
        };
        let mut receiver = native.events.subscribe();
        let account = native
            .call("account/read", json!({"refreshToken":false}))
            .await?;
        let ty = account["account"]["type"].as_str();
        // No fallback to API-key inference, no credentials or email sent to the UI.
        let mut view = self.view.lock().await;
        view.connected = true;
        view.error = None;
        view.handshake = native.handshake.clone();
        view.account = json!({"type":ty,"authenticated":ty==Some("chatgpt"),"model":"native configuration (unchanged)"});
        drop(view);
        *self.native.write().await = Some(native);
        let app = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let Some(app) = app.upgrade() else { break };
                        app.event(event, epoch).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        if let Some(app) = app.upgrade() {
                            let mut v = app.view.lock().await;
                            v.connected = false;
                            v.error=Some("Event buffer overflow; interrupt/reconnect explicitly; no requests replayed".into());
                        }
                        break;
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(())
    }
    async fn event(&self, event: Value, epoch: u64) {
        let mut v = self.view.lock().await;
        if self.epoch.load(Ordering::SeqCst) != epoch {
            return;
        }
        if event["method"] == "bridge/disconnected" {
            v.connected = false;
            v.approvals.clear();
            for s in v.sessions.values_mut() {
                s.budget_call = None;
            }
            v.error = Some(
                "Native disconnected. Outcome may be unknown; reconnect and resume explicitly."
                    .into(),
            );
            return;
        }
        let Some(thread) = event["params"]["threadId"].as_str() else {
            if event.get("id").is_some() {
                v.connected = false;
                v.error=Some("Native account/capability callback is not supported here. Use the managed native client and reconnect; no credentials were requested or sent.".into());
            } else if event["method"] == "error" {
                v.error=Some("Native client reported an unscoped error; inspect the native client. No acceptance was inferred.".into());
            }
            return;
        };
        if !v.sessions.contains_key(thread) {
            return;
        }
        let method = event["method"].as_str().unwrap_or("");
        if let Some(id) = event.get("id") {
            if v.approvals.len() >= 64 {
                v.connected = false;
                v.error = Some(
                    "Approval buffer full; no decisions sent. Interrupt/reconnect explicitly."
                        .into(),
                );
                return;
            }
            v.approvals.insert(id.to_string(), event.clone());
        }
        if method == "serverRequest/resolved" {
            v.approvals
                .remove(&event["params"]["requestId"].to_string());
        }
        let s = v.sessions.get_mut(thread).unwrap();
        if method == "turn/started" {
            s.active_turn = event["params"]["turn"]["id"].as_str().map(str::to_owned);
        }
        if method == "turn/completed" {
            if let Some(call) = s.budget_call.take()
                && let Err(e) = call.lock().await.finish(
                    &s.usage["last"],
                    event["params"]["turn"]["status"]
                        .as_str()
                        .unwrap_or("unknown"),
                )
            {
                s.push(json!({"method":"bridge/budget-error","params":{"error":e.to_string()}}));
            }
            s.active_turn = None;
        }
        if method == "thread/tokenUsage/updated" {
            s.usage = event["params"]["tokenUsage"].clone();
        }
        s.push(sanitize(event));
        while v
            .sessions
            .values()
            .flat_map(|s| s.events.iter())
            .map(|e| e.to_string().len())
            .sum::<usize>()
            > 8 * 1024 * 1024
        {
            if let Some(s) = v.sessions.values_mut().find(|s| !s.events.is_empty()) {
                s.events.pop_front();
            } else {
                break;
            }
        }
    }
    pub fn forge(&self) -> Result<Value> {
        let Some(path) = &self.manifest else {
            return Ok(
                json!({"status":"unavailable","reason":"Start workbench with --manifest PATH to inspect an external manifest for this authorized project. No manifest was auto-discovered."}),
            );
        };
        let _ = path;
        let c = self.inspection_context()?;
        let operation_active = self
            .forge_run
            .try_lock()
            .map(|h| h.as_ref().is_some_and(|h| !h.is_finished()))
            .unwrap_or(true);
        let mut value = json!({"execution_allowed":!c.manifest.automation.enabled && c.manifest.automation.publish.is_none(),"independent_review":crate::review::latest(&c)?,"allowed_roots":self.allowed_roots,"control_revision":self.control_revision(&c)?,"operation_active":operation_active,"inspection":c.inspect()?,"operational_state":crate::engine::status(&c,true)?,"budget":crate::budget::inspect(&c)?,"review":self.review(&c)?});
        value["control_journal"] = self.control_journal()?;
        Ok(sanitize(value))
    }

    pub async fn snapshot(&self) -> Value {
        let v = self.view.lock().await;
        sanitize(
            json!({"schema_version":1,"project":{"name":self.root.file_name().unwrap_or_default().to_string_lossy()},"connected":v.connected,"error":v.error,"account":v.account,"protocol":v.handshake,"sessions":v.sessions.values().collect::<Vec<_>>(),"approvals":v.approvals.values().map(|e|sanitize(e.clone())).collect::<Vec<_>>(),"forge":{"status":"not_evaluated","reason":"Native chat and tool approvals do not accept a Forge revision. Open a manifest to inspect external evidence."}}),
        )
    }
    pub(crate) fn reserve(&self, id: &str) -> Result<()> {
        ensure!(forge_domain::valid_id(id), "invalid operation ID");
        let db =
            rusqlite::Connection::open(guided::safe_path(&self.state.join("workbench.sqlite"))?)?;
        ensure!(
            db.execute(
                "INSERT OR IGNORE INTO operations(root,id) VALUES(?1,?2)",
                rusqlite::params![self.root.to_string_lossy(), id]
            )? == 1,
            "operation already submitted; outcome may be unknown; never replay automatically"
        );
        Ok(())
    }
    pub async fn action(self: &Arc<Self>, route: &str, a: Action) -> Result<Value> {
        let _control = self.control.lock().await;
        self.reserve(&a.operation_id)?;
        if route == "feature-preview" || route == "feature-apply" {
            return guided::feature(
                &self.root,
                a.id.as_deref().context("feature ID required")?,
                a.title.as_deref().context("title required")?,
                &self.state.join("workbench-candidates"),
                route == "feature-apply",
            );
        }
        if route == "planning-preview" || route == "planning-propose" {
            let reservation = if route == "planning-propose" && self.manifest.is_some() {
                Some(crate::budget::Reservation::begin(
                    &self.forge_context()?,
                    &format!("planning:{}", a.operation_id),
                    "planning",
                )?)
            } else {
                None
            };
            let mut result = crate::planning::propose_reserved(
                &self.root,
                a.request.as_deref().context("request required")?,
                &self.state.join("workbench-candidates"),
                &self
                    .state
                    .join("workbench-candidates/plans")
                    .join(format!("{}.json", a.operation_id)),
                route == "planning-preview",
                self.executable.as_deref(),
                None,
                reservation,
            )
            .await?;
            result["operation_id"] = json!(a.operation_id);
            return Ok(result);
        }
        if route == "reconnect" {
            self.connect().await?;
            return Ok(json!({"status":"connected","replayed":false}));
        }
        let n = self
            .native
            .read()
            .await
            .clone()
            .context("native disconnected")?;
        ensure!(
            self.view.lock().await.connected,
            "native disconnected; reconnect explicitly"
        );
        if route == "sessions" {
            let mut v = self.view.lock().await;
            ensure!(v.sessions.len() < 200, "session limit reached");
            drop(v);
            let r = n
                .call(
                    "thread/start",
                    json!({"cwd":self.root,"sandbox":"read-only","approvalPolicy":"on-request"}),
                )
                .await?;
            ensure!(
                r["cwd"].as_str() == self.root.to_str()
                    && r["thread"]["cwd"].as_str() == self.root.to_str(),
                "native project mismatch"
            );
            let id = r["thread"]["id"]
                .as_str()
                .context("missing native thread ID")?
                .to_owned();
            let title = format!("Conversa {}", self.view.lock().await.sessions.len() + 1);
            rusqlite::Connection::open(guided::safe_path(&self.state.join("workbench.sqlite"))?)?
                .execute(
                "INSERT OR IGNORE INTO sessions(root,id,title) VALUES(?1,?2,?3)",
                rusqlite::params![self.root.to_string_lossy(), id, title],
            )?;
            v = self.view.lock().await;
            v.sessions.insert(id.clone(),Session{id:id.clone(),title,loaded:true,effective:json!({"model":r["model"],"approvalPolicy":r["approvalPolicy"],"sandbox":r["sandbox"]}),..Session::default()});
            return Ok(json!({"thread_id":id}));
        }
        let id = a.thread_id.context("thread_id required")?;
        ensure!(
            self.view.lock().await.sessions.contains_key(&id),
            "session not authorized for this project"
        );
        match route {
            "resume" => {
                let mut r=n.call("thread/resume",json!({"threadId":id,"cwd":self.root,"sandbox":"read-only","approvalPolicy":"on-request"})).await?;
                ensure!(
                    r["thread"]["cwd"].as_str() == self.root.to_str(),
                    "session project mismatch"
                );
                if !r["thread"]["turns"].is_array() {
                    let page=n.call("thread/turns/list",json!({"threadId":id,"limit":40,"sortDirection":"asc","itemsView":"full"})).await?;
                    r["thread"]["turns"] = page["data"].clone();
                }
                let mut v = self.view.lock().await;
                let s = v.sessions.get_mut(&id).unwrap();
                s.events.clear();
                s.loaded = true;
                s.active_turn = r["thread"]["turns"]
                    .as_array()
                    .and_then(|turns| turns.iter().rev().find(|t| t["status"] == "inProgress"))
                    .and_then(|t| t["id"].as_str())
                    .map(str::to_owned);
                if let Some(turns) = r["thread"]["turns"].as_array() {
                    for turn in turns.iter().rev().take(40).rev() {
                        if let Some(items) = turn["items"].as_array() {
                            for item in items.iter().take(100) {
                                s.push(sanitize(
                                    json!({"method":"item/completed","params":{"item":item}}),
                                ));
                            }
                        }
                        // Native history contains terminal status separately from items.
                        // Rebuild it on explicit resume; never invent completion for an
                        // active, unknown or incomplete historical record.
                        if turn["id"].as_str().is_some_and(|id| !id.is_empty())
                            && matches!(
                                turn["status"].as_str(),
                                Some("completed" | "failed" | "interrupted")
                            )
                        {
                            s.push(sanitize(json!({"method":"turn/completed","params":{
                                "threadId":id,"turn":{"id":turn["id"],"status":turn["status"],"error":turn["error"]}
                            }})));
                        }
                    }
                }
                s.effective = json!({"model":r["model"],"approvalPolicy":r["approvalPolicy"],"sandbox":r["sandbox"]});
                Ok(json!({"status":"resumed"}))
            }
            "turn" => {
                ensure!(
                    self.view.lock().await.sessions[&id].loaded,
                    "saved session must be explicitly resumed before sending"
                );
                ensure!(
                    self.view.lock().await.account["authenticated"] == true,
                    "Native ChatGPT login required; authenticate with the native client. No API fallback."
                );
                let text = a.text.context("text required")?;
                ensure!(
                    !text.trim().is_empty() && text.len() <= 32768,
                    "message must contain 1..32768 bytes"
                );
                let reservation = if self.manifest.is_some() {
                    Some(Arc::new(Mutex::new(crate::budget::Reservation::begin(
                        &self.forge_context()?,
                        &format!("chat:{}", a.operation_id),
                        "chat",
                    )?)))
                } else {
                    None
                };
                {
                    let mut v = self.view.lock().await;
                    let s = v.sessions.get_mut(&id).unwrap();
                    ensure!(
                        s.active_turn.is_none(),
                        "turn already active; interrupt before sending another"
                    );
                    s.budget_call = reservation;
                    s.budget_operation = Some(a.operation_id.clone());
                    s.usage = Value::Null;
                    s.active_turn = Some("pending".into());
                    s.events
                        .push_back(json!({"method":"bridge/user","params":{"text":text}}));
                }
                let deadline = {
                    let v = self.view.lock().await;
                    match &v.sessions[&id].budget_call {
                        Some(c) => c.lock().await.deadline_seconds,
                        None => None,
                    }
                };
                if let Some(seconds) = deadline {
                    let app = Arc::downgrade(self);
                    let session = id.clone();
                    let operation = a.operation_id.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
                        if let Some(app) = app.upgrade() {
                            let expired = {
                                let v = app.view.lock().await;
                                v.sessions.get(&session).is_some_and(|s| {
                                    s.budget_call.is_some()
                                        && s.budget_operation.as_deref() == Some(&operation)
                                })
                            };
                            if expired {
                                if let Some(n) = app.native.read().await.clone() {
                                    n.shutdown().await;
                                }
                                let mut v = app.view.lock().await;
                                v.connected = false;
                                v.error=Some("Native active-time budget reached; connection stopped without replay".into());
                                if let Some(s) = v.sessions.get_mut(&session) {
                                    s.budget_call = None;
                                }
                            }
                        }
                    });
                }
                let r=n.call("turn/start",json!({"threadId":id,"input":[{"type":"text","text":text,"text_elements":[]}]})).await;
                if r.is_err() {
                    self.view.lock().await.error=Some("Turn result unknown/rejected. Inspect session before another action; no automatic replay.".into());
                }
                r
            }
            "interrupt" => {
                let turn = self.view.lock().await.sessions[&id]
                    .active_turn
                    .clone()
                    .context("no active turn")?;
                ensure!(
                    turn != "pending",
                    "turn ID unknown; reconnect and inspect native session"
                );
                n.call("turn/interrupt", json!({"threadId":id,"turnId":turn}))
                    .await
            }
            "approval" => {
                let request = a.request_id.context("request_id required")?;
                let event = self
                    .view
                    .lock()
                    .await
                    .approvals
                    .get(&request.to_string())
                    .cloned()
                    .context("approval no longer pending")?;
                ensure!(
                    event["params"]["threadId"] == id,
                    "approval project/session mismatch"
                );
                let decision = a.decision.context("decision required")?;
                let choices = approval_choices(&event);
                ensure!(
                    choices.contains(&decision),
                    "decision is not an available native choice"
                );
                let result = if event["method"] == "item/permissions/requestApproval" {
                    decision.clone()
                } else {
                    json!({"decision":decision})
                };
                n.write(json!({"id":request,"result":result})).await?;
                self.view
                    .lock()
                    .await
                    .approvals
                    .remove(&request.to_string());
                self.event(json!({"method":"bridge/decision","params":{"threadId":id,"decision":decision}}),self.epoch.load(Ordering::SeqCst)).await;
                Ok(json!({"status":"submitted","forge_acceptance":false}))
            }
            _ => bail!("unsupported action"),
        }
    }
}
pub fn approval_choices(event: &Value) -> Vec<Value> {
    if let Some(v) = event["params"]["availableDecisions"].as_array() {
        return v.clone();
    }
    match event["method"].as_str().unwrap_or("") {
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            let mut choices = vec![
                json!("accept"),
                json!("acceptForSession"),
                json!("decline"),
                json!("cancel"),
            ];
            if event["method"] == "item/commandExecution/requestApproval" {
                if event["params"]["proposedExecpolicyAmendment"].is_array() {
                    choices.push(json!({"acceptWithExecpolicyAmendment":{"execpolicy_amendment":event["params"]["proposedExecpolicyAmendment"]}}));
                }
                if let Some(amendments) =
                    event["params"]["proposedNetworkPolicyAmendments"].as_array()
                {
                    for amendment in amendments {
                        choices.push(json!({"applyNetworkPolicyAmendment":{"network_policy_amendment":amendment}}));
                    }
                }
            }
            choices
        }
        "item/permissions/requestApproval" => vec![
            json!({"permissions":{},"scope":"turn"}),
            json!({"permissions":event["params"]["permissions"],"scope":"turn"}),
        ],
        _ => vec![], // Unsupported requests remain blocked. Never invent a grant.
    }
}
pub(crate) fn sanitize(v: Value) -> Value {
    // Apply the same structural credential policy as persisted native logs, including
    // object/array credentials and case variants. Protocol IDs and safe strings survive.
    let v = serde_json::from_str(&forge_adapters::process::redact(&v.to_string()))
        .unwrap_or(Value::Null);
    without_identity(v)
}
fn without_identity(v: Value) -> Value {
    match v {
        Value::String(s) => Value::String(forge_adapters::process::redact(&s)),
        Value::Array(a) => Value::Array(a.into_iter().map(without_identity).collect()),
        Value::Object(o) => Value::Object(
            o.into_iter()
                .filter(|(k, _)| {
                    ![
                        "email",
                        "accessToken",
                        "refreshToken",
                        "idToken",
                        "apiKey",
                        "authorization",
                    ]
                    .contains(&k.as_str())
                })
                .map(|(k, v)| (k, without_identity(v)))
                .collect(),
        ),
        v => v,
    }
}
pub async fn serve(
    root: &Path,
    state: &Path,
    port: u16,
    executable: Option<PathBuf>,
    manifest: Option<PathBuf>,
) -> Result<Value> {
    let app = App::new(root, state, executable, manifest)?;
    // UI remains available with an honest error if the native client is unavailable.
    if let Err(e) = app.connect().await {
        app.view.lock().await.error = Some(e.to_string());
    }
    let result = crate::workbench_http::serve(app.clone(), port).await;
    app.stop_forge().await;
    if let Some(n) = app.native.write().await.take() {
        n.shutdown().await;
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn oversized_mcp_keeps_error_flag_after_content_exhausts_budget() {
        let original = json!({"method":"item/completed","params":{"threadId":"thread","item":{"id":"tool","type":"mcpToolCall","status":"completed","server":"logs","tool":"query","result":{"content":[{"type":"text","text":"a".repeat(30000)},{"type":"text","text":"b".repeat(30000)},{"type":"text","text":"c".repeat(30000)}],"isError":true}}}});
        let event = display_event(original.clone());
        assert_eq!(event["params"]["item"]["result"]["isError"], true);
        assert_eq!(event["params"]["item"]["status"], "completed");
        assert_eq!(event["params"]["item"]["id"], "tool");
        assert_eq!(event["params"]["item"]["forgeDisplayTruncated"], true);
        assert!(serde_json::to_vec(&event).unwrap().len() <= 65536);
        assert!(original["params"]["item"]["forgeDisplayTruncated"].is_null());
    }
    #[test]
    fn oversized_diffs_keep_every_retained_file_identity_without_content_budget() {
        let changes = (0..40).map(|i| json!({"id":format!("file-{i}"),"path":format!("src/ação-{i}.rs"),"kind":{"type":"update","move_path":format!("src/original-{i}.rs")},"diff":format!("@@ -0,0 +1 @@\n+{}\n", "x".repeat(30000))})).collect::<Vec<_>>();
        let event = display_event(
            json!({"method":"item/completed","params":{"item":{"id":"files","type":"fileChange","status":"completed","aggregatedOutput":"x".repeat(30000),"changes":changes}}}),
        );
        let item = &event["params"]["item"];
        let files = item["changes"].as_array().unwrap();
        assert_eq!(files.len(), 32);
        assert_eq!(item["forgeOriginalFileCount"], 40);
        for (i, file) in files.iter().enumerate() {
            assert_eq!(file["id"], format!("file-{i}"));
            assert_eq!(file["path"], format!("src/ação-{i}.rs"));
            assert_eq!(file["kind"]["move_path"], format!("src/original-{i}.rs"));
            assert!(file["diff"].is_string());
        }
        assert!(serde_json::to_vec(&event).unwrap().len() <= 65536);
    }
    #[test]
    fn escaped_payload_falls_back_to_metadata_and_preserves_error_code() {
        let event = display_event(
            json!({"method":"item/completed","params":{"item":{"id":"tool","type":"mcpToolCall","status":"completed","result":{"content":[{"text":"\u{0}".repeat(30000)},{"text":"\u{0}".repeat(30000)},{"text":"\u{0}".repeat(30000)}],"isError":true},"error":{"code":7,"type":"provider","message":"error"}}}}),
        );
        let item = &event["params"]["item"];
        assert_eq!(item["result"]["isError"], true);
        assert!(item["result"]["forgeContentOmitted"].is_string());
        assert_eq!(item["error"]["code"], 7);
        assert_eq!(item["error"]["type"], "provider");
        assert!(serde_json::to_vec(&event).unwrap().len() <= 65536);
    }
    #[test]
    fn small_events_remain_identical_and_oversized_metadata_fails_explicitly() {
        let event = json!({"method":"item/completed","params":{"item":{"id":"tool","type":"mcpToolCall","result":{"isError":true}}}});
        assert_eq!(display_event(event.clone()), event);
        let event = display_event(
            json!({"method":"item/completed","params":{"item":{"id":"files","type":"fileChange","changes":[{"path":"x".repeat(70000),"diff":""}]}}}),
        );
        assert_eq!(event["method"], "bridge/truncated");
    }
    #[test]
    fn oversized_tool_keeps_identity_and_marks_display_projection() {
        let original = serde_json::json!({"method":"item/completed","params":{"threadId":"thread","item":{"id":"tool","type":"mcpToolCall","status":"failed","server":"logs","tool":"query","arguments":{"limit":10},"error":{"message":"unavailable"},"result":{"text":"é".repeat(100000)}}}});
        let event = super::display_event(original.clone());
        assert_eq!(event["params"]["item"]["tool"], "query");
        assert_eq!(event["params"]["item"]["status"], "failed");
        assert_eq!(event["params"]["item"]["forgeDisplayTruncated"], true);
        assert_eq!(event["params"]["item"]["arguments"]["limit"], 10);
        assert!(serde_json::to_vec(&event).unwrap().len() <= 65536);
        assert!(original["params"]["item"]["forgeDisplayTruncated"].is_null());
    }
    use super::*;
    use std::time::Duration;
    fn action(value: Value) -> Action {
        serde_json::from_value(value).unwrap()
    }
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let state = base.join("state");
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workbench-native.py");
        (t, root, state, fixture)
    }
    #[test]
    fn browser_payload_redacts_structural_credentials_and_preserves_rpc_identity() {
        let clean = sanitize(
            json!({"id":"rpc-7","method":"turn/completed","params":{"password":["do-not-show"],"ID_TOKEN":{"private":"do-not-show"},"type":"completed","usage":{"inputTokens":9,"cachedInputTokens":3}}}),
        );
        assert_eq!(clean["id"], "rpc-7");
        assert_eq!(clean["params"]["type"], "completed");
        assert_eq!(clean["params"]["usage"]["inputTokens"], 9);
        assert!(!clean.to_string().contains("do-not-show"));
    }
    #[tokio::test]
    async fn session_names_survive_restart_without_persisting_prompt_content() {
        let (_t, root, state, fixture) = setup();
        let app = App::new(&root, &state, Some(fixture.clone()), None).unwrap();
        app.connect().await.unwrap();
        let result = app
            .action("sessions", action(json!({"operation_id":"new-title"})))
            .await
            .unwrap();
        let id = result["thread_id"].as_str().unwrap();
        app.action(
            "turn",
            action(
                json!({"operation_id":"title-turn","thread_id":id,"text":"private prompt content"}),
            ),
        )
        .await
        .unwrap();
        settled(&app).await;
        let title = app.snapshot().await["sessions"][0]["title"].clone();
        assert_eq!(title, "Conversa 1");
        app.native.write().await.take().unwrap().shutdown().await;
        drop(app);
        let restored = App::new(&root, &state, Some(fixture), None).unwrap();
        assert_eq!(restored.snapshot().await["sessions"][0]["title"], title);
        assert_eq!(restored.snapshot().await["sessions"][0]["loaded"], false);
        assert!(
            restored.reserve("title-turn").is_err(),
            "restart must not permit replay"
        );
        assert!(
            !String::from_utf8_lossy(&std::fs::read(state.join("workbench.sqlite")).unwrap())
                .contains("private prompt content")
        );
    }
    #[test]
    fn legacy_session_migration_preserves_other_project_and_operation_ownership() {
        let (_t, root, state, fixture) = setup();
        std::fs::create_dir_all(&state).unwrap();
        let db = rusqlite::Connection::open(state.join("workbench.sqlite")).unwrap();
        db.execute_batch("CREATE TABLE sessions(root TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(root,id)); CREATE TABLE operations(root TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(root,id));").unwrap();
        db.execute(
            "INSERT INTO sessions VALUES(?1,'legacy')",
            [root.to_string_lossy()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO sessions VALUES('another-project','foreign')",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO operations VALUES(?1,'previous-op')",
            [root.to_string_lossy()],
        )
        .unwrap();
        drop(db);
        let app = App::new(&root, &state, Some(fixture), None).unwrap();
        assert_eq!(app.view.try_lock().unwrap().sessions.len(), 1);
        assert_eq!(
            app.view.try_lock().unwrap().sessions["legacy"].title,
            "Conversa recuperada"
        );
        assert!(app.reserve("previous-op").is_err());
    }
    #[test]
    fn invalid_manifest_is_rejected_before_creating_operational_state() {
        let (_t, root, state, _fixture) = setup();
        let manifest = state.parent().unwrap().join("invalid.json");
        std::fs::write(&manifest, "{}").unwrap();
        assert!(App::new(&root, &state, None, Some(manifest)).is_err());
        assert!(!state.exists());
    }
    async fn settled(app: &App) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if app
                    .view
                    .lock()
                    .await
                    .sessions
                    .values()
                    .all(|s| s.active_turn.is_none())
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn workbench_session_and_project_isolation() {
        let (_t, root, state, fixture) = setup();
        let app = App::new(&root, &state, Some(fixture.clone()), None).unwrap();
        app.connect().await.unwrap();
        let session = app
            .action("sessions", action(json!({"operation_id":"new-1"})))
            .await
            .unwrap();
        let id = session["thread_id"].as_str().unwrap();
        app.action(
            "turn",
            action(json!({"operation_id":"turn-1","thread_id":id,"text":"hello"})),
        )
        .await
        .unwrap();
        settled(&app).await;
        let v = app.snapshot().await;
        assert!(!v.to_string().contains("never-render@example.invalid"));
        assert_eq!(v["sessions"][0]["usage"]["total"]["totalTokens"], 18);
        let other = root.parent().unwrap().join("other");
        std::fs::create_dir_all(other.join(".git")).unwrap();
        let b = App::new(&other, &state, Some(fixture), None).unwrap();
        b.connect().await.unwrap();
        assert!(
            b.action(
                "resume",
                action(json!({"operation_id":"resume-foreign","thread_id":id}))
            )
            .await
            .is_err()
        );
        assert!(
            b.snapshot().await["sessions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for a in [app, b] {
            a.native.read().await.as_ref().unwrap().shutdown().await;
        }
    }
    #[tokio::test]
    async fn workbench_approval_denial_is_preserved() {
        let (_t, root, state, fixture) = setup();
        let app = App::new(&root, &state, Some(fixture), None).unwrap();
        app.connect().await.unwrap();
        let r = app
            .action("sessions", action(json!({"operation_id":"new"})))
            .await
            .unwrap();
        let id = r["thread_id"].as_str().unwrap();
        app.action(
            "turn",
            action(json!({"operation_id":"turn","thread_id":id,"text":"approval"})),
        )
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if !app.view.lock().await.approvals.is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(app.action("approval",action(json!({"operation_id":"invented","thread_id":id,"request_id":"permission","decision":"acceptForSession"}))).await.is_err());
        app.action("approval",action(json!({"operation_id":"deny","thread_id":id,"request_id":"permission","decision":"decline"}))).await.unwrap();
        settled(&app).await;
        let v = app.snapshot().await;
        assert!(v["approvals"].as_array().unwrap().is_empty());
        assert!(
            v["sessions"][0]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["params"]["turn"]["status"] == "failed")
        );
        assert_eq!(v["forge"]["status"], "not_evaluated");
        assert!(app.action("approval",action(json!({"operation_id":"stale","thread_id":id,"request_id":"permission","decision":"accept"}))).await.is_err());
        app.native.read().await.as_ref().unwrap().shutdown().await;
    }
    #[tokio::test]
    async fn workbench_reconnect_does_not_repeat_writes() {
        let (_t, root, state, fixture) = setup();
        let app = App::new(&root, &state, Some(fixture.clone()), None).unwrap();
        app.connect().await.unwrap();
        let r = app
            .action("sessions", action(json!({"operation_id":"new"})))
            .await
            .unwrap();
        let id = r["thread_id"].as_str().unwrap();
        app.action(
            "turn",
            action(json!({"operation_id":"send-once","thread_id":id,"text":"hello"})),
        )
        .await
        .unwrap();
        settled(&app).await;
        app.action("reconnect", action(json!({"operation_id":"reconnect"})))
            .await
            .unwrap();
        let old_epoch = app.epoch.load(Ordering::SeqCst) - 1;
        app.event(
            json!({"method":"bridge/disconnected","params":{}}),
            old_epoch,
        )
        .await;
        assert_eq!(app.snapshot().await["connected"], true);
        assert_eq!(app.snapshot().await["sessions"][0]["loaded"], false);
        assert!(
            app.action(
                "turn",
                action(json!({"operation_id":"send-once","thread_id":id,"text":"hello"}))
            )
            .await
            .is_err()
        );
        app.native.read().await.as_ref().unwrap().shutdown().await;
        drop(app);
        let app = App::new(&root, &state, Some(fixture), None).unwrap();
        app.connect().await.unwrap();
        assert!(
            app.action(
                "turn",
                action(json!({"operation_id":"send-once","thread_id":id,"text":"hello"}))
            )
            .await
            .is_err()
        );
        assert!(
            app.snapshot().await["sessions"][0]["events"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        app.native.read().await.as_ref().unwrap().shutdown().await;
    }
    #[tokio::test]
    async fn explicit_resume_preserves_native_terminal_history_without_starting_a_turn() {
        let (_t, root, state, _) = setup();
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workbench-native-history.py");
        let app = App::new(&root, &state, Some(fixture), None).unwrap();
        app.connect().await.unwrap();
        let created = app
            .action("sessions", action(json!({"operation_id":"history-create"})))
            .await
            .unwrap();
        let id = created["thread_id"].as_str().unwrap();
        app.action(
            "resume",
            action(json!({"operation_id":"history-resume","thread_id":id})),
        )
        .await
        .unwrap();
        let snapshot = app.snapshot().await;
        let session = &snapshot["sessions"][0];
        assert!(session["active_turn"].is_null());
        let events = session["events"].as_array().unwrap();
        let terminals = events
            .iter()
            .filter(|e| e["method"] == "turn/completed")
            .collect::<Vec<_>>();
        assert_eq!(
            terminals
                .iter()
                .map(|e| e["params"]["turn"]["status"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["completed", "interrupted", "failed"]
        );
        assert!(terminals.iter().all(|e| e["params"]["threadId"] == id));
        assert_eq!(terminals[1]["params"]["turn"]["id"], "interrupted-history");
        assert_eq!(
            terminals[2]["params"]["turn"]["error"]["code"],
            "fixture_failure"
        );
        assert!(
            !snapshot
                .to_string()
                .contains("fixture-credential-not-for-browser")
        );
        assert!(
            events
                .iter()
                .any(|e| e["params"]["item"]["text"] == "Historical reply")
        );
        app.native.read().await.as_ref().unwrap().shutdown().await;
    }
    #[tokio::test]
    async fn workbench_interrupt_is_a_native_action() {
        let (_t, root, state, fixture) = setup();
        let app = App::new(&root, &state, Some(fixture), None).unwrap();
        app.connect().await.unwrap();
        let r = app
            .action("sessions", action(json!({"operation_id":"new"})))
            .await
            .unwrap();
        let id = r["thread_id"].as_str().unwrap();
        app.action(
            "turn",
            action(json!({"operation_id":"wait","thread_id":id,"text":"wait"})),
        )
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        app.action(
            "interrupt",
            action(json!({"operation_id":"stop","thread_id":id})),
        )
        .await
        .unwrap();
        settled(&app).await;
        assert!(app.snapshot().await.to_string().contains("interrupted"));
        app.native.read().await.as_ref().unwrap().shutdown().await;
    }
}
