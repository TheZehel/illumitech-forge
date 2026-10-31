use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub release_id: String,
    pub repos: BTreeMap<String, Repo>,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub gates: Vec<Gate>,
    #[serde(default)]
    pub memory: Memory,
    #[serde(default)]
    pub automation: Automation,
    #[serde(default)]
    pub context: ContextPolicy,
    #[serde(default)]
    pub budget: Budget,
    #[serde(default)]
    pub execution: Execution,
    #[serde(default)]
    pub review: ReviewPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<IssueSource>,
}
/// Selected demand provenance only; never execution or publication authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueSource {
    pub schema_version: u32,
    pub provider: String,
    pub repository: String,
    pub number: u64,
    pub url: String,
    pub content_sha256: String,
    pub provenance: IssueProvenance,
    pub spec_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueProvenance {
    AuthenticatedGh,
    OfflineSnapshot,
}
impl IssueSource {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1 && self.provider == "github",
            "unsupported issue source"
        );
        ensure!(
            self.repository.split('/').count() == 2 && self.repository.split('/').all(valid_id),
            "invalid issue repository"
        );
        ensure!(
            self.number > 0 && valid_hash(&self.content_sha256) && valid_id(&self.spec_id),
            "invalid issue source identity/hash"
        );
        ensure!(
            self.url.eq_ignore_ascii_case(&format!(
                "https://github.com/{}/issues/{}",
                self.repository, self.number
            )),
            "issue URL host/repository/number mismatch or redirect"
        );
        Ok(())
    }
}
/// Bounded local writers. Roots are explicitly supplied; Forge never commits or merges them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Execution {
    pub max_parallel: u32,
}
impl Default for Execution {
    fn default() -> Self {
        Self { max_parallel: 1 }
    }
}
/// Independent native review is advisory; every invocation is explicitly requested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReviewPolicy {
    pub max_iterations: u32,
    pub max_tokens: Option<u64>,
    pub timeout_seconds: u64,
}
impl Default for ReviewPolicy {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            max_tokens: None,
            timeout_seconds: 600,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    pub path: PathBuf,
    #[serde(default)]
    pub specs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub repo: String,
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default = "priority")]
    pub priority: i32,
    pub executor: Executor,
    pub acceptance: Vec<Acceptance>,
    #[serde(default = "task_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "attempts")]
    pub max_attempts: u32,
    #[serde(default)]
    pub context: Option<ContextPolicy>,
    #[serde(default)]
    pub budget: Budget,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextPolicy {
    pub profile: ContextProfile,
    pub budget_tokens: Option<u64>,
    /// Optional repo-relative files; instructions and referenced specs are always mandatory.
    pub sources: Vec<PathBuf>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextProfile {
    #[default]
    Full,
    Bounded,
}
impl ContextPolicy {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.sources.len() <= 100, "too many context sources");
        ensure!(
            self.sources.iter().all(|p| local_path(p)),
            "context sources require safe repo-relative paths"
        );
        ensure!(
            self.budget_tokens.is_none_or(|n| n > 0 && n <= 1_000_000),
            "context budget must be 1..=1000000"
        );
        ensure!(
            self.profile != ContextProfile::Bounded || self.budget_tokens.is_some(),
            "bounded context requires budget_tokens"
        );
        Ok(())
    }
}
fn task_timeout() -> u64 {
    1800
}
fn gate_timeout() -> u64 {
    600
}
fn attempts() -> u32 {
    3
}
fn priority() -> i32 {
    100
}
fn yes() -> bool {
    true
}
fn origin() -> String {
    "origin".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Automation {
    pub enabled: bool,
    pub require_clean_worktree: bool,
    pub publish: Option<DraftPr>,
    pub observability: Observability,
}
impl Default for Automation {
    fn default() -> Self {
        Self {
            enabled: false,
            require_clean_worktree: true,
            publish: None,
            observability: Observability::default(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Observability {
    pub langsmith: TraceMode,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    #[default]
    Off,
    Optional,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPr {
    pub repository: String,
    pub branch: String,
    pub base_branch: String,
    #[serde(default = "origin")]
    pub remote: String,
    #[serde(default = "yes")]
    pub draft: bool,
    /// Separate authorization for possible CI side effects, never a quota attestation.
    #[serde(default)]
    pub ci_authorized: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Executor {
    Command {
        argv: Vec<String>,
    },
    Codex {
        permission: Permission,
        #[serde(default)]
        model: Option<String>,
    },
    Claude {
        permission: Permission,
        #[serde(default)]
        model: Option<String>,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ReadOnly,
    WorkspaceWrite,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Acceptance {
    Gate { id: String },
    Artifact { path: PathBuf, sha256: String },
    Human { id: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub id: String,
    pub argv: Vec<String>,
    /// Evaluator source files, outside every task worktree, pinned by digest.
    pub inputs: BTreeMap<PathBuf, String>,
    #[serde(default = "gate_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub cache: GateCache,
}
/// Admission policy. Missing counters are unknown, never a zero cost estimate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Budget {
    pub max_tokens: Option<u64>,
    pub max_active_seconds: Option<u64>,
    pub max_attempts: Option<u32>,
    pub max_concurrency: Option<u32>,
    pub max_cost_micros: Option<u64>,
    pub actions: Actions,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actions {
    #[default]
    Off,
    On,
}
impl Budget {
    fn validate_limits(&self) -> Result<()> {
        ensure!(
            self.max_concurrency.is_none_or(|n| (1..=32).contains(&n)),
            "max_concurrency must be 1..=32"
        );
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.validate_limits()?;
        ensure!(
            self.actions == Actions::Off,
            "CI activation requires separate publication CI authorization; unavailable to task execution"
        );
        Ok(())
    }
}
/// Opt-in deterministic local gates only. All inherited environment is hashed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GateCache {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub environment: BTreeMap<String, String>,
    /// Absolute local executable/configuration paths pinned by digest.
    pub toolchain: BTreeMap<PathBuf, String>,
}
impl Default for GateCache {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_seconds: 300,
            environment: BTreeMap::new(),
            toolchain: BTreeMap::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    #[serde(default)]
    pub required: bool,
    pub endpoint: Option<String>,
    pub workspace: Option<String>,
    /// Exact repo ID to ai-memory project mapping; no inferred cross-project fallback.
    #[serde(default)]
    pub projects: BTreeMap<String, String>,
    pub token_env: Option<String>,
}

pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}
pub fn local_path(p: &Path) -> bool {
    !p.as_os_str().is_empty()
        && p.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}
pub fn valid_hash(h: &str) -> bool {
    h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Manifest {
    pub fn load(path: &Path) -> Result<(Self, String)> {
        let bytes = std::fs::read(path).context("read manifest")?;
        ensure!(bytes.len() <= 1_048_576, "manifest exceeds 1 MiB");
        let m: Self = serde_yaml::from_slice(&bytes).context("invalid manifest YAML")?;
        m.validate()?;
        Ok((m, crate::digest(&bytes)))
    }
    pub fn validate(&self) -> Result<Vec<String>> {
        ensure!(
            (1..=32).contains(&self.execution.max_parallel),
            "max_parallel must be 1..=32"
        );
        ensure!(
            (1..=10).contains(&self.review.max_iterations)
                && (1..=3600).contains(&self.review.timeout_seconds),
            "invalid review limits"
        );
        self.context.validate()?;
        // M08 publication CI consent is separate from task execution budgets.
        self.budget.validate_limits()?;
        ensure!(
            self.budget.actions == Actions::Off
                || self
                    .automation
                    .publish
                    .as_ref()
                    .is_some_and(|p| p.ci_authorized),
            "CI activation requires separate publication CI authorization"
        );
        if let Some(source) = &self.source {
            source.validate()?;
            ensure!(
                self.repos
                    .values()
                    .any(|r| r.specs.contains(&source.spec_id)),
                "issue spec must be referenced by a repository"
            );
            if let Some(publish) = &self.automation.publish {
                ensure!(
                    publish.repository.eq_ignore_ascii_case(&source.repository),
                    "issue/publication repository mismatch"
                );
            }
        }
        ensure!(self.schema_version == 1, "unsupported schema_version");
        ensure!(valid_id(&self.release_id), "invalid release_id");
        ensure!(
            !self.repos.is_empty() && !self.tasks.is_empty(),
            "repos and tasks cannot be empty"
        );
        ensure!(self.tasks.len() <= 1000, "too many tasks");
        if let Some(p) = &self.automation.publish {
            ensure!(
                self.repos.len() == 1 && self.automation.require_clean_worktree,
                "draft automation requires one initially clean worktree"
            );
            ensure!(
                p.draft
                    && p.repository.split('/').count() == 2
                    && p.repository.split('/').all(valid_id),
                "invalid draft PR target"
            );
            ensure!(
                valid_id(&p.remote)
                    && !p.branch.is_empty()
                    && !p.base_branch.is_empty()
                    && p.branch != p.base_branch,
                "invalid draft PR branch/remote"
            );
        }
        for (id, repo) in &self.repos {
            ensure!(
                valid_id(id) && !repo.path.as_os_str().is_empty(),
                "invalid repo"
            );
            for spec in &repo.specs {
                ensure!(valid_id(spec), "invalid spec reference");
            }
        }
        let mut gates = BTreeSet::new();
        for g in &self.gates {
            ensure!(
                !g.cache.enabled || (1..=86400).contains(&g.cache.ttl_seconds),
                "cache TTL must be 1..=86400"
            );
            ensure!(
                g.cache.toolchain.values().all(|h| valid_hash(h)),
                "invalid toolchain digest"
            );
            ensure!(
                g.cache
                    .environment
                    .keys()
                    .all(|k| !k.is_empty() && !k.contains(['=', '\0'])),
                "invalid environment key"
            );
            ensure!(
                valid_id(&g.id) && gates.insert(&g.id),
                "duplicate or invalid gate ID"
            );
            ensure!(
                !g.argv.is_empty() && !g.argv[0].is_empty(),
                "empty gate command"
            );
            ensure!(
                (1..=3600).contains(&g.timeout_seconds),
                "invalid gate timeout"
            );
            ensure!(
                !g.inputs.is_empty(),
                "gate requires pinned evaluator inputs"
            );
            for hash in g.inputs.values() {
                ensure!(valid_hash(hash), "invalid evaluator digest");
            }
        }
        let mut ids = BTreeSet::new();
        for t in &self.tasks {
            t.budget.validate()?;
            if let Some(context) = &t.context {
                context.validate()?;
            }
            ensure!(
                valid_id(&t.id) && ids.insert(&t.id),
                "duplicate or invalid task ID"
            );
            ensure!(self.repos.contains_key(&t.repo), "unknown repo in {}", t.id);
            ensure!(
                !t.description.trim().is_empty() && !t.acceptance.is_empty(),
                "task {} requires description and evidence acceptance",
                t.id
            );
            ensure!(
                (1..=86400).contains(&t.timeout_seconds) && (1..=3).contains(&t.max_attempts),
                "invalid task limits"
            );
            if let Executor::Command { argv } = &t.executor {
                ensure!(!argv.is_empty() && !argv[0].is_empty(), "empty command");
            }
            for a in &t.acceptance {
                match a {
                    Acceptance::Gate { id } => ensure!(gates.contains(id), "unknown gate {id}"),
                    Acceptance::Artifact { path, sha256 } => ensure!(
                        local_path(path) && valid_hash(sha256),
                        "artifact requires safe relative path and SHA-256"
                    ),
                    Acceptance::Human { id } => ensure!(valid_id(id), "invalid human gate ID"),
                }
            }
        }
        for t in &self.tasks {
            let mut deps = BTreeSet::new();
            for dep in &t.dependencies {
                ensure!(
                    ids.contains(dep) && deps.insert(dep) && dep != &t.id,
                    "invalid dependency {dep}"
                );
            }
        }
        if self.memory.required {
            ensure!(
                self.memory.endpoint.is_some() && self.memory.workspace.is_some(),
                "required memory needs endpoint and workspace"
            );
        }
        if self.memory.endpoint.is_some() {
            ensure!(
                self.memory.workspace.as_ref().is_some_and(|w| valid_id(w)),
                "memory needs valid workspace"
            );
            ensure!(
                self.memory.projects.len() == self.repos.len(),
                "memory requires exact repo-root project mapping"
            );
            let projects: BTreeSet<_> = self.memory.projects.values().collect();
            ensure!(
                projects.len() == self.repos.len(),
                "memory projects must be isolated per repo"
            );
            for key in self.repos.keys() {
                ensure!(
                    self.memory.projects.get(key).is_some_and(|s| valid_id(s)),
                    "missing memory project"
                );
            }
        }
        let mut done = BTreeSet::new();
        let mut ordered = Vec::new();
        while ordered.len() < self.tasks.len() {
            let Some(t) = self
                .tasks
                .iter()
                .filter(|t| {
                    !done.contains(&t.id) && t.dependencies.iter().all(|d| done.contains(d))
                })
                .min_by_key(|t| t.priority)
            else {
                bail!("dependency cycle")
            };
            done.insert(t.id.clone());
            ordered.push(t.id.clone());
        }
        Ok(ordered)
    }
    pub fn roots(&self, base: &Path) -> Result<BTreeMap<String, PathBuf>> {
        let mut roots = BTreeMap::new();
        for (id, r) in &self.repos {
            let p = base
                .join(&r.path)
                .canonicalize()
                .with_context(|| format!("repo {id} unavailable"))?;
            let top = crate::git(&p, &["rev-parse", "--show-toplevel"])?;
            ensure!(
                Path::new(top.trim()).canonicalize()? == p,
                "repo must be a Git worktree root"
            );
            ensure!(
                !roots
                    .values()
                    .any(|other: &PathBuf| p.starts_with(other) || other.starts_with(&p)),
                "overlapping worktrees"
            );
            for s in &r.specs {
                for file in ["spec.md", "plan.md", "tasks.md"] {
                    ensure!(
                        p.join("specs").join(s).join(file).is_file(),
                        "missing spec {s}/{file}"
                    );
                }
            }
            roots.insert(id.clone(), p);
        }
        Ok(roots)
    }
    pub fn verify_evaluators(&self, base: &Path, roots: &BTreeMap<String, PathBuf>) -> Result<()> {
        for g in &self.gates {
            for (p, expected) in &g.inputs {
                let actual = base.join(p).canonicalize().context("evaluator missing")?;
                ensure!(
                    !roots.values().any(|r| actual.starts_with(r)),
                    "evaluator must be outside agent-writable repositories"
                );
                ensure!(
                    crate::digest(&std::fs::read(actual)?) == expected.to_lowercase(),
                    "evaluator digest mismatch"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Manifest {
        serde_yaml::from_str("schema_version: 1\nrelease_id: test\nrepos: { app: {path: .} }\ntasks:\n- id: a\n  repo: app\n  description: check\n  executor: {kind: command, argv: [echo]}\n  acceptance: [{kind: human, id: review}]\n").unwrap()
    }
    #[test]
    fn observability_is_explicit_and_strict() {
        assert_eq!(sample().automation.observability.langsmith, TraceMode::Off);
        let config: Automation =
            serde_yaml::from_str("observability: {langsmith: optional}").unwrap();
        assert_eq!(config.observability.langsmith, TraceMode::Optional);
        assert!(serde_yaml::from_str::<Automation>("observability: {langsmith: enabled}").is_err());
        assert!(
            serde_yaml::from_str::<Automation>(
                "observability: {langsmith: optional, api_key: forbidden}"
            )
            .is_err()
        );
    }
    #[test]
    fn context_is_additive_explicit_and_strict() {
        let mut m = sample();
        assert_eq!(m.context.profile, ContextProfile::Full);
        m.context =
            serde_yaml::from_str("profile: bounded\nbudget_tokens: 2000\nsources: [src/lib.rs]")
                .unwrap();
        m.validate().unwrap();
        m.context.budget_tokens = None;
        assert!(m.validate().is_err());
        m.context.budget_tokens = Some(2000);
        m.context.sources = vec!["../other-project".into()];
        assert!(m.validate().is_err());
        assert!(serde_yaml::from_str::<ContextPolicy>("profile: cheap").is_err());
        assert!(serde_yaml::from_str::<ContextPolicy>("profile: bounded\nmodel: other").is_err());
    }
    #[test]
    fn rejects_cycles_duplicates_and_unknown_refs() {
        let mut m = sample();
        m.validate().unwrap();
        m.tasks[0].dependencies.push("missing".into());
        assert!(m.validate().is_err());
        m.tasks[0].dependencies = vec!["a".into()];
        assert!(m.validate().is_err());
        m.tasks[0].dependencies.clear();
        m.tasks.push(m.tasks[0].clone());
        assert!(m.validate().is_err());
    }
    #[test]
    fn textual_completion_is_not_acceptance() {
        let mut m = sample();
        m.tasks[0].description = "All done, tests passed".into();
        m.tasks[0].acceptance.clear();
        assert!(m.validate().is_err());
    }
    #[test]
    fn rejects_unpinned_and_escaping_artifacts() {
        let mut m = sample();
        m.tasks[0].acceptance = vec![Acceptance::Artifact {
            path: "../evil".into(),
            sha256: "a".repeat(64),
        }];
        assert!(m.validate().is_err());
    }
}
