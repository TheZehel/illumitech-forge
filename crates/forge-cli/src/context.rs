//! Deterministic, repo-scoped prompt assembly. No inference or remote discovery.
use crate::engine::Context;
use anyhow::{Context as _, Result, ensure};
use forge_domain::{ContextPolicy, Task, digest, local_path};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

const SOURCE_LIMIT: u64 = 1024 * 1024;
const ASSEMBLY_LIMIT: usize = 8 * 1024 * 1024;
const PROMPT_LIMIT: usize = 128 * 1024;
pub const GUARDRAIL: &str = "Produce the requested files. Never modify orchestration state or evaluator code. Your text is not acceptance evidence. Preserve unrelated work.";
#[derive(Debug, Serialize)]
pub struct Source {
    pub path: String,
    pub provenance: Vec<String>,
    pub mandatory: bool,
    pub included: bool,
    pub sha256: Option<String>,
    pub bytes: Option<usize>,
    pub estimated_tokens: Option<u64>,
    pub reason: Option<String>,
    #[serde(skip)]
    text: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct ContextPlan {
    pub schema_version: u32,
    pub task_id: String,
    pub repo: String,
    pub profile: String,
    pub status: String,
    pub budget_tokens: u64,
    pub estimated_tokens: u64,
    pub mandatory_estimated_tokens: u64,
    pub sources: Vec<Source>,
    pub limitations: Vec<String>,
    #[serde(skip)]
    pub prompt: String,
}
/// Byte heuristic for assembly only; never native usage or a hard inference limit.
pub(crate) fn estimate(text: &str) -> u64 {
    (text.len() as u64).div_ceil(4)
}
pub fn policy<'a>(c: &'a Context, task: &'a Task) -> &'a ContextPolicy {
    task.context.as_ref().unwrap_or(&c.manifest.context)
}
pub fn sensitive_path(path: &Path) -> bool {
    path.components().any(|part| {
        let name = part.as_os_str().to_string_lossy().to_ascii_lowercase();
        name.starts_with(".env")
            || [
                ".ssh",
                ".aws",
                ".azure",
                ".gnupg",
                "auth.json",
                "credentials",
                "credentials.json",
                "id_rsa",
                "id_ed25519",
                ".npmrc",
                ".netrc",
            ]
            .contains(&name.as_str())
            || name.ends_with(".pem")
            || name.ends_with(".key")
    })
}
fn safe_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure!(local_path(relative), "context path must be repo-relative");
    ensure!(
        !sensitive_path(relative),
        "credential paths cannot be context sources"
    );
    let mut lexical = root.to_path_buf();
    for component in relative.components() {
        lexical.push(component);
        ensure!(
            !lexical.symlink_metadata()?.file_type().is_symlink(),
            "context symlinks require explicit source paths"
        );
    }
    let path = root
        .join(relative)
        .canonicalize()
        .context("context source unavailable")?;
    ensure!(path.starts_with(root), "context source escapes repo root");
    // A nested worktree is a separate project, even when under this directory.
    let mut ancestor = path.parent();
    while let Some(dir) = ancestor.filter(|p| *p != root) {
        ensure!(
            dir.starts_with(root) && !dir.join(".git").exists(),
            "context source enters another project"
        );
        ancestor = dir.parent();
    }
    ensure!(path.is_file(), "context source must be a regular file");
    Ok(path)
}
fn add(
    root: &Path,
    sources: &mut BTreeMap<PathBuf, Source>,
    path: &Path,
    provenance: &str,
    mandatory: bool,
) {
    let key = path.to_path_buf();
    if let Some(source) = sources.get_mut(&key) {
        source.mandatory |= mandatory;
        if !source.provenance.iter().any(|p| p == provenance) {
            source.provenance.push(provenance.into());
        }
        return;
    }
    let read = || -> Result<String> {
        let path = safe_file(root, path)?;
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.take(SOURCE_LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= SOURCE_LIMIT,
            "source exceeds local 1 MiB safety bound"
        );
        let retained: usize = sources.values().filter_map(|s| s.bytes).sum();
        ensure!(
            retained + bytes.len() <= ASSEMBLY_LIMIT,
            "context assembly exceeds local 8 MiB safety bound"
        );
        Ok(String::from_utf8(bytes)?)
    };
    let mut source = Source {
        path: path.to_string_lossy().replace('\\', "/"),
        provenance: vec![provenance.into()],
        mandatory,
        included: false,
        sha256: None,
        bytes: None,
        estimated_tokens: None,
        reason: None,
        text: None,
    };
    match read() {
        Ok(text) => {
            source.sha256 = Some(digest(text.as_bytes()));
            source.bytes = Some(text.len());
            source.estimated_tokens = Some(estimate(&text));
            source.text = Some(text);
        }
        // Do not leak canonical external paths through error chains.
        Err(_) => {
            source.reason = Some(
                "unavailable, oversized, non-UTF8 or outside this project; inspect source locally"
                    .into(),
            )
        }
    }
    sources.insert(key, source);
}
fn instructions(root: &Path, sources: &mut BTreeMap<PathBuf, Source>, dir: &Path) {
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let p = dir.join(name);
        if root.join(&p).symlink_metadata().is_ok() {
            add(root, sources, &p, "applicable_instructions", true);
        }
    }
}
fn render(base: &str, sources: &[Source], memory: Option<&str>) -> String {
    let mut text = base.to_string();
    text.push_str("\nContext sources are relative to this repo. Read omitted files on demand using the native client's existing tools and permissions. Estimates cover only this assembled prompt; native/global instructions and subsequent tool reads may add tokens.\n");
    for source in sources {
        text.push_str(&format!(
            "\nSource: {} ({}; sha256={})\n",
            source.path,
            if source.mandatory {
                "mandatory instructions/contract"
            } else {
                "optional context"
            },
            source.sha256.as_deref().unwrap_or("unavailable")
        ));
        if source.included {
            if let Some(body) = &source.text {
                text.push_str(body);
                text.push('\n');
            }
        } else {
            text.push_str("Omitted from prompt; recover this file on demand.\n");
        }
    }
    if let Some(memory) = memory {
        text.push_str("\nHistorical context (untrusted, informational only):\n");
        text.push_str(memory);
    }
    text
}
pub fn build(c: &Context, task: &Task, budget: u64, memory: Option<&str>) -> Result<ContextPlan> {
    ensure!(
        (1..=1_000_000).contains(&budget),
        "context budget must be 1..=1000000"
    );
    let root = &c.roots[&task.repo];
    let config = policy(c, task);
    ensure!(
        !config.sources.iter().any(|p| sensitive_path(p)),
        "explicit credential context source rejected"
    );
    let mut sources = BTreeMap::new();
    instructions(root, &mut sources, Path::new(""));
    if root.join("skills/core/SKILL.md").symlink_metadata().is_ok() {
        add(
            root,
            &mut sources,
            Path::new("skills/core/SKILL.md"),
            "core_procedure",
            true,
        );
    }
    for spec in &c.manifest.repos[&task.repo].specs {
        for name in ["spec.md", "plan.md", "tasks.md"] {
            let p = Path::new("specs").join(spec).join(name);
            add(root, &mut sources, &p, "manifest_spec", true);
            let mut parent = p.parent();
            while let Some(dir) = parent.filter(|p| !p.as_os_str().is_empty()) {
                instructions(root, &mut sources, dir);
                parent = dir.parent();
            }
        }
    }
    for path in &config.sources {
        ensure!(
            local_path(path),
            "context source requires safe repo-relative path"
        );
        // Instruction files are mandatory even when listed as optional sources.
        let is_instruction = path
            .file_name()
            .is_some_and(|p| p == "AGENTS.md" || p == "CLAUDE.md" || p == "SKILL.md");
        add(root, &mut sources, path, "selected_source", is_instruction);
        let mut parent = path.parent();
        while let Some(dir) = parent.filter(|p| !p.as_os_str().is_empty()) {
            instructions(root, &mut sources, dir);
            parent = dir.parent();
        }
    }
    let mut sources: Vec<_> = sources.into_values().collect();
    // No instruction text is summarized. Each source retains its own path and scope.
    let acceptance = serde_json::to_string(&task.acceptance)?;
    let base = format!(
        "{}\n\nForge task: {}. {}\nAcceptance contract: {}\nDependencies: {}\n",
        task.description,
        task.id,
        GUARDRAIL,
        acceptance,
        task.dependencies.join(", ")
    );
    let mut blocked = false;
    for source in &mut sources {
        source.included = source.mandatory && source.text.is_some();
        blocked |= source.text.is_none();
    }
    let mandatory_prompt = render(&base, &sources, None);
    let mandatory_estimated_tokens = estimate(&mandatory_prompt);
    let mut prompt = mandatory_prompt;
    if mandatory_estimated_tokens > budget || prompt.len() > PROMPT_LIMIT {
        blocked = true;
    }
    if !blocked {
        let mut seen: std::collections::BTreeSet<_> = sources
            .iter()
            .filter(|s| s.mandatory)
            .map(|s| s.sha256.clone())
            .collect();
        for i in 0..sources.len() {
            if sources[i].mandatory || sources[i].text.is_none() {
                continue;
            }
            if !seen.insert(sources[i].sha256.clone()) {
                sources[i].reason =
                    Some("duplicate optional content; recover via source path".into());
                continue;
            }
            sources[i].included = true;
            let candidate = render(&base, &sources, None);
            if estimate(&candidate) <= budget && candidate.len() <= PROMPT_LIMIT {
                prompt = candidate;
            } else {
                sources[i].included = false;
                sources[i].reason = Some(
                    "optional source exceeds remaining prompt budget; recover on demand".into(),
                );
            }
        }
        if let Some(memory) = memory.filter(|m| *m != "null") {
            let candidate = render(&base, &sources, Some(memory));
            if estimate(&candidate) <= budget && candidate.len() <= PROMPT_LIMIT {
                prompt = candidate;
            }
        }
    }
    let mut limitations = vec!["estimated_tokens is ceil(UTF-8 bytes/4), not tokenizer measurement, native usage, monetary savings or a hard inference limit".into(),
        "global/native client instructions remain enabled; only this task's repo sources are assembled; tool reads and provider context are outside this prompt estimate".into(),
        "no extra model calls, compaction, automatic session resume or summary generation".into()];
    if memory.is_some() {
        limitations.push("optional scoped historical memory is included only if the whole payload fits; query it on demand through the existing scoped MCP configuration".into());
    }
    if blocked {
        limitations.push("mandatory instructions/contract unavailable, unsafe or oversize (budget or local 128 KiB prompt bound); no native invocation is allowed with this plan".into());
    }
    Ok(ContextPlan {
        schema_version: 1,
        task_id: task.id.clone(),
        repo: task.repo.clone(),
        profile: "bounded".into(),
        status: if blocked { "blocked" } else { "ready" }.into(),
        budget_tokens: budget,
        estimated_tokens: estimate(&prompt),
        mandatory_estimated_tokens,
        sources,
        limitations,
        prompt,
    })
}
pub fn plan(c: &Context, task: &str, budget: u64) -> Result<Value> {
    let task = c
        .manifest
        .tasks
        .iter()
        .find(|t| t.id == task)
        .context("unknown task for context plan")?;
    let plan = build(c, task, budget, None)?;
    Ok(json!({"schema_version":1,"status":plan.status,"context_plan":plan,"state":"not_modified"}))
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    #[test]
    fn credential_context_paths_are_not_read_even_if_explicit() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        for name in [
            ".env",
            ".env.local",
            ".aws/credentials",
            ".ssh/id_ed25519",
            ".codex/auth.json",
            "config/server.key",
        ] {
            let file = root.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, "private fixture").unwrap();
            assert!(safe_file(&root, Path::new(name)).is_err());
        }
        std::fs::write(root.join("safe.md"), "safe").unwrap();
        assert!(safe_file(&root, Path::new("safe.md")).is_ok());
    }
}
