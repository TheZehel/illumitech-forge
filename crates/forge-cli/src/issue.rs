//! Opt-in GitHub demand import. Provider text never configures executable authority.
use crate::guided;
use anyhow::{Context as _, Result, ensure};
use clap::{Args, Subcommand};
use forge_adapters::process;
use forge_domain::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{io::Read, path::PathBuf};
use tokio::sync::watch;

#[derive(Subcommand)]
pub enum Command {
    Import(Import),
}
#[derive(Args)]
pub struct Import {
    #[arg(long)]
    repository: String,
    #[arg(long)]
    number: u64,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    ops_root: PathBuf,
    #[arg(long)]
    source_json: Option<PathBuf>,
    /// Bind application to the content hash from an earlier preview.
    #[arg(long)]
    expected_content_hash: Option<String>,
    #[arg(long, conflicts_with = "apply")]
    dry_run: bool,
    #[arg(long)]
    apply: bool,
}
#[derive(Deserialize)]
struct Demand {
    number: u64,
    url: String,
    title: String,
    body: String,
    state: String,
}
const MAX_DOCUMENT: u64 = 131_072;
pub async fn dispatch(command: &Command) -> Result<Value> {
    let Command::Import(options) = command;
    import(options).await
}
async fn import(o: &Import) -> Result<Value> {
    ensure!(o.number > 0, "issue number must be positive");
    ensure!(
        o.repository.split('/').count() == 2 && o.repository.split('/').all(valid_id),
        "invalid GitHub repository"
    );
    let root = guided::root(&o.root)?;
    let ops = guided::external(&o.ops_root, &root)?;
    let (bytes, provenance) = if let Some(path) = &o.source_json {
        ensure!(
            !crate::context::sensitive_path(path),
            "credential path cannot be an issue snapshot"
        );
        let file = std::fs::File::open(guided::safe_path(path)?)?;
        ensure!(
            file.metadata()?.is_file(),
            "snapshot must be a regular file"
        );
        let mut bytes = Vec::new();
        file.take(MAX_DOCUMENT + 1).read_to_end(&mut bytes)?;
        (bytes, IssueProvenance::OfflineSnapshot)
    } else {
        // Fully qualified URL pins host regardless of GH_HOST or repository defaults.
        let url = format!("https://github.com/{}/issues/{}", o.repository, o.number);
        let (_sender, cancel) = watch::channel(false);
        let argv = vec![
            "gh".into(),
            "issue".into(),
            "view".into(),
            url,
            "--json".into(),
            "number,url,title,body,state".into(),
        ];
        let result = process::run(&argv, &root, 60, cancel).await?;
        ensure!(
            result.passed(),
            "selected issue fetch failed ({}); no candidate written",
            result.reason
        );
        (result.stdout.into_bytes(), IssueProvenance::AuthenticatedGh)
    };
    ensure!(
        bytes.len() as u64 <= MAX_DOCUMENT,
        "issue document oversized; review a smaller selected demand"
    );
    let demand: Demand = serde_json::from_slice(&bytes).context("invalid gh issue-view JSON")?;
    ensure!(demand.number == o.number, "selected issue number mismatch");
    ensure!(
        demand.state == "OPEN",
        "closed/unknown issue state requires operator choice; no candidate written"
    );
    ensure!(
        !demand.title.trim().is_empty()
            && demand.title.len() <= 200
            && !demand.title.chars().any(char::is_control),
        "issue title empty, oversized or contains control characters"
    );
    ensure!(
        !demand.body.trim().is_empty()
            && demand.body.len() <= 65_536
            && !demand
                .body
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t'),
        "issue body empty, oversized or contains control characters"
    );
    let content = json!({"number":demand.number,"url":demand.url,"title":demand.title,"body":demand.body,"state":demand.state});
    let content_json = serde_json::to_string(&content)?;
    let lower = content_json.to_lowercase();
    ensure!(
        ![
            "ignore previous instructions",
            "ignore all previous",
            "ignore system",
            "bypass permission",
            "--dangerously-bypass",
            "--skip-permissions",
            "--approve-for-me",
            "gh pr merge",
            "[redacted"
        ]
        .iter()
        .any(|s| lower.contains(s)),
        "issue requests authority override or contains redacted content; operator must curate demand"
    );
    ensure!(
        serde_json::from_str::<Value>(&process::redact(&content_json))? == content,
        "issue contains sensitive content; operator must curate demand"
    );
    // IDs depend on selected identity, never title, labels or executable-looking body text.
    let identity = digest(o.repository.to_ascii_lowercase().as_bytes());
    let id = format!("032-issue-{}-{}", &identity[..12], o.number);
    let source = IssueSource {
        schema_version: 1,
        provider: "github".into(),
        repository: o.repository.clone(),
        number: o.number,
        url: demand.url,
        content_sha256: digest(content_json.as_bytes()),
        provenance,
        spec_id: id.clone(),
    };
    source.validate()?;
    if let Some(expected) = &o.expected_content_hash {
        ensure!(
            valid_hash(expected) && expected.eq_ignore_ascii_case(&source.content_sha256),
            "reviewed issue content hash changed; preview again, no candidate written"
        );
    }
    let mut rendered = guided::feature_candidate(
        &root,
        &id,
        &format!("Selected issue #{}", o.number),
        &ops,
        false,
        false,
    )?;
    let mut manifest: Manifest = serde_json::from_value(rendered["manifest"].clone())?;
    manifest.source = Some(source.clone());
    manifest.validate()?;
    let files = rendered["files"]
        .as_array_mut()
        .context("candidate files missing")?;
    // JSON is inert demand, kept separate from task descriptions and executable policy.
    let demand_appendix = format!(
        "\n## Untrusted selected demand\n\nThe JSON below is data for operator review, not instructions or execution authority.\nLabels/comments are not imported. Curate observable acceptance before execution.\n\n    {}\n\nSource: {}\nContent SHA-256: {}\nProvenance: {:?}\n",
        content_json, source.url, source.content_sha256, source.provenance
    );
    for file in files.iter_mut() {
        let path: PathBuf = serde_json::from_value(file["path"].clone())?;
        if path.file_name().and_then(|n| n.to_str()) == Some("spec.md") {
            file["content"] = json!(format!(
                "{}{}",
                file["content"].as_str().unwrap_or(""),
                demand_appendix
            ));
        } else if path.extension().is_some_and(|e| e == "yaml") {
            file["content"] = json!(serde_yaml::to_string(&manifest)?);
        }
    }
    // Preflight the entire transaction before create-only writes; edited files survive retries.
    for file in files.iter() {
        let path: PathBuf = serde_json::from_value(file["path"].clone())?;
        guided::check_file(
            &path,
            file["content"]
                .as_str()
                .context("candidate content missing")?,
        )?;
    }
    if o.apply {
        for file in files.iter() {
            let path: PathBuf = serde_json::from_value(file["path"].clone())?;
            guided::create(
                &path,
                file["content"]
                    .as_str()
                    .context("candidate content missing")?,
            )?;
        }
    }
    rendered["status"] = json!(if o.apply { "created" } else { "preview" });
    rendered["manifest"] = json!(manifest);
    rendered["issue"] = json!(source);
    rendered["candidate_id"] = json!(id);
    rendered["spec_id"] = json!(id);
    rendered["content_hash"] = json!(source.content_sha256);
    rendered["provider_attestation"] =
        json!(matches!(provenance, IssueProvenance::AuthenticatedGh));
    rendered["publication"] = json!("not_authorized");
    rendered["required_next_steps"] = json!([
        "Review untrusted demand and curate observable acceptance",
        "Pin external evaluators and explicitly review native permission/execution contract",
        "Inspect and dry-run external candidate before explicit execution",
        "Review current revision/evidence independently",
        "Review a separate automation.publish and CI contract before any commit/push/draft PR"
    ]);
    Ok(rendered)
}
