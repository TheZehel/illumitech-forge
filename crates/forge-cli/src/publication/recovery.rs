//! Release recovery uses its own operator-owned metadata, never the draft-PR journal.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) struct Inputs<'a> {
    pub repo: &'a str,
    pub tag: &'a str,
    pub assets: &'a Path,
    pub notes: &'a Path,
    pub visibility: ExpectedVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Asset {
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Contract {
    manifest_sha256: String,
    source: Value,
    evidence_sha256: String,
    repository: String,
    tag: String,
    commit: String,
    title: String,
    visibility: String,
    notes_sha256: String,
    assets: BTreeMap<String, Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum Intent {
    Create,
    Upload { name: String },
    DeleteStarter { name: String, id: u64 },
    Publish,
}

#[derive(Debug, Serialize, Deserialize)]
struct Journal {
    schema_version: u32,
    contract: Contract,
    release_id: Option<u64>,
    intent: Option<Intent>,
    complete: bool,
    verified_assets: BTreeMap<String, u64>,
}

#[derive(Debug, Deserialize)]
struct RemoteRelease {
    id: u64,
    tag_name: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct RemoteAsset {
    id: u64,
    name: String,
    size: u64,
    state: String,
}

impl Contract {
    fn capture(c: &Context, run: &Run, input: &Inputs<'_>) -> Result<Self> {
        let mut assets = BTreeMap::new();
        for path in archive_files(input.assets)? {
            let bytes = std::fs::read(&path)?;
            assets.insert(
                path.file_name()
                    .unwrap()
                    .to_str()
                    .context("non-UTF8 asset")?
                    .into(),
                Asset {
                    size: bytes.len() as u64,
                    sha256: digest(&bytes),
                },
            );
        }
        Ok(Self {
            manifest_sha256: c.hash.clone(),
            source: serde_json::to_value(&run.revisions)?,
            evidence_sha256: evidence_hash(run)?,
            repository: input.repo.into(),
            tag: input.tag.into(),
            commit: run.revisions.values().next().unwrap().commit.clone(),
            title: format!(
                "{} {}",
                input.repo.split('/').next_back().unwrap(),
                input.tag
            ),
            visibility: input.visibility.as_str().into(),
            notes_sha256: digest(&std::fs::read(input.notes)?),
            assets,
        })
    }
}

fn persist(c: &Context, key: &str, journal: &Journal) -> Result<()> {
    let body = serde_json::to_value(journal)?;
    let mut store = Store::open(&c.db, false)?;
    if store.metadata(key)?.as_ref() != Some(&body) {
        // Each transition and its evidence are atomic. Completed retries add no transition.
        store.journal_metadata(&c.manifest.release_id, key, &body)?;
    }
    Ok(())
}

async fn api<T: serde::de::DeserializeOwned>(
    root: &Path,
    endpoint: &str,
    cancel: &watch::Receiver<bool>,
) -> Result<Vec<T>> {
    // Complete paginated collections avoid guessing absence from untyped CLI errors.
    // All API calls explicitly override GH_HOST; release/repo calls use qualified repos.
    let response = command(
        root,
        vec![
            "gh".into(),
            "api".into(),
            "--hostname".into(),
            "github.com".into(),
            endpoint.into(),
            "--paginate".into(),
            "--slurp".into(),
        ],
        cancel,
    )
    .await?;
    let pages: Vec<Vec<T>> =
        serde_json::from_str(&response).context("invalid GitHub collection")?;
    ensure!(!pages.is_empty(), "incomplete GitHub collection");
    Ok(pages.into_iter().flatten().collect())
}

async fn preflight(
    c: &Context,
    input: &Inputs<'_>,
    contract: &Contract,
    cancel: &watch::Receiver<bool>,
) -> Result<()> {
    ensure!(!*cancel.borrow(), "publication cancelled");
    let run = engine::assert_publishable(c)?;
    ensure!(
        Contract::capture(c, &run, input)? == *contract,
        "publication inputs changed; journal preserved"
    );
    let root = single_root(c)?;
    target(root, "origin", input.repo)?;
    ensure!(
        git(root, &["status", "--porcelain"])?.trim().is_empty(),
        "release publication requires a clean evaluated commit"
    );
    let info: Value = serde_json::from_str(
        &gh(
            root,
            &["repo", "view", input.repo, "--json", "visibility"],
            cancel,
        )
        .await?,
    )?;
    input.visibility.check(&info)?;
    let refs = command(
        root,
        vec![
            "git".into(),
            "ls-remote".into(),
            "origin".into(),
            format!("refs/tags/{}", input.tag),
            format!("refs/tags/{}^{{}}", input.tag),
        ],
        cancel,
    )
    .await?;
    let tag_ref = format!("refs/tags/{}", input.tag);
    let peeled_ref = format!("{tag_ref}^{{}}");
    let mut found = BTreeMap::new();
    for line in refs.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        ensure!(
            parts.len() == 2
                && (parts[1] == tag_ref || parts[1] == peeled_ref)
                && matches!(parts[0].len(), 40 | 64)
                && parts[0].bytes().all(|b| b.is_ascii_hexdigit())
                && found.insert(parts[1], parts[0]).is_none(),
            "unknown remote tag response"
        );
    }
    let actual = found
        .get(peeled_ref.as_str())
        .or_else(|| found.get(tag_ref.as_str()));
    ensure!(
        actual.copied() == Some(contract.commit.as_str()),
        "remote tag must identify the evaluated commit"
    );
    Ok(())
}

fn validate_release(release: &RemoteRelease, journal: &Journal) -> Result<()> {
    let contract = &journal.contract;
    // GitHub ignores target_commitish for an existing tag; it may remain a branch name.
    // Source identity is checked by preflight against the resolved remote tag, never that branch.
    ensure!(
        release.id > 0 && journal.release_id.is_none_or(|id| id == release.id),
        "remote release identity changed"
    );
    ensure!(
        release.tag_name == contract.tag
            && release.name == contract.title
            && digest(release.body.as_bytes()) == contract.notes_sha256
            && !release.prerelease
            && release.html_url
                == format!(
                    "https://github.com/{}/releases/tag/{}",
                    contract.repository, contract.tag
                ),
        "remote release differs (tag/title/notes/prerelease/URL); manual reconciliation required"
    );
    ensure!(
        !journal.complete || !release.draft,
        "completed release was changed back to a draft"
    );
    Ok(())
}

fn owned_starter(asset: &RemoteAsset, journal: &Journal) -> bool {
    asset.state == "starter"
        && asset.size == 0
        && journal.release_id.is_some()
        && match &journal.intent {
            Some(Intent::Upload { name }) => *name == asset.name,
            Some(Intent::DeleteStarter { name, id }) => *name == asset.name && *id == asset.id,
            _ => false,
        }
}

/// Inspect the entire inventory before any repair. Metadata digests are never byte evidence.
async fn verify_assets(
    c: &Context,
    input: &Inputs<'_>,
    journal: &Journal,
    assets: &[RemoteAsset],
    cancel: &watch::Receiver<bool>,
) -> Result<Option<(String, u64)>> {
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut starter = None;
    ensure!(
        journal.verified_assets.iter().all(|(name, id)| assets
            .iter()
            .any(|a| a.name == *name && a.id == *id && a.state == "uploaded")),
        "previously verified remote asset missing/replaced; preserved"
    );
    for asset in assets {
        ensure!(
            names.insert(&asset.name) && asset.id > 0 && ids.insert(asset.id),
            "duplicate remote asset identity"
        );
        let expected = journal
            .contract
            .assets
            .get(&asset.name)
            .context("unexpected remote asset; preserved")?;
        if owned_starter(asset, journal) {
            ensure!(
                starter.is_none(),
                "multiple starter assets; manual reconciliation required"
            );
            starter = Some((asset.name.clone(), asset.id));
        } else {
            ensure!(
                asset.state == "uploaded" && asset.size == expected.size,
                "conflicting remote asset state/size; preserved"
            );
        }
    }
    if assets.iter().any(|a| a.state == "uploaded") {
        let dir = tempfile::Builder::new()
            .prefix("release-download-")
            .tempdir_in(c.db.parent().unwrap())?;
        gh(
            single_root(c)?,
            &[
                "release",
                "download",
                input.tag,
                "--repo",
                input.repo,
                "--dir",
                dir.path().to_str().context("non-UTF8 download directory")?,
            ],
            cancel,
        )
        .await?;
        let mut downloaded = BTreeSet::new();
        for entry in std::fs::read_dir(dir.path())? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("non-UTF8 download filename"))?;
            ensure!(
                assets
                    .iter()
                    .any(|a| a.name == name && a.state == "uploaded"),
                "unexpected download; publication blocked"
            );
            let expected = &journal.contract.assets[&name];
            ensure!(
                entry.file_type()?.is_file() && entry.metadata()?.len() == expected.size,
                "download type/size mismatch: {name}"
            );
            // Binary data travels through files, never process stdout's UTF-8/redacted logs.
            ensure!(
                digest(&std::fs::read(entry.path())?) == expected.sha256,
                "download checksum mismatch: {name}"
            );
            downloaded.insert(name);
        }
        ensure!(
            assets
                .iter()
                .filter(|a| a.state == "uploaded")
                .all(|a| downloaded.contains(&a.name)),
            "missing asset download"
        );
    }
    Ok(starter)
}

pub(super) async fn publish(
    c: &Context,
    run: &Run,
    input: Inputs<'_>,
    cancel: &watch::Receiver<bool>,
) -> Result<Value> {
    let root = single_root(c)?;
    let key = format!("release-publication:{}", c.manifest.release_id);
    let contract = Contract::capture(c, run, &input)?;
    let saved = Store::open(&c.db, false)?.metadata(&key)?;
    let mut journal: Journal = match saved {
        Some(value) => {
            serde_json::from_value(value).context("invalid release publication journal")?
        }
        None => Journal {
            schema_version: 1,
            contract: contract.clone(),
            release_id: None,
            intent: None,
            complete: false,
            verified_assets: BTreeMap::new(),
        },
    };
    ensure!(
        journal.schema_version == 1 && journal.contract == contract,
        "publication contract changed; journal preserved; review original inputs"
    );
    preflight(c, &input, &contract, cancel).await?;
    persist(c, &key, &journal)?;

    // Freeze transport inputs outside source. Re-check the originals before each mutation.
    let staged = tempfile::Builder::new()
        .prefix("release-inputs-")
        .tempdir_in(c.db.parent().unwrap())?;
    let notes = staged.path().join("notes.md");
    std::fs::copy(input.notes, &notes)?;
    ensure!(
        digest(&std::fs::read(&notes)?) == contract.notes_sha256,
        "release notes changed while staging"
    );
    let staged_assets = staged.path().join("assets");
    std::fs::create_dir(&staged_assets)?;
    for (name, expected) in &contract.assets {
        let dest = staged_assets.join(name);
        std::fs::copy(input.assets.join(name), &dest)?;
        ensure!(
            digest(&std::fs::read(dest)?) == expected.sha256,
            "asset changed while staging"
        );
    }
    let mut attempted = BTreeSet::new();
    loop {
        preflight(c, &input, &contract, cancel).await?;
        let releases: Vec<Value> =
            api(root, &format!("repos/{}/releases", input.repo), cancel).await?;
        ensure!(
            releases.iter().all(|r| r["tag_name"].is_string()),
            "unknown release identity in collection"
        );
        let candidates = releases
            .iter()
            .filter(|r| r["tag_name"] == input.tag)
            .collect::<Vec<_>>();
        ensure!(candidates.len() <= 1, "ambiguous remote release");
        let Some(candidate) = candidates.first() else {
            ensure!(
                journal.release_id.is_none()
                    && matches!(journal.intent, None | Some(Intent::Create))
                    && !journal.complete,
                "previous release disappeared; manual reconciliation required"
            );
            ensure!(
                attempted.insert("create".into()),
                "creation not observed; retry explicitly"
            );
            journal.intent = Some(Intent::Create);
            persist(c, &key, &journal)?;
            gh(
                root,
                &[
                    "release",
                    "create",
                    input.tag,
                    "--repo",
                    input.repo,
                    "--draft",
                    "--verify-tag",
                    "--target",
                    &contract.commit,
                    "--title",
                    &contract.title,
                    "--notes-file",
                    notes.to_str().context("non-UTF8 notes path")?,
                ],
                cancel,
            )
            .await
            .context(
                "create outcome uncertain; intent retained; retry release publish explicitly",
            )?;
            continue;
        };
        // Unrelated releases may legitimately have null names/bodies. Only the selected
        // candidate must satisfy the full typed publication protocol.
        let release: RemoteRelease =
            serde_json::from_value((*candidate).clone()).context("invalid release state")?;
        validate_release(&release, &journal)?;
        if journal.release_id.is_none() {
            journal.release_id = Some(release.id);
            persist(c, &key, &journal)?;
        }
        let assets: Vec<RemoteAsset> = api(
            root,
            &format!("repos/{}/releases/{}/assets", input.repo, release.id),
            cancel,
        )
        .await?;
        let starter = verify_assets(c, &input, &journal, &assets, cancel).await?;
        let verified = assets
            .iter()
            .filter(|a| a.state == "uploaded")
            .map(|a| (a.name.clone(), a.id))
            .collect();
        if journal.verified_assets != verified {
            journal.verified_assets = verified;
            persist(c, &key, &journal)?;
        }
        let missing = contract
            .assets
            .keys()
            .filter(|name| name.as_str() != "SHA256SUMS")
            .chain(
                contract
                    .assets
                    .get_key_value("SHA256SUMS")
                    .map(|(name, _)| name),
            )
            .find(|name| !assets.iter().any(|a| a.name == **name));
        if !release.draft {
            ensure!(
                starter.is_none() && missing.is_none(),
                "published release inventory incomplete; preserved"
            );
            preflight(c, &input, &contract, cancel).await?;
            if !journal.complete {
                journal.complete = true;
                journal.intent = None;
                persist(c, &key, &journal)?;
                Store::open(&c.db, false)?.save(run, "published", &json!({"repository":input.repo,"tag":input.tag,"url":release.html_url,"expected_visibility":contract.visibility,"readback":"verified"}))?;
            }
            return Ok(
                json!({"schema_version":1,"published":true,"url":release.html_url,"readback":"verified"}),
            );
        }
        // Recheck local gates, tag and visibility immediately before every remote mutation.
        preflight(c, &input, &contract, cancel).await?;
        if let Some((name, id)) = starter {
            ensure!(
                attempted.insert(format!("delete:{id}")),
                "starter deletion not observed; retry explicitly"
            );
            journal.intent = Some(Intent::DeleteStarter { name, id });
            persist(c, &key, &journal)?;
            command(
                root,
                vec![
                    "gh".into(),
                    "api".into(),
                    "--hostname".into(),
                    "github.com".into(),
                    "--method".into(),
                    "DELETE".into(),
                    format!("repos/{}/releases/assets/{id}", input.repo),
                ],
                cancel,
            )
            .await
            .context("starter deletion outcome uncertain; intent retained; retry explicitly")?;
        } else if let Some(name) = missing {
            ensure!(
                attempted.insert(format!("upload:{name}")),
                "upload not observed; retry explicitly"
            );
            journal.intent = Some(Intent::Upload { name: name.clone() });
            persist(c, &key, &journal)?;
            let path = staged_assets.join(name);
            gh(
                root,
                &[
                    "release",
                    "upload",
                    input.tag,
                    "--repo",
                    input.repo,
                    path.to_str().context("non-UTF8 asset path")?,
                ],
                cancel,
            )
            .await
            .context(
                "upload outcome uncertain; intent retained; retry release publish explicitly",
            )?;
        } else {
            ensure!(
                attempted.insert("publish".into()),
                "publication not observed; retry explicitly"
            );
            journal.intent = Some(Intent::Publish);
            persist(c, &key, &journal)?;
            gh(
                root,
                &[
                    "release",
                    "edit",
                    input.tag,
                    "--repo",
                    input.repo,
                    "--draft=false",
                ],
                cancel,
            )
            .await
            .context(
                "publish outcome uncertain; intent retained; retry release publish explicitly",
            )?;
        }
    }
}
