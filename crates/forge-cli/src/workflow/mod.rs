//! A scope grant authorizes a frozen contract, never future acceptance evidence.
mod checks;
mod prepare;
mod recovery;
pub(crate) mod ui;
use crate::{engine, guided};
use anyhow::{Context as _, Result, ensure};
use clap::Subcommand;
use forge_domain::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Subcommand)]
pub enum Command {
    /// Review a request and exact checks; --apply saves an external candidate only.
    Prepare {
        input: PathBuf,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Explicit scope consent; creates reviewed specs and runs inspect/dry-run before execution.
    Start {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        approve_scope: String,
    },
    Status {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
    Diagnose {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
    Resume {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        reconcile: Option<String>,
    },
    List {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
    Doctor {
        #[arg(long)]
        root: PathBuf,
    },
    Recipes {
        #[arg(long)]
        root: PathBuf,
    },
    Findings {
        input: PathBuf,
        #[arg(long)]
        root: PathBuf,
    },
    #[command(hide = true)]
    Check {
        contract: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        root: PathBuf,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub before_exit: Option<i32>,
    #[serde(default)]
    pub rescan: bool,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default = "check_timeout")]
    pub timeout_seconds: u64,
}
fn check_timeout() -> u64 {
    600
}
fn context_default() -> ContextPolicy {
    ContextPolicy {
        profile: ContextProfile::Bounded,
        budget_tokens: Some(16384),
        sources: vec![],
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub request: String,
    pub recipe: String,
    #[serde(default)]
    pub checks: Vec<Check>,
    /// Frozen external regression scripts. Review their full contents before consenting.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub proposal: Option<crate::planning::Proposal>,
    /// New workflows only. Legacy manifests keep their own context policy.
    #[serde(default = "context_default")]
    pub context: ContextPolicy,
    #[serde(default)]
    pub human_gates: Vec<String>,
    #[serde(default)]
    pub planning_operation: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckContract {
    pub schema_version: u32,
    pub root: PathBuf,
    pub checks: Vec<Check>,
    pub files: BTreeMap<String, String>,
    #[serde(default)]
    pub scanner_inputs: BTreeMap<PathBuf, String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub schema_version: u32,
    pub root: PathBuf,
    pub source: Revision,
    pub request: Request,
    pub manifest: Manifest,
    pub documents: BTreeMap<PathBuf, String>,
    pub checks: CheckContract,
    pub evaluator: PathBuf,
    pub evaluator_sha256: String,
    #[serde(default)]
    pub planning_call: Option<forge_domain::usage::TokenCall>,
}
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let path = guided::safe_path(path)?;
    let f = std::fs::File::open(path)?;
    ensure!(f.metadata()?.is_file(), "regular JSON file required");
    let mut bytes = vec![];
    f.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024 * 1024, "JSON exceeds 1 MiB");
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn directory(root: &Path, ops: &Path, id: &str) -> Result<PathBuf> {
    ensure!(valid_id(id) && id.len() <= 60, "invalid workflow ID");
    Ok(guided::external(ops, root)?.join("workflows").join(id))
}
pub fn load(root: &Path, ops: &Path, id: &str) -> Result<(PathBuf, Candidate)> {
    load_candidate(root, ops, id, true)
}
pub(super) fn load_projection(root: &Path, ops: &Path, id: &str) -> Result<(PathBuf, Candidate)> {
    load_candidate(root, ops, id, false)
}
fn load_candidate(
    root: &Path,
    ops: &Path,
    id: &str,
    integrity: bool,
) -> Result<(PathBuf, Candidate)> {
    let root = guided::root(root)?;
    let dir = directory(&root, ops, id)?;
    let c: Candidate = read_json(&dir.join("candidate.json"))?;
    ensure!(
        c.schema_version == 1 && c.root == root && c.request.id == id,
        "workflow project/identity mismatch"
    );
    prepare::validate(&c.request, &root)?;
    if integrity {
        ensure!(
            digest(&std::fs::read(guided::safe_path(&c.evaluator)?)?) == c.evaluator_sha256,
            "workflow evaluator changed"
        );
    }
    ensure!(
        serde_json::to_value(read_json::<Manifest>(&dir.join("release.json"))?)?
            == serde_json::to_value(&c.manifest)?,
        "prepared manifest changed"
    );
    ensure!(
        serde_json::to_value(read_json::<CheckContract>(&dir.join("checks.json"))?)?
            == serde_json::to_value(&c.checks)?,
        "prepared check contract changed"
    );
    ensure!(
        !c.manifest.automation.enabled
            && c.manifest.automation.publish.is_none()
            && c.manifest.repos.len() == 1
            && c.manifest
                .repos
                .values()
                .all(|r| guided::root(&r.path).is_ok_and(|p| p == root)),
        "workflow scope cannot publish or escape its project"
    );
    for path in c.documents.keys() {
        ensure!(
            guided::safe_path(path)?.starts_with(root.join("specs").join(id)),
            "document outside workflow spec directory"
        );
    }
    c.manifest.validate()?;
    if integrity {
        c.manifest
            .verify_evaluators(&dir, &BTreeMap::from([("project".into(), root.clone())]))?;
    }
    Ok((dir, c))
}
pub fn consent_digest(c: &Candidate) -> Result<String> {
    Ok(digest(&serde_json::to_vec(c)?))
}
pub async fn dispatch(command: &Command) -> Result<Value> {
    match command {
        Command::Prepare {
            input,
            root,
            ops_root,
            apply,
        } => prepare::prepare(read_json(input)?, root, ops_root, *apply),
        Command::Start {
            id,
            root,
            ops_root,
            approve_scope,
        } => start(root, ops_root, id, approve_scope).await,
        Command::Status { id, root, ops_root } | Command::Diagnose { id, root, ops_root } => {
            recovery::status(root, ops_root, id)
        }
        Command::Resume {
            id,
            root,
            ops_root,
            reconcile,
        } => resume(root, ops_root, id, reconcile.clone()).await,
        Command::List { root, ops_root } => list(root, ops_root),
        Command::Doctor { root } => doctor(root).await,
        Command::Recipes { root } => Ok(
            json!({"schema_version":1,"recipes":["feature","bug","dependencies","pr"],"suggested_checks":prepare::suggested(&guided::root(root)?)?,"bug_requires":"frozen external regression with nonzero before_exit","publication":"never implicit"}),
        ),
        Command::Findings { input, root } => crate::findings::file(input, root),
        Command::Check { contract, id, root } => checks::run(contract, id, root, None).await,
    }
}
pub async fn start(root: &Path, ops: &Path, id: &str, approval: &str) -> Result<Value> {
    let (dir, c) = load(root, ops, id)?;
    ensure!(
        consent_digest(&c)? == approval,
        "scope changed; review the new digest before starting"
    );
    let roots = BTreeMap::from([("project".to_owned(), c.root.clone())]);
    let preparation_lock = WorktreeLocks::acquire(&roots)?;
    {
        ensure!(
            revision(&c.root)? == c.source,
            "project changed after preparation; prepare a fresh reviewed candidate"
        );
        for (path, body) in &c.documents {
            guided::check_file(path, body)?;
        }
        let mut store = Store::open(&dir.join("state/state.sqlite"), false)?;
        ensure!(
            store.metadata("workflow-scope")?.is_none(),
            "scope already submitted; inspect status and resume instead of replaying start"
        );
        store.journal_metadata(&c.manifest.release_id,"workflow-scope",&json!({"schema_version":1,"digest":approval,"phase":"preparing","acceptance_granted":false}))?;
        for (path, body) in &c.documents {
            guided::create(path, body)?;
        }
        let context = engine::Context::load(&dir.join("release.json"), &dir.join("state"))?;
        // These are the same engine checks used by explicit release inspect and --dry-run.
        context.inspect()?;
        ensure!(
            context.manifest.validate()?.len() == c.manifest.tasks.len(),
            "invalid workflow DAG"
        );
        store.journal_metadata(&c.manifest.release_id,"workflow-scope",&json!({"schema_version":1,"digest":approval,"phase":"prepared","revision":revision(&c.root)?,"acceptance_granted":false}))?;
    }
    // Baselines are journaled before spawn; unknown outcomes must never be replayed.
    let mut store = Store::open(&dir.join("state/state.sqlite"), false)?;
    for check in &c.checks.checks {
        if let Some(exit) = check.before_exit {
            store.journal_metadata(
                &c.manifest.release_id,
                "workflow-baseline",
                &json!({"check":check.id,"status":"running"}),
            )?;
            let before = revision(&c.root)?;
            checks::run(&dir.join("checks.json"), &check.id, &c.root, Some(exit)).await?;
            ensure!(
                revision(&c.root)? == before,
                "baseline changed project files; inspect before continuing"
            );
            store.journal_metadata(
                &c.manifest.release_id,
                "workflow-baseline",
                &json!({"check":check.id,"status":"passed"}),
            )?;
        }
    }
    if let Some(call) = &c.planning_call {
        store.record_token_call(&c.manifest.release_id, call)?;
    }
    let mut grant = store.metadata("workflow-scope")?.context("scope missing")?;
    grant["phase"] = json!("execution_authorized");
    grant["revision"] = serde_json::to_value(revision(&c.root)?)?;
    store.journal_metadata(&c.manifest.release_id, "workflow-scope", &grant)?;
    let control = store.metadata("workflow-control")?;
    let expected = revisions(&roots)?;
    drop(store);
    drop(preparation_lock);
    if control
        .as_ref()
        .is_some_and(|c| c == "pause" || c == "cancel")
    {
        return Ok(
            json!({"schema_version":1,"workflow_id":id,"status":if control==Some(json!("pause")){"paused"}else{"cancelled"},"replayed":false}),
        );
    }
    // Initial execution is separate from subsequent explicit resume.
    engine::execute_approved(
        engine::Context::load(&dir.join("release.json"), &dir.join("state"))?,
        expected,
    )
    .await
}
pub async fn resume(root: &Path, ops: &Path, id: &str, reconcile: Option<String>) -> Result<Value> {
    let (dir, c) = load(root, ops, id)?;
    let store = Store::open(&dir.join("state/state.sqlite"), true)?;
    let scope = store
        .metadata("workflow-scope")?
        .context("no scope consent; use start")?;
    ensure!(
        scope["digest"] == consent_digest(&c)?,
        "contract changed; a new scope consent is required"
    );
    ensure!(
        scope["phase"] == "execution_authorized",
        "preparation/baseline interrupted; inspect files and prepare a fresh reviewed workflow, no command replayed"
    );
    let context = engine::Context::load(&dir.join("release.json"), &dir.join("state"))?;
    if store.load(&c.manifest.release_id)?.is_none() {
        ensure!(
            scope["revision"] == serde_json::to_value(revision(&c.root)?)?,
            "project changed before initial spawn; new scope consent required"
        );
        let expected = revisions(&context.roots)?;
        drop(store);
        Store::open(&dir.join("state/state.sqlite"), false)?.remove_metadata("workflow-control")?;
        return engine::execute_approved(context, expected).await;
    }
    drop(store);
    engine::execute(context, true, reconcile).await
}

pub fn list(root: &Path, ops: &Path) -> Result<Value> {
    let root = guided::root(root)?;
    let base = guided::external(ops, &root)?.join("workflows");
    let mut workflows = vec![];
    if base.exists() {
        for entry in std::fs::read_dir(guided::safe_path(&base)?)?.take(100) {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            if let Ok(c) = read_json::<Candidate>(&base.join(&id).join("candidate.json")) {
                if c.root != root {
                    continue;
                }
                workflows.push(json!({"id":id,"title":c.request.title,"integrity":"verified on open/start, not listing"}));
            }
        }
    }
    workflows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(json!({"schema_version":1,"workflows":workflows,"limit":100,"mutation":false}))
}

pub async fn doctor(root: &Path) -> Result<Value> {
    let root = guided::root(root)?;
    let signal = crate::publication::Signals::new();
    let mut checks = vec![];
    let mut auth_kind = "unavailable";
    for (id, argv) in [
        ("git", vec!["git".into(), "--version".into()]),
        ("client", vec!["codex".into(), "--version".into()]),
        (
            "authentication",
            vec!["codex".into(), "login".into(), "status".into()],
        ),
    ] {
        match forge_adapters::process::run(&argv,&root,10,signal.receiver.clone()).await{
   Ok(result)=>{if id=="authentication"&&result.passed(){auth_kind=if format!("{}{}",result.stdout,result.stderr).to_lowercase().contains("chatgpt"){"subscription"}else{"native_configured"};}checks.push(json!({"id":id,"passed":result.passed(),"reason":result.reason}));},
   Err(_)=>checks.push(json!({"id":id,"passed":false,"reason":"not available; install/authenticate the native client separately"}))
  }
    }
    let healthy = checks.iter().all(|c| c["passed"] == true);
    Ok(
        json!({"schema_version":1,"status":if healthy{"passed"}else{"blocked"},"checks":checks,"authentication_kind":auth_kind,"installation":crate::installation::dispatch(&crate::installation::Command::Doctor).await?,"model":"native configuration unchanged; not inferred by this diagnostic","native_inference_calls":0,"mutation":false,"memory":"optional","next_action":if healthy{"Review your workflow before execution"}else{"Resolve the reported prerequisite, then rerun this read-only diagnostic. No installation, login flow or configuration change was attempted."}}),
    )
}
