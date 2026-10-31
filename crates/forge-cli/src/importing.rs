use anyhow::{Context, Result, ensure};
use forge_domain::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn candidate(
    id: String,
    repo: &str,
    description: String,
    priority: i32,
    dependencies: Vec<String>,
) -> Task {
    Task {
        acceptance: vec![Acceptance::Human {
            id: format!("review-{id}"),
        }],
        id,
        repo: repo.into(),
        description,
        priority,
        dependencies,
        executor: Executor::Codex {
            permission: Permission::ReadOnly,
            model: None,
        },
        timeout_seconds: 1800,
        max_attempts: 3,
        context: None,
        budget: Budget::default(),
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SpecMetadata {
    dependencies: Vec<String>,
    priority: i32,
}
impl Default for SpecMetadata {
    fn default() -> Self {
        Self {
            dependencies: vec![],
            priority: 100,
        }
    }
}

/// Stable document IDs survive moved lines. Content digests are a disclosed legacy fallback.
pub fn spec_tasks(root: &Path, repo: &str, slug: &str) -> Result<Vec<Task>> {
    ensure!(
        valid_id(repo) && valid_id(slug),
        "invalid repo/spec identifier"
    );
    let path = root.join("specs").join(slug).join("tasks.md");
    let text = std::fs::read_to_string(path)?;
    let mut tasks = Vec::new();
    let mut ids = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(body) = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("* [ ] "))
        else {
            continue;
        };
        let (body, metadata) = if let Some((text, annotation)) = body.split_once("<!-- forge:") {
            let json = annotation
                .strip_suffix("-->")
                .context("unterminated Forge task metadata")?;
            (
                text.trim(),
                serde_json::from_str::<SpecMetadata>(json.trim())
                    .context("invalid Forge task metadata")?,
            )
        } else {
            (body, SpecMetadata::default())
        };
        let first = body
            .split_whitespace()
            .next()
            .context("empty Spec Kit task")?
            .trim_end_matches(':');
        let explicit = valid_id(first) && first.bytes().any(|b| b.is_ascii_digit());
        let local = if explicit {
            first.to_string()
        } else {
            format!(
                "legacy-{}",
                &digest(
                    body.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .as_bytes()
                )[..12]
            )
        };
        let id = format!("{repo}-{slug}-{local}");
        ensure!(
            valid_id(&id) && id.len() <= 93 && ids.insert(id.clone()),
            "duplicate/oversized Spec Kit ID; assign distinct permanent IDs"
        );
        tasks.push(candidate(id, repo, format!("{body}\nSource: specs/{slug}/tasks.md ({local}). Imported candidate: review permissions and attach executable acceptance. Historical checkboxes are not evidence."), metadata.priority, metadata.dependencies));
    }
    // Unqualified dependencies refer to permanent IDs in this spec. Fully qualified IDs
    // may refer to another imported spec or an explicit Loop task; Manifest validates them.
    let prefix = format!("{repo}-{slug}-");
    for task in &mut tasks {
        for dependency in &mut task.dependencies {
            let qualified = format!("{prefix}{dependency}");
            if ids.contains(&qualified) {
                *dependency = qualified;
            }
        }
    }
    Ok(tasks)
}

fn finish(
    m: Manifest,
    source_hash: String,
    warnings: Vec<String>,
    output: Option<&Path>,
) -> Result<Value> {
    m.validate()?;
    if let Some(path) = output {
        let content = serde_yaml::to_string(&m)?;
        if path.exists() {
            ensure!(
                std::fs::read_to_string(path)? == content,
                "existing candidate differs; choose another output"
            );
        } else {
            use std::io::Write;
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?
                .write_all(content.as_bytes())?;
        }
    }
    Ok(
        json!({"schema_version":1,"candidate":m,"source_sha256":source_hash,"legacy_state_imported":false,"warnings":warnings,"written":output}),
    )
}

pub fn import_loop(input: &Path, output: Option<&Path>) -> Result<Value> {
    let bytes = std::fs::read(input)?;
    let legacy: Value = serde_yaml::from_slice(&bytes)?;
    let base = input
        .canonicalize()?
        .parent()
        .context("manifest parent")?
        .to_path_buf();
    let mut repos = BTreeMap::new();
    for (id, r) in legacy["repos"].as_object().context("Loop repos missing")? {
        let path = base
            .join(r["path"].as_str().context("Loop repo path missing")?)
            .canonicalize()?;
        let specs: Vec<String> = serde_json::from_value(
            r.get("speckit")
                .and_then(|s| s.get("specs"))
                .cloned()
                .unwrap_or(json!([])),
        )?;
        repos.insert(id.clone(), Repo { path, specs });
    }
    let mut tasks = Vec::new();
    for t in legacy["tasks"].as_array().unwrap_or(&vec![]) {
        let id = t["id"]
            .as_str()
            .context("Loop task ID missing")?
            .to_string();
        ensure!(id.len() <= 93, "task ID too long for review gate");
        tasks.push(candidate(id, t["repo"].as_str().context("Loop repo missing")?,
            format!("{}\nImported candidate: review legacy acceptance and attach current executable evidence.",t["description"].as_str().unwrap_or("Review legacy task")),
            t["priority"].as_i64().map(i32::try_from).transpose()?.unwrap_or(100),
            serde_json::from_value(t.get("dependencies").cloned().unwrap_or(json!([])))?));
    }
    for (id, repo) in &repos {
        for slug in &repo.specs {
            tasks.extend(spec_tasks(&repo.path, id, slug)?);
        }
    }
    finish(Manifest {schema_version:1,release_id:legacy["release_id"].as_str().context("Loop release_id missing")?.into(),repos,tasks,gates:vec![],memory:Memory::default(),automation:Automation::default(),context:ContextPolicy::default(),budget:Budget::default(),execution:Default::default(),review:Default::default(),source:None},digest(&bytes),
        vec!["Legacy gates require manual conversion into pinned external evaluators. Automation is disabled. Pending Spec Kit tasks were expanded; old states and checked tasks were not imported. Content-derived legacy IDs change when their text changes; assign permanent IDs before editing.".into()],output)
}

pub fn import_spec(
    root: &Path,
    specs: &[String],
    release: &str,
    output: Option<&Path>,
) -> Result<Value> {
    let root = root.canonicalize()?;
    let mut tasks = vec![];
    let mut source = Vec::new();
    for slug in specs {
        tasks.extend(spec_tasks(&root, "app", slug)?);
        source.extend(std::fs::read(
            root.join("specs").join(slug).join("tasks.md"),
        )?);
    }
    finish(Manifest {schema_version:1,release_id:release.into(),repos:BTreeMap::from([("app".into(),Repo{path:root,specs:specs.to_vec()})]),tasks,gates:vec![],memory:Memory::default(),automation:Automation::default(),context:ContextPolicy::default(),budget:Budget::default(),execution:Default::default(),review:Default::default(),source:None},digest(&source),vec!["Review candidate permissions and replace/add executable gates before running. Checked tasks are not execution evidence.".into()],output)
}
