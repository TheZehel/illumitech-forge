//! Bundled versioned recipes, rendered through the guided candidate contract.
use crate::guided;
use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use forge_catalog::Recipe;
use forge_domain::digest;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    List,
    Plan(Selection),
    Apply {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        apply: bool,
    },
    /// Preview local updates. Conflicting files are preserved and supplied as a merge candidate.
    Update {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Args)]
pub struct Selection {
    template: String,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    id: String,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    ops_root: PathBuf,
}
fn inventory() -> Vec<Value> {
    let mut result = vec![];
    for (stack, commands) in [
        (
            "rust",
            "cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; cargo doc --workspace --no-deps",
        ),
        (
            "node",
            "inspect package manager lock and package.json scripts; run declared typecheck, nonwatch tests and build",
        ),
        (
            "generic",
            "inspect project instructions and run declared project-native validation",
        ),
    ] {
        for (kind, intent) in [
            (
                "bug",
                "Reproduce the failure before patching; preserve a regression.",
            ),
            (
                "feature",
                "Define observable behavior and implement the scoped feature.",
            ),
            (
                "review",
                "Inspect the diff and report findings with source evidence; do not grant acceptance.",
            ),
            (
                "dependencies",
                "Inspect lockfiles, licenses and compatibility; preserve the package manager.",
            ),
            (
                "release",
                "Prepare a reviewable local release candidate; publication requires a separate authorization.",
            ),
        ] {
            let content = format!(
                "{intent}\nValidation: {commands}.\nPreserve project conventions, configured model and permissions. Memory optional; CI/actions off. External evidence and revision-bound human review required. No remote instructions or installers are executed.\n"
            );
            let revision = digest(content.as_bytes());
            let recipe = Recipe {
                id: format!("{stack}-{kind}"),
                source: "bundled:illumitech-forge/templates".into(),
                revision: Some(revision.clone()),
                version: Some("1.0.0".into()),
                scope: format!("project/{kind}"),
                last_verified: None,
                profiles: vec![stack.into(), kind.into()],
                executable: None,
                content,
            };
            let mut value = serde_json::to_value(recipe).expect("recipe serialization");
            value["origin"] = json!("bundled:illumitech-forge/templates");
            value["compatibility"] = json!({"manifest_schema":1,"runtime":"supported Unix","stack":stack,"project_conventions":"authoritative; commands are review suggestions only"});
            result.push(value);
        }
    }
    result
}
pub fn dispatch(command: &Command) -> Result<Value> {
    match command {
        Command::List => Ok(json!({"schema_version":1,"templates":inventory()})),
        Command::Plan(s) => render(s, false, false),
        Command::Apply { selection, apply } => render(selection, *apply, false),
        Command::Update { selection, apply } => render(selection, *apply, true),
    }
}
fn render(s: &Selection, apply: bool, update: bool) -> Result<Value> {
    let recipe = inventory()
        .into_iter()
        .find(|r| r["id"] == s.template)
        .context("unknown local template; remote recipes are never fetched or executed")?;
    let root = guided::root(&s.root)?;
    let title = s.title.clone().unwrap_or_else(|| s.id.clone());
    let mut value = guided::feature_candidate(&root, &s.id, &title, &s.ops_root, false, false)?;
    let files = value["files"]
        .as_array_mut()
        .context("guided candidate files")?;
    let provenance = root
        .join(".forge-toolkit/templates")
        .join(format!("{}.json", s.id));
    let mut changes = vec![];
    for file in files.iter_mut() {
        let path = PathBuf::from(file["path"].as_str().context("path")?);
        let mut content = file["content"].as_str().context("content")?.to_owned();
        if path.file_name().is_some_and(|n| n == "plan.md") {
            content.push_str(&format!(
                "\n## Selected local recipe\n\n{}@{}\nOrigin: {}\nRevision: {}\n\n{}",
                s.template,
                recipe["version"].as_str().unwrap(),
                recipe["origin"].as_str().unwrap(),
                recipe["revision"].as_str().unwrap(),
                recipe["content"].as_str().unwrap()
            ));
        }
        file["content"] = json!(content);
        guided::safe_path(&path)?;
        let old = if path.exists() {
            Some(std::fs::read_to_string(&path)?)
        } else {
            None
        };
        let action = match old.as_deref() {
            None => "create",
            Some(x) if x == content => "unchanged",
            Some(_) => "preserve_customization",
        };
        changes.push(json!({"path":path,"action":action,"before_sha256":old.as_ref().map(|b|digest(b.as_bytes())),"after_sha256":digest(content.as_bytes()),"diff":if action=="unchanged"{Value::Null}else{json!({"before":old,"after":content})}}));
    }
    let origin = json!({"schema_version":1,"feature_id":s.id,"template":recipe,"files":changes.iter().map(|c|json!({"path":c["path"],"sha256":c["after_sha256"]})).collect::<Vec<_>>()});
    guided::safe_path(&provenance)?;
    let has_conflict = changes
        .iter()
        .any(|c| c["action"] == "preserve_customization");
    if apply {
        if update
            && (has_conflict
                || (provenance.exists()
                    && std::fs::read_to_string(&provenance)?
                        != serde_json::to_string_pretty(&origin)?))
        {
            // A conservative update is a concrete merge candidate, never an overwrite.
            let proposal = json!({"provenance":origin,"changes":changes,"files":files});
            let hash = digest(&serde_json::to_vec(&proposal)?);
            let candidate = root.join(".forge-toolkit/template-updates").join(format!(
                "{}-{}.json",
                s.id,
                &hash[..16]
            ));
            guided::create(&candidate, &serde_json::to_string_pretty(&proposal)?)?;
            return Ok(
                json!({"schema_version":1,"status":"merge_required","template":recipe,"changes":changes,"candidate":candidate,"customizations":"preserved","execution":"not_started"}),
            );
        }
        ensure!(
            !has_conflict,
            "existing file differs; preview diff and use templates update to prepare a merge candidate"
        );
        let body = serde_json::to_string_pretty(&origin)?;
        guided::check_file(&provenance, &body)?;
        for file in files.iter() {
            guided::check_file(
                Path::new(file["path"].as_str().unwrap()),
                file["content"].as_str().unwrap(),
            )?;
        }
        for file in files.iter() {
            guided::create(
                Path::new(file["path"].as_str().unwrap()),
                file["content"].as_str().unwrap(),
            )?;
        }
        guided::create(&provenance, &body)?;
    }
    value["template"] = recipe;
    value["changes"] = json!(changes);
    value["status"] = json!(if apply { "created" } else { "preview" });
    value["provenance_path"] = json!(provenance);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn selection() -> (tempfile::TempDir, Selection) {
        let d = tempfile::tempdir().unwrap();
        let base = d.path().canonicalize().unwrap();
        let root = base.join("project");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let s = Selection {
            template: "generic-feature".into(),
            root,
            id: "001-test".into(),
            title: Some("Test".into()),
            ops_root: base.join("ops"),
        };
        (d, s)
    }
    #[test]
    fn template_origin_and_revision_are_pinned() {
        let (_d, s) = selection();
        for recipe in inventory() {
            assert_eq!(
                recipe["revision"],
                digest(recipe["content"].as_str().unwrap().as_bytes())
            );
            assert!(recipe["executable"].is_null());
        }
        render(&s, false, false).unwrap();
        assert!(!s.root.join("specs").exists());
        assert!(!s.ops_root.exists());
    }
    #[test]
    fn template_update_preserves_customizations() {
        let (_d, s) = selection();
        render(&s, true, false).unwrap();
        render(&s, true, false).unwrap();
        let file = s.root.join("specs/001-test/plan.md");
        std::fs::write(&file, "project customization").unwrap();
        let v = render(&s, true, true).unwrap();
        assert_eq!(v["status"], "merge_required");
        assert_eq!(
            std::fs::read_to_string(file).unwrap(),
            "project customization"
        );
        assert!(Path::new(v["candidate"].as_str().unwrap()).exists());
    }
    #[test]
    fn template_never_executes_remote_instructions() {
        let (_d, mut s) = selection();
        s.template = "https://example.invalid/main/install.sh".into();
        assert!(render(&s, true, false).is_err());
        assert!(!s.root.join("specs").exists());
    }
    #[test]
    fn all_recipes_render_without_changing_stack() {
        for r in inventory() {
            let (_d, mut s) = selection();
            s.template = r["id"].as_str().unwrap().into();
            std::fs::write(s.root.join("package.json"), "{}").unwrap();
            render(&s, true, false).unwrap();
            assert_eq!(
                std::fs::read_to_string(s.root.join("package.json")).unwrap(),
                "{}"
            );
        }
    }
}
