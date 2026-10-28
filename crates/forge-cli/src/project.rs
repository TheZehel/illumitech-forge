use anyhow::{Context, Result, ensure};
use forge_domain::*;
use serde_json::{Value, json};
use std::{io::Write, path::Path};
fn create(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        ensure!(
            std::fs::read_to_string(path)? == content,
            "existing file differs; preserving {}",
            path.display()
        );
        return Ok(());
    }
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    f.write_all(content.as_bytes())?;
    Ok(())
}
pub fn init(root: &Path) -> Result<Value> {
    ensure!(
        root.join(".git").exists(),
        "initialize a Git repository first"
    );
    create(
        &root.join(".ai-memory.toml"),
        "workspace = \"illumitech\"\nproject_strategy = \"repo-root\"\n",
    )?;
    create(
        &root.join(".forge/README.md"),
        "# Forge\n\nKeep manifests and evaluator code outside agent-writable repositories.\nUse `release inspect` and `release run --dry-run` before execution.\n",
    )?;
    Ok(json!({"schema_version":1,"initialized":true,"root":root}))
}
pub fn spec_init(root: &Path, id: &str, title: &str) -> Result<Value> {
    ensure!(
        valid_id(id) && id.len() >= 4 && id.as_bytes()[..3].iter().all(u8::is_ascii_digit),
        "spec ID must start with three digits"
    );
    let base = root.join("specs").join(id);
    for (name, body) in [
        (
            "spec.md",
            format!(
                "# {id} — {title}\n\n## Acceptance\n\n- {id}-AC1: Define observable outcome and evidence.\n\n## Dependencies\n\nNone.\n"
            ),
        ),
        (
            "plan.md",
            format!(
                "# Plan {id}\n\nDefine contracts, failure handling, migration and verification before execution.\n"
            ),
        ),
        (
            "tasks.md",
            format!(
                "# Tasks {id}\n\n- [ ] {id}-T01: Implement {title}. Evidence: external gate.\n"
            ),
        ),
    ] {
        create(&base.join(name), &body)?;
    }
    Ok(json!({"schema_version":1,"spec":id,"path":base}))
}
pub fn spec_check(root: &Path) -> Result<Value> {
    let mut ids = std::collections::BTreeSet::new();
    let mut count = 0;
    for entry in std::fs::read_dir(root.join("specs"))? {
        let p = entry?.path();
        if !p.is_dir() {
            continue;
        }
        let slug = p.file_name().unwrap().to_str().context("non-UTF8 spec")?;
        ensure!(valid_id(slug), "invalid spec slug");
        for name in ["spec.md", "plan.md", "tasks.md"] {
            ensure!(
                !std::fs::read_to_string(p.join(name))?.trim().is_empty(),
                "empty spec document"
            );
        }
        let tasks = std::fs::read_to_string(p.join("tasks.md"))?;
        for line in tasks.lines().filter(|l| l.starts_with("- [")) {
            let id = line
                .split_once("] ")
                .and_then(|(_, rest)| rest.split_whitespace().next())
                .context("task needs permanent ID")?
                .trim_end_matches(':');
            ensure!(
                valid_id(id) && ids.insert(id.to_string()),
                "duplicate or invalid spec task ID"
            );
        }
        count += 1;
    }
    ensure!(count > 0, "no specs");
    Ok(json!({"schema_version":1,"status":"passed","specs":count,"task_ids":ids.len()}))
}
pub fn import_loop(input: &Path, output: Option<&Path>) -> Result<Value> {
    let bytes = std::fs::read(input)?;
    let legacy: Value = serde_yaml::from_slice(&bytes)?;
    let mut repos = std::collections::BTreeMap::new();
    for (id, r) in legacy["repos"].as_object().context("Loop repos missing")? {
        let p = r["path"].as_str().context("Loop repo path missing")?;
        repos.insert(
            id.clone(),
            Repo {
                path: input
                    .canonicalize()?
                    .parent()
                    .unwrap()
                    .join(p)
                    .canonicalize()?,
                specs: vec![],
            },
        );
    }
    let mut tasks = Vec::new();
    for t in legacy["tasks"].as_array().context("Loop tasks missing")? {
        let id = t["id"]
            .as_str()
            .context("Loop task ID missing")?
            .to_string();
        tasks.push(Task{id:id.clone(),repo:t["repo"].as_str().context("Loop repo missing")?.into(),description:format!("{}\nImported candidate: review original acceptance and attach current executable evidence.",t["description"].as_str().unwrap_or("Review legacy task")),dependencies:serde_json::from_value(t.get("dependencies").cloned().unwrap_or(json!([])))?,executor:Executor::Codex{permission:Permission::ReadOnly,model:None},acceptance:vec![Acceptance::Human{id:format!("review-{id}")}],timeout_seconds:1800,max_attempts:3});
    }
    let m = Manifest {
        schema_version: 1,
        release_id: legacy["release_id"]
            .as_str()
            .context("Loop release_id missing")?
            .into(),
        repos,
        tasks,
        gates: vec![],
        memory: Memory::default(),
    };
    m.validate()?;
    if let Some(p) = output {
        create(p, &serde_yaml::to_string(&m)?)?;
    }
    Ok(
        json!({"schema_version":1,"candidate":m,"source_sha256":digest(&bytes),"legacy_state_imported":false,"warning":"Review candidate; legacy gates and Spec Kit implicit tasks require manual mapping","written":output}),
    )
}
