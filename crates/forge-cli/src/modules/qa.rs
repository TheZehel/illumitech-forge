//! Frozen, explicit QA checks. This supervisor detects source mutation; it is not a sandbox.
use super::{QaLevel, coverage};
use crate::{guided, workflow};
use anyhow::{Context as _, Result, ensure};
use clap::Subcommand;
use forge_adapters::process;
use forge_domain::{Revision, Store, WorktreeLocks, digest, revision, valid_hash, valid_id};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Component, Path, PathBuf},
};
use tokio::sync::watch;

#[derive(Subcommand)]
pub enum Command {
    /// Read stack indicators and suggest commands; never executes them.
    Discover {
        #[arg(long)]
        root: PathBuf,
    },
    /// Preview a frozen QA request; --apply saves it externally without executing.
    Prepare {
        input: PathBuf,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        apply: bool,
    },
    /// Execute this exact scope once, with explicit consent to its commands.
    Start {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        approve_scope: String,
    },
    /// Read retained observations and revalidate their source/contract/evaluators.
    Status {
        id: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        ops_root: PathBuf,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    #[default]
    Text,
    Sarif,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub coverage_id: String,
    pub argv: Vec<String>,
    pub timeout_seconds: u64,
    pub inputs: BTreeMap<PathBuf, String>,
    #[serde(default)]
    pub format: Format,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub level: QaLevel,
    pub checks: Vec<Check>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    schema_version: u32,
    root: PathBuf,
    git_dir: PathBuf,
    source: Revision,
    request: Request,
    evaluator: PathBuf,
    evaluator_sha256: String,
}
const ALL: &[&str] = &[
    "QA-QUALITY",
    "QA-SECRETS",
    "QA-DEPENDENCIES",
    "QA-SAST",
    "QA-AUTH",
    "QA-BUILD",
    "QA-ADVERSARIAL",
    "QA-THREAT",
];
fn count(level: QaLevel) -> usize {
    match level {
        QaLevel::Light => 3,
        QaLevel::Standard => 6,
        QaLevel::Intensive => 8,
    }
}
fn external(path: &Path, root: &Path) -> Result<PathBuf> {
    ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "path traversal rejected"
    );
    guided::external(path, root)
}
fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let path = guided::safe_path(path)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() <= 256 * 1024 * 1024,
        "input must be a regular file no larger than 256 MiB"
    );
    let mut hash = Sha256::new();
    let mut input = file.take(256 * 1024 * 1024 + 1);
    let mut n = 0;
    let mut buffer = [0u8; 65536];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        n += read;
    }
    ensure!(n <= 256 * 1024 * 1024, "input grew beyond bound");
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn validate(request: &Request, root: &Path) -> Result<()> {
    ensure!(request.schema_version == 1, "unsupported QA request schema");
    ensure!(
        valid_id(&request.id) && request.id.len() <= 60,
        "invalid QA ID"
    );
    ensure!(
        !request.title.trim().is_empty()
            && request.title.len() <= 200
            && !request.title.contains(['\n', '\r', '\0']),
        "one-line QA title required"
    );
    ensure!(
        !request.checks.is_empty() && request.checks.len() <= 32,
        "QA requires 1..32 explicit checks"
    );
    let mut ids = BTreeSet::new();
    for check in &request.checks {
        ensure!(
            valid_id(&check.id) && ids.insert(&check.id),
            "invalid/duplicate QA check ID"
        );
        ensure!(
            ALL[..count(request.level)].contains(&check.coverage_id.as_str()),
            "unknown coverage_id or outside requested QA level"
        );
        ensure!(
            !check.argv.is_empty()
                && !check.argv[0].is_empty()
                && check.argv.len() <= 64
                && check
                    .argv
                    .iter()
                    .all(|a| a.len() <= 8192 && !a.contains('\0')),
            "invalid QA argv"
        );
        ensure!(
            (1..=600).contains(&check.timeout_seconds),
            "QA timeout must be 1..600 seconds"
        );
        ensure!(
            !check.inputs.is_empty() && check.inputs.len() <= 64,
            "each QA check needs 1..64 pinned external inputs"
        );
        for (path, hash) in &check.inputs {
            external(path, root)?;
            ensure!(
                path.is_absolute() && valid_hash(hash),
                "absolute external input and SHA-256 required"
            );
        }
    }
    Ok(())
}
fn inputs(request: &Request, root: &Path) -> Result<()> {
    for check in &request.checks {
        for (path, expected) in &check.inputs {
            ensure!(
                hash_file(&external(path, root)?)?.eq_ignore_ascii_case(expected),
                "external QA input changed: {}",
                path.display()
            );
        }
    }
    Ok(())
}
fn directory(root: &Path, ops: &Path, id: &str) -> Result<PathBuf> {
    ensure!(valid_id(id) && id.len() <= 60, "invalid QA ID");
    external(&external(ops, root)?.join("qa").join(id), root)
}
fn git_dir(root: &Path) -> Result<PathBuf> {
    Ok(
        PathBuf::from(forge_domain::git(root, &["rev-parse", "--absolute-git-dir"])?.trim())
            .canonicalize()?,
    )
}
fn scope(c: &Candidate) -> Result<String> {
    Ok(digest(&serde_json::to_vec(c)?))
}
fn save(path: &Path, value: &impl Serialize) -> Result<()> {
    guided::create(path, &serde_json::to_string_pretty(value)?)
}
fn load(root: &Path, ops: &Path, id: &str) -> Result<(PathBuf, Candidate)> {
    let root = guided::root(root)?;
    let dir = directory(&root, ops, id)?;
    state_path(&dir)?;
    let c: Candidate = workflow::read_json(&dir.join("candidate.json"))?;
    ensure!(
        c.schema_version == 1 && c.root == root && c.source.worktree == root && c.request.id == id,
        "QA project/identity mismatch"
    );
    validate(&c.request, &root)?;
    let seal: Value = workflow::read_json(&dir.join("scope.json"))?;
    ensure!(
        seal["scope_digest"] == scope(&c)?,
        "QA candidate changed; new reviewed scope required"
    );
    let request: Request = workflow::read_json(&dir.join("request.json"))?;
    ensure!(
        serde_json::to_value(request)? == serde_json::to_value(&c.request)?,
        "QA request contract changed"
    );
    Ok((dir, c))
}
fn current(c: &Candidate) -> Result<()> {
    ensure!(
        guided::root(&c.root)? == c.root && git_dir(&c.root)? == c.git_dir,
        "QA project identity changed"
    );
    ensure!(
        revision(&c.root)? == c.source,
        "QA source changed; new reviewed scope required"
    );
    ensure!(
        std::env::current_exe()?.canonicalize()? == c.evaluator
            && hash_file(&c.evaluator)? == c.evaluator_sha256,
        "QA evaluator changed; new reviewed scope required"
    );
    inputs(&c.request, &c.root)
}
fn coverage_rows(request: &Request, checks: &[Value]) -> Vec<Value> {
    let mut rows = coverage(QaLevel::Intensive);
    for row in &mut rows {
        let id = row["id"].as_str().unwrap();
        let selected = ALL[..count(request.level)].contains(&id);
        let group = checks
            .iter()
            .filter(|c| c["coverage_id"] == id)
            .collect::<Vec<_>>();
        let status = if !selected {
            "not_requested"
        } else if group
            .iter()
            .any(|c| c["status"] == "blocked" || c["status"] == "cancelled")
        {
            "blocked"
        } else if group.iter().any(|c| {
            matches!(
                c["status"].as_str(),
                Some("failed" | "timeout" | "output_limit")
            )
        }) {
            "failed"
        } else if group.iter().any(|c| c["status"] == "unavailable") {
            "unavailable"
        } else if group.is_empty() || group.iter().any(|c| c["status"] != "passed") {
            "not_run"
        } else {
            "passed"
        };
        let evidence = group
            .iter()
            .filter_map(|c| c.get("evidence"))
            .filter(|e| !e.is_null())
            .cloned()
            .collect::<Vec<_>>();
        row["status"] = json!(status);
        row["checks"] = json!(group.iter().map(|c| &c["id"]).collect::<Vec<_>>());
        row["evidence"] = if evidence.is_empty() {
            Value::Null
        } else {
            json!(evidence)
        };
    }
    rows
}
fn pending_checks(request: &Request) -> Vec<Value> {
    request.checks.iter().map(|c| json!({"id":c.id,"coverage_id":c.coverage_id,"argv":c.argv,"status":"not_run","evidence":null})).collect()
}
fn view(c: &Candidate, status: &str, checks: &[Value]) -> Result<Value> {
    Ok(
        json!({"schema_version":1,"module":"qa","qa_id":c.request.id,"status":status,"scope_digest":scope(c)?,"root":c.root,"level":c.request.level,"checks":checks,"coverage":coverage_rows(&c.request, checks),"source":c.source,"checks_executed":checks.iter().filter(|c|c.get("process").is_some()).count(),"native_call":false,"acceptance":"observed checks only; not a universal security attestation","scope_warning":"Reviewed commands can write files. Source changes block acceptance and are never restored. No OS read-only sandbox is imposed."}),
    )
}
pub fn prepare(request: Request, root: &Path, ops: &Path, apply: bool) -> Result<Value> {
    let root = guided::root(root)?;
    let dir = directory(&root, ops, &request.id)?;
    validate(&request, &root)?;
    inputs(&request, &root)?;
    let evaluator = std::env::current_exe()?.canonicalize()?;
    let c = Candidate {
        schema_version: 1,
        git_dir: git_dir(&root)?,
        source: revision(&root)?,
        evaluator_sha256: hash_file(&evaluator)?,
        evaluator,
        root,
        request,
    };
    ensure!(
        serde_json::to_vec_pretty(&c)?.len() <= 1024 * 1024,
        "QA candidate exceeds 1 MiB"
    );
    if apply {
        let _lock = WorktreeLocks::acquire(&BTreeMap::from([("project".into(), c.root.clone())]))?;
        current(&c)?;
        let files = [
            ("candidate.json", serde_json::to_value(&c)?),
            ("request.json", serde_json::to_value(&c.request)?),
            ("scope.json", json!({"scope_digest":scope(&c)?})),
        ];
        for (name, value) in &files {
            guided::check_file(&dir.join(name), &serde_json::to_string_pretty(value)?)?;
        }
        for (name, value) in &files {
            save(&dir.join(name), value)?;
        }
    }
    let mut value = view(
        &c,
        if apply { "prepared" } else { "preview" },
        &pending_checks(&c.request),
    )?;
    value["candidate"] = serde_json::to_value(c)?;
    value["executed"] = json!(false);
    Ok(value)
}

pub fn discover(root: &Path) -> Result<Value> {
    let root = guided::root(root)?;
    let mut indicators = vec![];
    let mut suggestions = vec![];
    for (name, stack, argv) in [
        (
            "Cargo.toml",
            "rust",
            vec!["cargo", "test", "--workspace", "--locked"],
        ),
        ("package.json", "node", vec!["npm", "test"]),
        ("pyproject.toml", "python", vec!["python", "-m", "pytest"]),
        ("go.mod", "go", vec!["go", "test", "./..."]),
    ] {
        let path = guided::safe_path(&root.join(name))?;
        if path.is_file() {
            indicators.push(json!({"path":name,"stack":stack,"sha256":hash_file(&path)?}));
            suggestions.push(json!({"source":name,"kind":"static indicator heuristic; inspect scripts and permissions","coverage_id":"QA-QUALITY","argv":argv,"availability":"unknown","status":"not_run"}));
        }
    }
    Ok(
        json!({"schema_version":1,"module":"qa","status":"discovered","root":root,"indicators":indicators,"suggestions":suggestions,"checks":[],"checks_executed":0,"coverage":coverage(QaLevel::Intensive),"executed":false,"mutation":false,"native_call":false,"limitations":["Static suggestions do not prove installed tools, applicability or coverage", "No dependency installation, script execution or network scan"]}),
    )
}

fn state_path(dir: &Path) -> Result<PathBuf> {
    let state = dir.join("state");
    ensure!(
        !state.ancestors().any(|p| p.join(".git").exists()),
        "QA state must remain outside Git projects"
    );
    for name in [
        "state.sqlite",
        "state.sqlite-wal",
        "state.sqlite-shm",
        "state.sqlite-journal",
        ".forge-state-access.lock",
    ] {
        guided::safe_path(&state.join(name))?;
    }
    Ok(state.join("state.sqlite"))
}
fn journal(dir: &Path, id: &str, key: &str, value: &Value) -> Result<()> {
    let path = state_path(dir)?;
    let mut store = Store::open(&path, false)?;
    store.journal_metadata(id, key, value)
}
fn revalidate(c: &Candidate, ops: &Path) -> Result<()> {
    let (_, disk) = load(&c.root, ops, &c.request.id)?;
    ensure!(
        scope(&disk)? == scope(c)?,
        "QA candidate replaced during execution"
    );
    current(c)
}
fn unavailable(error: &anyhow::Error) -> bool {
    error.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    })
}
async fn observe(check: &Check, c: &Candidate, cancel: watch::Receiver<bool>) -> Value {
    let argv = check
        .argv
        .iter()
        .map(|a| {
            if a == "{repo}" {
                c.root.to_string_lossy().into_owned()
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>();
    let result = process::run(&argv, &c.root, check.timeout_seconds, cancel).await;
    let mut value = json!({"id":check.id,"coverage_id":check.coverage_id,"argv":argv,"format":check.format,"inputs":check.inputs});
    match result {
        Err(e) => {
            value["status"] = json!(if unavailable(&e) {
                "unavailable"
            } else {
                "failed"
            });
            value["error"] = json!(process::redact(&format!("{e:#}")));
        }
        Ok(result) => {
            value["status"] = json!(if result.passed() {
                "passed"
            } else {
                match result.reason.as_str() {
                    "cancelled" => "cancelled",
                    "timeout" => "timeout",
                    "output_limit" => "output_limit",
                    _ => "failed",
                }
            });
            if check.format == Format::Sarif {
                let parsed = serde_json::from_str::<Value>(&result.stdout)
                    .context("invalid/incomplete SARIF JSON")
                    .and_then(|v| crate::findings::parse(&v, &c.root));
                match parsed {
                    Ok(sarif) => {
                        if (sarif["status"] != "imported"
                            || !sarif["findings"].as_array().is_some_and(Vec::is_empty))
                            && value["status"] == "passed"
                        {
                            value["status"] = json!("failed");
                        }
                        value["sarif"] = sarif;
                    }
                    Err(e) => {
                        if value["status"] == "passed" {
                            value["status"] = json!("failed");
                        }
                        value["sarif_error"] = json!(process::redact(&format!("{e:#}")));
                    }
                }
            }
            value["process"] =
                serde_json::to_value(result).expect("serializable bounded process result");
        }
    }
    value
}
fn overall(c: &Candidate, checks: &[Value]) -> &'static str {
    let coverage = coverage_rows(&c.request, checks);
    if checks
        .iter()
        .any(|r| matches!(r["status"].as_str(), Some("blocked" | "cancelled")))
    {
        "blocked"
    } else if coverage.iter().any(|r| r["status"] == "failed") {
        "failed"
    } else if coverage
        .iter()
        .any(|r| matches!(r["status"].as_str(), Some("not_run" | "unavailable")))
    {
        "degraded"
    } else {
        "passed"
    }
}
pub async fn start(
    root: &Path,
    ops: &Path,
    id: &str,
    approval: &str,
    cancel: watch::Receiver<bool>,
) -> Result<Value> {
    let root = guided::root(root)?;
    let _lock = WorktreeLocks::acquire(&BTreeMap::from([("project".into(), root.clone())]))?;
    // Reload under the same lock held throughout execution. Consent never approves a new candidate.
    let (dir, c) = load(&root, ops, id)?;
    ensure!(
        scope(&c)? == approval,
        "QA scope digest mismatch; review the prepared scope"
    );
    current(&c)?;
    ensure!(
        !dir.join("intent.json").try_exists()? && !dir.join("result.json").try_exists()?,
        "QA scope already submitted; inspect status and prepare a new ID; no replay"
    );
    ensure!(!*cancel.borrow(), "QA cancelled before execution");
    let intent = json!({"schema_version":1,"qa_id":id,"scope_digest":approval,"status":"running","owner_pid":std::process::id(),"source":c.source});
    save(&dir.join("intent.json"), &intent)?;
    journal(&dir, id, "qa-scope", &intent)?;
    let mut checks = pending_checks(&c.request);
    for (i, check) in c.request.checks.iter().enumerate() {
        let preflight = revalidate(&c, ops);
        if let Err(e) = preflight {
            checks[i]["status"] = json!("blocked");
            checks[i]["error"] = json!(process::redact(&format!("{e:#}")));
        } else if *cancel.borrow() {
            checks[i]["status"] = json!("cancelled");
        } else {
            let intention = json!({"schema_version":1,"scope_digest":approval,"check_id":check.id,"status":"running"});
            save(&dir.join(format!("check-{i:02}-intent.json")), &intention)?;
            journal(&dir, id, &format!("qa-check:{}", check.id), &intention)?;
            checks[i] = observe(check, &c, cancel.clone()).await;
            checks[i]["source_before"] = serde_json::to_value(&c.source)?;
            match revision(&c.root) {
                Ok(source) => checks[i]["source_after"] = serde_json::to_value(source)?,
                Err(e) => {
                    checks[i]["source_after"] = Value::Null;
                    checks[i]["source_error"] = json!(process::redact(&format!("{e:#}")));
                }
            }
            if let Err(e) = revalidate(&c, ops) {
                checks[i]["observed_status"] = checks[i]["status"].clone();
                checks[i]["status"] = json!("blocked");
                checks[i]["error"] = json!(process::redact(&format!("{e:#}")));
                checks[i]["source_mutation_policy"] = json!(
                    "No files restored or overwritten; review changed source and prepare a new scope"
                );
            }
        }
        let evidence = dir.join(format!("check-{i:02}.json"));
        let observation =
            json!({"schema_version":1,"scope_digest":approval,"source":c.source,"check":checks[i]});
        save(&evidence, &observation)?;
        checks[i]["evidence"] = json!({"path":evidence,"sha256":hash_file(&evidence)?});
        journal(&dir, id, &format!("qa-check:{}", check.id), &checks[i])?;
        if matches!(checks[i]["status"].as_str(), Some("blocked" | "cancelled")) {
            break;
        }
    }
    let mut value = view(&c, overall(&c, &checks), &checks)?;
    if let Err(e) = revalidate(&c, ops) {
        value["status"] = json!("blocked");
        value["error"] = json!(process::redact(&format!("{e:#}")));
    }
    value["executed"] = json!(true);
    value["replayed"] = json!(false);
    save(&dir.join("result.json"), &value)?;
    journal(&dir, id, "qa-result", &value)?;
    // The final create-only receipt seals the closed journal. Status reads bytes,
    // not SQLite: even a read-only SQLite connection can create WAL/SHM files.
    save(
        &dir.join("result-sha256.json"),
        &json!({"sha256":hash_file(&dir.join("result.json"))?,
            "journal_sha256":hash_file(&state_path(&dir)?)?,
            "intent_sha256":hash_file(&dir.join("intent.json"))?}),
    )?;
    Ok(value)
}

pub fn status(root: &Path, ops: &Path, id: &str) -> Result<Value> {
    let (dir, c) = load(root, ops, id)?;
    let fresh = current(&c);
    let mut value = if guided::safe_path(&dir.join("result.json"))?.try_exists()? {
        let seal: Value = workflow::read_json(&dir.join("result-sha256.json"))?;
        ensure!(
            seal["sha256"] == hash_file(&dir.join("result.json"))?,
            "QA result changed or incomplete"
        );
        // A bounded process observation can be larger than the request's 1 MiB bound.
        let result = read_result(&dir.join("result.json"))?;
        ensure!(
            result["scope_digest"] == scope(&c)? && result["qa_id"] == id,
            "QA result scope mismatch"
        );
        let intent: Value = workflow::read_json(&dir.join("intent.json"))?;
        ensure!(
            intent["scope_digest"] == scope(&c)?
                && intent["source"] == serde_json::to_value(&c.source)?,
            "QA execution intent changed"
        );
        ensure!(
            seal["intent_sha256"] == hash_file(&dir.join("intent.json"))?
                && seal["journal_sha256"] == hash_file(&state_path(&dir)?)?,
            "QA intent/journal changed; new reviewed scope required"
        );
        let wal = dir.join("state/state.sqlite-wal");
        ensure!(
            !wal.try_exists()? || std::fs::metadata(wal)?.len() == 0,
            "QA journal has unsealed WAL changes; outcome uncertain; no replay"
        );
        for (i, row) in result["checks"]
            .as_array()
            .context("QA checks missing")?
            .iter()
            .enumerate()
        {
            if !row["evidence"].is_null() {
                let path = dir.join(format!("check-{i:02}.json"));
                ensure!(
                    row["evidence"]["path"] == json!(path)
                        && row["evidence"]["sha256"] == hash_file(&path)?,
                    "QA check evidence changed"
                );
            }
        }
        result
    } else {
        let attempted = guided::safe_path(&dir.join("intent.json"))?.try_exists()?;
        let mut checks = pending_checks(&c.request);
        if attempted {
            let intent: Value = workflow::read_json(&dir.join("intent.json"))?;
            ensure!(
                intent["scope_digest"] == scope(&c)?,
                "QA intent scope changed"
            );
            for (i, check) in checks.iter_mut().enumerate() {
                let path = dir.join(format!("check-{i:02}.json"));
                if guided::safe_path(&path)?.try_exists()? {
                    let observation = read_result(&path)?;
                    ensure!(
                        observation["scope_digest"] == scope(&c)?
                            && observation["check"]["id"] == check["id"],
                        "QA observation scope changed"
                    );
                    *check = observation["check"].clone();
                    check["evidence"] = json!({"path":path,"sha256":hash_file(&path)?});
                } else if guided::safe_path(&dir.join(format!("check-{i:02}-intent.json")))?
                    .try_exists()?
                {
                    check["status"] = json!("blocked");
                    check["error"] =
                        json!("Check in progress or interrupted; result unknown; no replay");
                }
            }
        }
        let mut v = view(&c, if attempted { "blocked" } else { "prepared" }, &checks)?;
        if attempted {
            v["error"] = json!(
                "Execution in progress or interrupted/incomplete. Inspect retained evidence; a fresh scope is required for another execution."
            );
        }
        v
    };
    let source_current = revision(&c.root).is_ok_and(|r| r == c.source);
    let fresh = fresh.and_then(|()| {
        ensure!(
            source_current,
            "QA source changed during status; new reviewed scope required"
        );
        Ok(())
    });
    value["scope_current"] = json!(fresh.is_ok());
    value["source_current"] = json!(source_current);
    if let Err(e) = fresh {
        value["recorded_status"] = value["status"].clone();
        value["status"] = json!("stale");
        value["effective_status"] = json!("blocked");
        value["error"] = json!(process::redact(&format!("{e:#}")));
        value["evidence_current"] = json!(false);
    } else {
        value["evidence_current"] =
            json!(value["status"] != "prepared" && value["status"] != "blocked");
    }
    value["mutation"] = json!(false);
    value["replayed"] = json!(false);
    Ok(value)
}
fn read_result(path: &Path) -> Result<Value> {
    let file = std::fs::File::open(guided::safe_path(path)?)?;
    ensure!(
        file.metadata()?.is_file(),
        "regular QA evidence file required"
    );
    // 32 bounded stdout/stderr pairs, plus SARIF projections and JSON escaping.
    const LIMIT: u64 = 256 * 1024 * 1024;
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "QA evidence exceeds bound");
    Ok(serde_json::from_slice(&bytes)?)
}
pub async fn dispatch(command: &Command) -> Result<Value> {
    match command {
        Command::Discover { root } => discover(root),
        Command::Prepare {
            input,
            root,
            ops_root,
            apply,
        } => prepare(workflow::read_json(input)?, root, ops_root, *apply),
        Command::Start {
            id,
            root,
            ops_root,
            approve_scope,
        } => {
            let signals = crate::publication::Signals::new();
            start(root, ops_root, id, approve_scope, signals.receiver.clone()).await
        }
        Command::Status { id, root, ops_root } => status(root, ops_root, id),
    }
}
