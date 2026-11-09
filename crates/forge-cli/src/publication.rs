use crate::engine::{self, Context};
use anyhow::{Context as _, Result, ensure};
use forge_adapters::process;
use forge_domain::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use tokio::sync::watch;

mod recovery;

/// Expected remote visibility is a check, never a request to change it.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
#[value(rename_all = "UPPER")]
pub enum ExpectedVisibility {
    #[default]
    Private,
    Public,
}
impl ExpectedVisibility {
    fn as_str(self) -> &'static str {
        match self {
            Self::Private => "PRIVATE",
            Self::Public => "PUBLIC",
        }
    }
    fn check(self, info: &Value) -> Result<()> {
        ensure!(
            info["visibility"].as_str() == Some(self.as_str()),
            "target repository visibility must match expected {}",
            self.as_str()
        );
        Ok(())
    }
}

pub(crate) struct Signals {
    pub(crate) receiver: watch::Receiver<bool>,
    task: tokio::task::JoinHandle<()>,
}
impl Signals {
    pub(crate) fn new() -> Self {
        let (tx, receiver) = watch::channel(false);
        let task = tokio::spawn(async move {
            #[cfg(unix)]
            {
                if let Ok(mut term) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    tokio::select! { _=process::interrupt()=>{}, _=term.recv()=>{} }
                } else {
                    let _ = process::interrupt().await;
                }
            }
            #[cfg(not(unix))]
            let _ = process::interrupt().await;
            let _ = tx.send(true);
        });
        Self { receiver, task }
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn command(root: &Path, argv: Vec<String>, cancel: &watch::Receiver<bool>) -> Result<String> {
    ensure!(!*cancel.borrow(), "publication cancelled");
    let r = process::run(&argv, root, 120, cancel.clone()).await?;
    ensure!(
        r.passed(),
        "{} failed ({}): {}",
        argv[0],
        r.reason,
        r.stderr
    );
    Ok(r.stdout.trim().into())
}
async fn gh(root: &Path, args: &[&str], cancel: &watch::Receiver<bool>) -> Result<String> {
    let mut pinned = args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    for index in 0..args.len() {
        if (index > 0 && args[index - 1] == "--repo")
            || (index == 2 && args[..2] == ["repo", "view"])
        {
            repository(args[index])?;
            pinned[index] = format!("github.com/{}", args[index]);
        }
    }
    command(
        root,
        std::iter::once("gh".to_owned()).chain(pinned).collect(),
        cancel,
    )
    .await
}
fn single_root(c: &Context) -> Result<&Path> {
    ensure!(
        c.roots.len() == 1,
        "publication requires one repository per contract"
    );
    Ok(c.roots.values().next().unwrap())
}
fn repository(value: &str) -> Result<()> {
    ensure!(
        value.split('/').count() == 2 && value.split('/').all(valid_id),
        "invalid GitHub repository"
    );
    Ok(())
}
fn target(root: &Path, remote: &str, expected: &str) -> Result<()> {
    repository(expected)?;
    ensure!(valid_id(remote), "invalid remote name");
    // Git expands insteadOf/pushInsteadOf and pushurl. Validate the effective destinations.
    for args in [
        vec!["remote", "get-url", "--all", remote],
        vec!["remote", "get-url", "--push", "--all", remote],
    ] {
        let urls = git(root, &args)?;
        ensure!(
            urls.lines().count() == 1,
            "one effective remote URL required"
        );
        let url = urls.trim();
        let slug = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("git@github.com:"));
        ensure!(
            slug.is_some_and(|s| s
                .strip_suffix(".git")
                .unwrap_or(s)
                .eq_ignore_ascii_case(expected)),
            "repository does not match the effective Git remote destination"
        );
    }
    Ok(())
}
pub fn archive_files(assets: &Path) -> Result<Vec<PathBuf>> {
    let assets = assets.canonicalize()?;
    ensure!(assets.is_dir(), "assets must be a directory");
    let sumfile = assets.join("SHA256SUMS");
    ensure!(
        !sumfile.symlink_metadata()?.file_type().is_symlink(),
        "checksum symlink refused"
    );
    let sums = std::fs::read_to_string(&sumfile)?;
    let mut files = Vec::new();
    let mut names = BTreeSet::new();
    for line in sums.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, name) = line
            .split_once("  ")
            .context("SHA256SUMS requires SHA256, two spaces, filename")?;
        ensure!(
            valid_hash(hash)
                && !name.is_empty()
                && Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name)
                && !name.starts_with('.')
                && name != "SHA256SUMS"
                && !name.contains(['/', '\\', '\n', '\r'])
                && names.insert(name.to_owned()),
            "invalid/duplicate checksum filename"
        );
        let path = assets.join(name);
        ensure!(
            path.symlink_metadata()?.is_file(),
            "asset must be a regular file, not a symlink"
        );
        ensure!(
            digest(&std::fs::read(&path)?) == hash.to_lowercase(),
            "asset checksum mismatch: {name}"
        );
        files.push(path);
    }
    ensure!(!files.is_empty(), "at least one checksummed asset required");
    for entry in std::fs::read_dir(&assets)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        // Per-archive checksum files are permitted as build outputs, but aren't uploaded.
        ensure!(
            name == "SHA256SUMS"
                || names.contains(&name)
                || name
                    .strip_suffix(".sha256")
                    .is_some_and(|n| names.contains(n)),
            "unlisted asset: {name}"
        );
    }
    files.push(sumfile);
    Ok(files)
}

pub async fn release(
    c: &Context,
    repo: &str,
    tag: &str,
    assets: &Path,
    notes: &Path,
    dry: bool,
    visibility: ExpectedVisibility,
) -> Result<Value> {
    crate::observability::phase(
        c,
        dry,
        forge_adapters::observability::Phase::Publish,
        Box::pin(release_inner(c, repo, tag, assets, notes, dry, visibility)),
    )
    .await
}
async fn release_inner(
    c: &Context,
    repo: &str,
    tag: &str,
    assets: &Path,
    notes: &Path,
    dry: bool,
    visibility: ExpectedVisibility,
) -> Result<Value> {
    let signals = Signals::new();
    let cancel = &signals.receiver;
    let _locks = WorktreeLocks::acquire(&c.roots)?;
    let run = engine::assert_publishable(c)?;
    if !dry {
        remote_write_policy(c)?;
        ensure!(
            c.manifest
                .automation
                .publish
                .as_ref()
                .is_some_and(|p| p.repository.eq_ignore_ascii_case(repo)),
            "CI authorization repository does not match release target"
        );
    }
    let root = single_root(c)?;
    target(root, "origin", repo)?;
    ensure!(
        git(root, &["status", "--porcelain"])?.trim().is_empty(),
        "release publication requires a clean evaluated commit"
    );
    ensure!(!tag.starts_with('-'), "invalid tag");
    git(root, &["check-ref-format", &format!("refs/tags/{tag}")])?;
    let notes = notes.canonicalize()?;
    ensure!(notes.is_file(), "release notes missing");
    let files = archive_files(assets)?;
    if dry {
        return Ok(
            json!({"schema_version":1,"status":"passed","dry_run":true,"expected_visibility":visibility.as_str(),"remote_visibility_checked":false,"repository":repo,"tag":tag,"assets":files,"notes":notes}),
        );
    }
    recovery::publish(
        c,
        &run,
        recovery::Inputs {
            repo,
            tag,
            assets,
            notes: &notes,
            visibility,
        },
        cancel,
    )
    .await
}

pub fn draft_preflight(c: &Context, config: &DraftPr) -> Result<()> {
    let root = single_root(c)?;
    target(root, &config.remote, &config.repository)?;
    ensure!(
        config.draft,
        "automatic publication supports draft PRs only"
    );
    for name in [&config.branch, &config.base_branch] {
        git(root, &["check-ref-format", &format!("refs/heads/{name}")])?;
    }
    ensure!(
        config.branch != config.base_branch
            && human_label(&config.branch)
            && human_label(&c.manifest.release_id),
        "use a dedicated feature branch"
    );
    ensure!(
        git(root, &["branch", "--show-current"])?.trim() == config.branch,
        "checkout is not on the declared publication branch"
    );
    ensure!(
        git(root, &["rev-parse", "--git-dir"])?.trim()
            != git(root, &["rev-parse", "--git-common-dir"])?.trim(),
        "draft automation requires an isolated Git worktree"
    );
    for field in ["GIT_AUTHOR_IDENT", "GIT_COMMITTER_IDENT"] {
        let identity = git(root, &["var", field])?;
        ensure!(
            human_label(&identity) && !identity.contains("[bot]") && identity.contains('<'),
            "configure a human Git identity before publication"
        );
    }
    Ok(())
}

pub async fn draft_pr(c: &Context, dry: bool) -> Result<Value> {
    let signals = Signals::new();
    draft_pr_with_cancel(c, dry, &signals.receiver).await
}

pub async fn draft_pr_with_cancel(
    c: &Context,
    dry: bool,
    cancel: &watch::Receiver<bool>,
) -> Result<Value> {
    crate::observability::phase(
        c,
        dry,
        forge_adapters::observability::Phase::Publish,
        Box::pin(draft_pr_inner(c, dry, cancel)),
    )
    .await
}
async fn draft_pr_inner(c: &Context, dry: bool, cancel: &watch::Receiver<bool>) -> Result<Value> {
    ensure!(!*cancel.borrow(), "publication cancelled");
    let config = c
        .manifest
        .automation
        .publish
        .as_ref()
        .context("manifest has no draft PR contract")?;
    draft_preflight(c, config)?;
    let root = single_root(c)?;
    if dry {
        return preview(c);
    }
    let _locks = WorktreeLocks::acquire(&c.roots)?;
    let key = format!("draft-pr:{}", c.manifest.release_id);
    let store = Store::open(&c.db, true)?;
    let journal = store.metadata(&key)?;
    // A journal is the only authority to recover a commit made after evaluated evidence.
    if let Some(j) = &journal {
        ensure!(
            j["manifest_hash"] == c.hash,
            "publication contract changed; review journal before proceeding"
        );
        if j["phase"] == "committing" || j["phase"] == "committed" {
            ensure!(
                git(root, &["status", "--porcelain"])?.trim().is_empty(),
                "incomplete commit; review staged/worktree changes manually"
            );
            ensure!(
                git(root, &["rev-parse", "HEAD^{tree}"])?.trim()
                    == j["tree"].as_str().unwrap_or(""),
                "recovered commit tree differs from approved staged tree"
            );
            let head = git(root, &["rev-parse", "HEAD"])?;
            ensure!(
                git(root, &["rev-parse", "HEAD^"])?.trim() == j["parent"].as_str().unwrap_or(""),
                "unexpected commit ancestry"
            );
            if !dry {
                let run = engine::reevaluate_commit(c, cancel.clone()).await?;
                Store::open(&c.db, false)?.set_metadata(
                    &key,
                    &json!({"phase":"evaluated","manifest_hash":c.hash,"commit":head.trim()}),
                )?;
                ensure!(
                    run.status == "passed",
                    "new commit requires passing gates/current human acceptance; resume explicitly"
                );
            }
        }
    }
    engine::assert_ready(c, false)?;
    if !git(root, &["status", "--porcelain"])?.trim().is_empty() {
        remote_write_policy(c)?;
        let mut changed = git(
            root,
            &[
                "ls-files",
                "-z",
                "--modified",
                "--others",
                "--exclude-standard",
            ],
        )?;
        changed.push_str(&git(root, &["diff", "--cached", "--name-only", "-z"])?);
        for file in changed.split('\0').filter(|p| !p.is_empty()) {
            ensure!(
                !Path::new(file).components().any(|p| p
                    .as_os_str()
                    .to_str()
                    .is_some_and(|s| s.starts_with(".env")
                        || [".aws", ".ssh", "credentials.json", "auth.json"].contains(&s))),
                "sensitive file path requires manual publication review"
            );
        }
        command(
            root,
            vec!["git".into(), "add".into(), "--all".into()],
            cancel,
        )
        .await?;
        let diff = command(
            root,
            vec!["git".into(), "diff".into(), "--cached".into()],
            cancel,
        )
        .await?;
        ensure!(
            !diff.lines().any(|l| l.starts_with('+')
                && l.contains("-----BEGIN ")
                && l.contains("PRIVATE KEY-----")),
            "private key detected; staged changes preserved for review"
        );
        let tree = git(root, &["write-tree"])?;
        let parent = git(root, &["rev-parse", "HEAD"])?;
        Store::open(&c.db,false)?.set_metadata(&key,&json!({"phase":"committing","manifest_hash":c.hash,"tree":tree.trim(),"parent":parent.trim()}))?;
        command(
            root,
            vec![
                "git".into(),
                "commit".into(),
                "-m".into(),
                format!("feat(forge): {}", c.manifest.release_id),
            ],
            cancel,
        )
        .await?;
        ensure!(
            git(root, &["rev-parse", "HEAD^{tree}"])?.trim() == tree.trim(),
            "commit hook changed evaluated content"
        );
        let checked = engine::reevaluate_commit(c, cancel.clone()).await?;
        Store::open(&c.db,false)?.set_metadata(&key,&json!({"phase":"evaluated","manifest_hash":c.hash,"commit":git(root,&["rev-parse","HEAD"])?.trim()}))?;
        ensure!(
            checked.status == "passed",
            "new commit requires passing gates/current human acceptance; resume explicitly"
        );
    }
    let latest = engine::assert_ready(c, false)?;
    let commit = git(root, &["rev-parse", "HEAD"])?;
    let attribution = git(root, &["log", "-1", "--format=%an%n%cn%n%B"])?;
    ensure!(
        attribution
            .lines()
            .take(2)
            .all(|line| human_label(line) && !line.to_lowercase().contains("[bot]"))
            && !attribution
                .lines()
                .any(|l| l.to_lowercase().starts_with("co-authored-by:") && !human_label(l)),
        "agent authorship is forbidden"
    );
    let title = format!("feat: {}", c.manifest.release_id);
    let body_text = draft_body(c, commit.trim(), &latest)?;
    if let Some(journal) = &journal
        && ["creating", "uncertain", "published"]
            .iter()
            .any(|phase| journal["phase"] == *phase)
    {
        ensure!(
            journal["commit"] == commit.trim()
                && journal["body_sha256"] == digest(body_text.as_bytes()),
            "publication revision/body changed; journal requires explicit reconciliation"
        );
    }
    let existing = find_draft(root, config, commit.trim(), &title, &body_text, cancel).await?;
    if let Some(pr) = existing {
        return record_draft(c, &latest, &key, &pr, commit.trim(), &body_text, true);
    }
    if let Some(j) = &journal {
        ensure!(
            !["creating", "uncertain", "published"]
                .iter()
                .any(|phase| j["phase"] == *phase),
            "previous creation/publication has no matching remote PR; explicit operator reconciliation required, no retry create"
        );
    }
    remote_write_policy(c)?;
    // Re-read the branch before any write. Never overwrite an unexpected remote revision.
    let refs = command(
        root,
        vec![
            "git".into(),
            "ls-remote".into(),
            config.remote.clone(),
            format!("refs/heads/{}", config.branch),
        ],
        cancel,
    )
    .await?;
    ensure!(refs.lines().count() <= 1, "ambiguous remote branch");
    if !refs.is_empty() {
        ensure!(
            refs.split_whitespace().next() == Some(commit.trim()),
            "remote branch changed; manual reconciliation required"
        );
    } else {
        engine::assert_ready(c, false)?;
        draft_preflight(c, config)?;
        command(
            root,
            vec![
                "git".into(),
                "push".into(),
                config.remote.clone(),
                format!("HEAD:refs/heads/{}", config.branch),
            ],
            cancel,
        )
        .await?;
    }
    let refs = command(
        root,
        vec![
            "git".into(),
            "ls-remote".into(),
            config.remote.clone(),
            format!("refs/heads/{}", config.branch),
        ],
        cancel,
    )
    .await?;
    ensure!(
        refs.lines().count() == 1 && refs.split_whitespace().next() == Some(commit.trim()),
        "pushed branch readback differs from evaluated revision"
    );
    // A PR may have appeared during push. Query once more before create.
    if let Some(pr) = find_draft(root, config, commit.trim(), &title, &body_text, cancel).await? {
        return record_draft(c, &latest, &key, &pr, commit.trim(), &body_text, true);
    }
    engine::assert_ready(c, false)?;
    let body =
        c.db.parent()
            .context("state parent")?
            .join(format!("{}-pr-body.md", c.manifest.release_id));
    // Only trusted generated metadata is published, never the raw issue body or comments.
    std::fs::write(&body, &body_text)?;
    Store::open(&c.db, false)?.set_metadata(&key, &json!({"phase":"creating","manifest_hash":c.hash,"commit":commit.trim(),"body_sha256":digest(body_text.as_bytes())}))?;
    draft_preflight(c, config)?;
    let create = gh(
        root,
        &[
            "pr",
            "create",
            "--repo",
            &config.repository,
            "--draft",
            "--base",
            &config.base_branch,
            "--head",
            &config.branch,
            "--title",
            &title,
            "--body-file",
            &body.display().to_string(),
        ],
        cancel,
    )
    .await;
    // Even a successful CLI response is not publication evidence. Query remote state.
    match find_draft(root, config, commit.trim(), &title, &body_text, cancel).await {
        Ok(Some(pr)) => record_draft(
            c,
            &latest,
            &key,
            &pr,
            commit.trim(),
            &body_text,
            create.is_err(),
        ),
        result => {
            Store::open(&c.db, false)?.set_metadata(&key, &json!({"phase":"uncertain","manifest_hash":c.hash,"commit":commit.trim(),"body_sha256":digest(body_text.as_bytes())}))?;
            result?;
            anyhow::bail!(
                "creation outcome unverified; journal retained, retry will query remote before any create"
            )
        }
    }
}

fn human_label(value: &str) -> bool {
    let value = value.to_lowercase();
    !["codex", "claude", "generated by", "agent author"]
        .iter()
        .any(|s| value.contains(s))
}
fn remote_write_policy(c: &Context) -> Result<()> {
    ensure!(
        c.manifest.budget.actions == Actions::On
            && c.manifest
                .automation
                .publish
                .as_ref()
                .is_some_and(|p| p.ci_authorized),
        "remote writes blocked by budget.actions=off or missing explicit CI authorization; push/PR/release events may activate Actions; skip-ci is not a guarantee"
    );
    Ok(())
}
/// Complete, local-only preview. Readiness and provider quota are independent facts.
pub fn preview(c: &Context) -> Result<Value> {
    let Some(config) = &c.manifest.automation.publish else {
        return Ok(json!({"authorized":false,"publication":"not_authorized","commands":[]}));
    };
    let root = single_root(c)?;
    let preflight = draft_preflight(c, config);
    let author = git(root, &["var", "GIT_AUTHOR_IDENT"]).ok();
    let committer = git(root, &["var", "GIT_COMMITTER_IDENT"]).ok();
    let policy = remote_write_policy(c);
    let ready = engine::assert_ready(c, false);
    let clean = git(root, &["status", "--porcelain"])?.trim().is_empty();
    let body_preview = if clean {
        ready
            .as_ref()
            .ok()
            .map(|run| {
                draft_body(
                    c,
                    &run.revisions
                        .values()
                        .next()
                        .context("missing revision")?
                        .commit,
                    run,
                )
            })
            .transpose()?
    } else {
        None
    };
    let effective_remote = if preflight.is_ok() {
        Some(
            json!({"fetch":git(root, &["remote","get-url","--all",&config.remote])?.trim(),
            "push":git(root, &["remote","get-url","--push","--all",&config.remote])?.trim()}),
        )
    } else {
        None
    };
    Ok(
        json!({"schema_version":1,"dry_run":true,"draft":true,"repository":config.repository,"remote":config.remote,
        "branch":config.branch,"base_branch":config.base_branch,"effective_remote":effective_remote,
        "pr_title":format!("feat: {}",c.manifest.release_id),"pr_body":body_preview,
        "pr_body_condition":"exact body available only for a clean evaluated commit; new commits require fresh evidence/human review",
        "identity":{"author":author,"committer":committer},
        "permissions":{"commit_push_draft":true,"ci_authorized":config.ci_authorized,"actions":c.manifest.budget.actions,"merge":false,"release":false,"issue_mutation":false,"force_push":false},
        "remote_write_allowed":policy.is_ok(),"policy_block":policy.err().map(|e|e.to_string()),"provider_quota":"unverified","provider_readback":"not_performed",
        "preflight_passed":preflight.is_ok(),"preflight_block":preflight.err().map(|e|e.to_string()),"evidence_ready":ready.is_ok(),"evidence_block":ready.err().map(|e|e.to_string()),
        "commands":[["git","add","--all"],["git","commit","-m",&format!("feat(forge): {}",c.manifest.release_id)],
        ["gh","pr","list","--repo",&format!("github.com/{}",config.repository),"--head",&config.branch,"--state","all","--limit","100","--json",PR_FIELDS],
        ["git","ls-remote",&config.remote,&format!("refs/heads/{}",config.branch)],
        ["git","push",&config.remote,&format!("HEAD:refs/heads/{}",config.branch)],
        ["gh","pr","create","--repo",&format!("github.com/{}",config.repository),"--draft","--base",&config.base_branch,"--head",&config.branch,"--title",&format!("feat: {}",c.manifest.release_id),"--body-file",&c.db.parent().context("state parent")?.join(format!("{}-pr-body.md",c.manifest.release_id)).display().to_string()]],
        "command_conditions":"commit only if dirty; query before writes; reuse exact existing draft; push only absent branch; create only absent PR with no uncertain journal; read back after create",
        "traceability":traceability(c)?,"publication":"not_performed"}),
    )
}

pub fn traceability(c: &Context) -> Result<Value> {
    let read = || -> Result<Value> {
        if !c.db.exists() {
            return Ok(
                json!({"status":"not_run","publication_journal":null,"release_publication_journal":null,"evidence":null,
                "independent_review":{"status":"not_run","acceptance":"unchanged"}}),
            );
        }
        let store = Store::open(&c.db, true)?;
        let journal = store.metadata(&format!("draft-pr:{}", c.manifest.release_id))?;
        let release_journal =
            store.metadata(&format!("release-publication:{}", c.manifest.release_id))?;
        let evidence = store.load(&c.manifest.release_id)?.map(|run| {
            json!({"manifest_hash":run.manifest_hash,"evaluated_revisions":run.revisions,"status":run.status,
                "tasks":run.tasks.iter().map(|(id,t)|json!({"id":id,"status":t.status,"revision":t.revision,"evidence":t.evidence})).collect::<Vec<_>>()})
        });
        Ok(
            json!({"status":"available","publication_journal":journal,"release_publication_journal":release_journal,"evidence":evidence,"independent_review":crate::review::latest(c)?}),
        )
    };
    // Inspection must retain the duration/budget contracts for legacy/corrupt stores.
    // Unavailable readback conveys no approval or publication; mutations still fail closed.
    let snapshot = read().unwrap_or_else(|error| json!({"status":"unavailable","reason":error.to_string(),
        "publication_journal":null,"release_publication_journal":null,"evidence":null,"independent_review":{"status":"unavailable","acceptance":"unchanged"}}));
    Ok(
        json!({"operational_readback":{"status":snapshot["status"],"reason":snapshot["reason"]},
        "evidence":snapshot["evidence"],"issue":c.manifest.source,
        "specs":c.manifest.repos.iter().map(|(id,r)|json!({"repo":id,"specs":r.specs})).collect::<Vec<_>>(),
        "tasks":c.manifest.tasks.iter().map(|t|t.id.clone()).collect::<Vec<_>>(),"current_revisions":revisions(&c.roots)?,
        "independent_review":snapshot["independent_review"],"publication_journal":snapshot["publication_journal"],
        "release_publication_journal":snapshot["release_publication_journal"],
        "commit_pr_link":"only verified journal entries attest publication; current local commit may precede this demand"}),
    )
}
fn draft_body(c: &Context, commit: &str, run: &Run) -> Result<String> {
    let review = crate::review::latest(c)?;
    let review_hash = digest(&serde_json::to_vec(&review)?);
    let source = c
        .manifest
        .source
        .as_ref()
        .map(|s| {
            format!(
                "Selected issue: {}\nDemand SHA-256: {}\nSpec: {}\nProvenance: {:?}\n",
                s.url, s.content_sha256, s.spec_id, s.provenance
            )
        })
        .unwrap_or_default();
    Ok(format!(
        "Implements release `{}` at evaluated commit `{}`.\n\n{}\nTasks: {}\nEvidence SHA-256: {}\nManifest SHA-256: {}\nIndependent review status: {}\nReview SHA-256: {}\n\nThis is a draft PR. Human review, merge and release remain separate actions.",
        c.manifest.release_id,
        commit,
        source,
        c.manifest
            .tasks
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        evidence_hash(run)?,
        c.hash,
        review["status"].as_str().unwrap_or("unknown"),
        review_hash
    ))
}
fn evidence_hash(run: &Run) -> Result<String> {
    // Stable across resume/status timestamps; only revision-bound task evidence is included.
    Ok(digest(&serde_json::to_vec(
        &run.tasks
            .iter()
            .map(|(id, t)| (id, &t.revision, &t.evidence))
            .collect::<Vec<_>>(),
    )?))
}
const PR_FIELDS: &str =
    "url,state,isDraft,baseRefName,headRefOid,headRefName,headRepository,title,body";
async fn find_draft(
    root: &Path,
    config: &DraftPr,
    commit: &str,
    title: &str,
    body: &str,
    cancel: &watch::Receiver<bool>,
) -> Result<Option<Value>> {
    let prs: Value = serde_json::from_str(
        &gh(
            root,
            &[
                "pr",
                "list",
                "--repo",
                &config.repository,
                "--head",
                &config.branch,
                "--state",
                "all",
                "--limit",
                "100",
                "--json",
                PR_FIELDS,
            ],
            cancel,
        )
        .await?,
    )?;
    let matches = prs.as_array().context("invalid PR list")?;
    ensure!(matches.len() <= 1, "ambiguous existing PRs for this branch");
    let Some(pr) = matches.first() else {
        return Ok(None);
    };
    let url = pr["url"].as_str().context("PR URL missing")?;
    let prefix = format!("https://github.com/{}/pull/", config.repository);
    ensure!(
        url.get(..prefix.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(&prefix))
            && url.get(prefix.len()..).is_some_and(|n| !n.is_empty()
                && n.bytes().all(|c| c.is_ascii_digit())
                && n.parse::<u64>().is_ok_and(|n| n > 0)),
        "PR URL target mismatch"
    );
    ensure!(
        pr["state"] == "OPEN"
            && pr["isDraft"] == true
            && pr["baseRefName"] == config.base_branch
            && pr["headRefName"] == config.branch
            && pr["headRefOid"] == commit
            && pr["headRepository"]["nameWithOwner"]
                .as_str()
                .is_some_and(|r| r.eq_ignore_ascii_case(&config.repository))
            && pr["title"] == title
            && pr["body"] == body,
        "existing PR differs (state/draft/base/head/revision/title/body); manual reconciliation required"
    );
    Ok(Some(pr.clone()))
}
fn record_draft(
    c: &Context,
    run: &Run,
    key: &str,
    pr: &Value,
    commit: &str,
    body: &str,
    reused: bool,
) -> Result<Value> {
    engine::assert_ready(c, false)?;
    let mut store = Store::open(&c.db, false)?;
    if let Some(journal) = store.metadata(key)? {
        ensure!(
            journal["url"].is_null() || journal["url"] == pr["url"],
            "remote PR identity changed; journal preserved"
        );
    }
    let entry = json!({"phase":"published","manifest_hash":c.hash,"commit":commit,"url":pr["url"],"body_sha256":digest(body.as_bytes()),"issue":c.manifest.source,"evidence_sha256":evidence_hash(run)?,"readback":"verified"});
    store.set_metadata(key, &entry)?;
    store.save(run, "draft_pr_published", &entry)?;
    Ok(
        json!({"schema_version":1,"status":"passed","draft_pr":pr["url"],"commit":commit,"reused":reused,"readback":"verified","traceability":traceability(c)?}),
    )
}

#[cfg(test)]
mod visibility_tests {
    use super::*;

    #[test]
    fn visibility_is_explicit_and_fails_closed() {
        for (expected, accepted) in [
            (ExpectedVisibility::default(), "PRIVATE"),
            (ExpectedVisibility::Public, "PUBLIC"),
        ] {
            for actual in ["PRIVATE", "PUBLIC", "INTERNAL", "private", ""] {
                assert_eq!(
                    expected.check(&json!({"visibility": actual})).is_ok(),
                    actual == accepted
                );
            }
            assert!(expected.check(&json!({})).is_err());
            assert!(expected.check(&json!({"visibility": null})).is_err());
        }
    }
}
