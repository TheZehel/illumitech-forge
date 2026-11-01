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
pub fn init(root: &Path, memory_workspace: Option<&str>) -> Result<Value> {
    ensure!(
        root.join(".git").exists(),
        "initialize a Git repository first"
    );
    let root = root.canonicalize()?;
    let readme = "# Forge\n\nKeep manifests and evaluator code outside agent-writable repositories.\nUse `release inspect` and `release run --dry-run` before execution.\n";
    let readme_path = root.join(".forge/README.md");
    // Preflight every destination before creating optional routing or scaffolding.
    if let Ok(meta) = root.join(".forge").symlink_metadata() {
        ensure!(
            meta.is_dir() && !meta.file_type().is_symlink(),
            "existing .forge is not a regular directory; preserving it"
        );
    }
    check_existing(&readme_path, readme)?;
    let memory_path = root.join(".ai-memory.toml");
    let mut selected = None;
    let memory_status = if let Some(workspace) = memory_workspace {
        ensure!(
            valid_id(workspace),
            "memory workspace must use letters, digits, hyphens or underscores"
        );
        let content = format!("workspace = \"{workspace}\"\nproject_strategy = \"repo-root\"\n");
        // Explicit selection never changes an existing project's routing.
        check_existing(&memory_path, &content)?;
        if memory_path.symlink_metadata().is_ok() {
            "preserved"
        } else {
            selected = Some(content);
            "created"
        }
    } else if memory_path.symlink_metadata().is_ok() {
        "preserved"
    } else {
        "not_requested"
    };
    if let Some(content) = selected {
        create(&memory_path, &content)?;
    }
    create(&readme_path, readme)?;
    Ok(json!({"schema_version":1,"initialized":true,"root":root,"memory_config":memory_status}))
}
fn check_existing(path: &Path, expected: &str) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            ensure!(
                meta.is_file()
                    && !meta.file_type().is_symlink()
                    && meta.len() == expected.len() as u64,
                "existing file differs or is not regular; preserving {}",
                path.display()
            );
            ensure!(
                std::fs::read(path)? == expected.as_bytes(),
                "existing file differs; preserving {}",
                path.display()
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.into()),
    }
    Ok(())
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
