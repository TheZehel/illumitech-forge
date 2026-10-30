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

struct Signals {
    receiver: watch::Receiver<bool>,
    task: tokio::task::JoinHandle<()>,
}
impl Signals {
    fn new() -> Self {
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
    command(
        root,
        std::iter::once("gh")
            .chain(args.iter().copied())
            .map(String::from)
            .collect(),
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
    let info: Value = serde_json::from_str(
        &gh(
            root,
            &["repo", "view", repo, "--json", "visibility"],
            cancel,
        )
        .await?,
    )?;
    visibility.check(&info)?;
    let refs = command(
        root,
        vec![
            "git".into(),
            "ls-remote".into(),
            "origin".into(),
            format!("refs/tags/{tag}"),
            format!("refs/tags/{tag}^{{}}"),
        ],
        cancel,
    )
    .await?;
    let expected = &run.revisions.values().next().unwrap().commit;
    let actual = refs
        .lines()
        .find(|l| l.ends_with("^{}"))
        .or_else(|| refs.lines().next())
        .and_then(|l| l.split_whitespace().next());
    ensure!(
        actual == Some(expected.as_str()),
        "remote tag must identify the evaluated commit"
    );
    let mut argv = vec![
        "gh".into(),
        "release".into(),
        "create".into(),
        tag.into(),
        "--repo".into(),
        repo.into(),
        "--verify-tag".into(),
        "--title".into(),
        format!("{} {tag}", repo.split('/').next_back().unwrap()),
        "--notes-file".into(),
        notes.display().to_string(),
    ];
    argv.extend(files.iter().map(|p| p.display().to_string()));
    let url = command(root, argv, cancel).await?;
    Store::open(&c.db, false)?.save(
        &run,
        "published",
        &json!({"repository":repo,"tag":tag,"url":url,"expected_visibility":visibility.as_str()}),
    )?;
    Ok(json!({"schema_version":1,"published":true,"url":url}))
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
        config.branch != config.base_branch && !config.branch.to_lowercase().contains("codex"),
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
            !identity.to_lowercase().contains("codex") && identity.contains('<'),
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
    let _locks = WorktreeLocks::acquire(&c.roots)?;
    let config = c
        .manifest
        .automation
        .publish
        .as_ref()
        .context("manifest has no draft PR contract")?;
    draft_preflight(c, config)?;
    let root = single_root(c)?;
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
    let run = engine::assert_ready(c, false)?;
    if dry {
        return Ok(
            json!({"schema_version":1,"status":"passed","dry_run":true,"draft":true,"repository":config.repository,"branch":config.branch,"identity":"configured_human","publication":"not_performed"}),
        );
    }
    if !git(root, &["status", "--porcelain"])?.trim().is_empty() {
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
        !attribution
            .lines()
            .take(2)
            .any(|l| l.to_lowercase().contains("codex"))
            && !attribution
                .lines()
                .any(|l| l.to_lowercase().starts_with("co-authored-by:")
                    && l.to_lowercase().contains("codex")),
        "agent authorship is forbidden"
    );
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
                "--json",
                "url,state,isDraft,baseRefName,headRefOid",
            ],
            cancel,
        )
        .await?,
    )?;
    let matches = prs.as_array().context("invalid PR list")?;
    ensure!(matches.len() <= 1, "ambiguous existing PRs for this branch");
    if let Some(pr) = matches.first() {
        ensure!(
            pr["state"] == "OPEN"
                && pr["isDraft"] == true
                && pr["baseRefName"] == config.base_branch
                && pr["headRefOid"] == commit.trim(),
            "existing PR differs; manual reconciliation required"
        );
        Store::open(&c.db, false)?.save(
            &latest,
            "draft_pr_published",
            &json!({"url":pr["url"],"commit":commit.trim()}),
        )?;
        return Ok(
            json!({"schema_version":1,"status":"passed","draft_pr":pr["url"],"reused":true}),
        );
    }
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
    let body =
        c.db.parent()
            .context("state parent")?
            .join(format!("{}-pr-body.md", run.release_id));
    std::fs::write(
        &body,
        format!(
            "Implements release `{}`. All declared gates passed on commit `{}`.\n\nThis is a draft PR. Human review and merge remain separate actions.\n",
            run.release_id,
            commit.trim()
        ),
    )?;
    let url = gh(
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
            &format!("feat: {}", run.release_id),
            "--body-file",
            &body.display().to_string(),
        ],
        cancel,
    )
    .await?;
    Store::open(&c.db, false)?.save(
        &latest,
        "draft_pr_published",
        &json!({"url":url,"commit":commit.trim()}),
    )?;
    Ok(json!({"schema_version":1,"status":"passed","draft_pr":url,"reused":false}))
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
