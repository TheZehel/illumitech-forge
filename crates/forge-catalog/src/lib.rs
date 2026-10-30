//! Selective, local-only recipes. Remote instructions are never executed.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub source: String,
    pub revision: Option<String>,
    pub version: Option<String>,
    pub scope: String,
    pub last_verified: Option<String>,
    pub profiles: Vec<String>,
    pub executable: Option<String>,
    pub content: String,
}
pub fn recipes() -> Result<Vec<Recipe>> {
    Ok(serde_json::from_str(include_str!("recipes.json"))?)
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Debug, Serialize)]
pub struct Change {
    pub recipe: String,
    pub path: PathBuf,
    pub sha256: String,
    pub action: String,
}
pub fn plan(root: &Path, ids: &[String]) -> Result<Vec<Change>> {
    ensure!(!ids.is_empty(), "select at least one recipe");
    let catalog = recipes()?;
    let mut changes = Vec::new();
    for id in ids {
        let r = catalog
            .iter()
            .find(|r| &r.id == id)
            .ok_or_else(|| anyhow::anyhow!("unknown recipe {id}"))?;
        let path = root.join(".forge-toolkit").join(format!("{id}.md"));
        if root.join(".forge-toolkit").exists() {
            ensure!(
                !std::fs::symlink_metadata(root.join(".forge-toolkit"))?
                    .file_type()
                    .is_symlink(),
                "symlink toolkit directory refused"
            );
        }
        let action = if path.exists() {
            ensure!(
                !std::fs::symlink_metadata(&path)?.file_type().is_symlink(),
                "symlink recipe refused"
            );
            if std::fs::read(&path)? == r.content.as_bytes() {
                "unchanged"
            } else {
                "conflict"
            }
        } else {
            "create"
        };
        changes.push(Change {
            recipe: id.clone(),
            path,
            sha256: hash(r.content.as_bytes()),
            action: action.into(),
        });
    }
    Ok(changes)
}
pub fn apply(root: &Path, ids: &[String]) -> Result<Vec<Change>> {
    let changes = plan(root, ids)?;
    ensure!(
        !changes.iter().any(|c| c.action == "conflict"),
        "existing recipe differs; preserve it and review manually"
    );
    let catalog = recipes()?;
    for c in &changes {
        if c.action == "create" {
            std::fs::create_dir_all(c.path.parent().unwrap())?;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&c.path)?;
            f.write_all(
                catalog
                    .iter()
                    .find(|r| r.id == c.recipe)
                    .unwrap()
                    .content
                    .as_bytes(),
            )?;
            f.sync_all()?;
        }
    }
    Ok(changes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idempotent_preserves_custom_files() {
        let d = tempfile::tempdir().unwrap();
        let ids = vec!["rust".into()];
        assert_eq!(apply(d.path(), &ids).unwrap()[0].action, "create");
        assert_eq!(apply(d.path(), &ids).unwrap()[0].action, "unchanged");
        std::fs::write(d.path().join(".forge-toolkit/rust.md"), "custom").unwrap();
        assert!(apply(d.path(), &ids).is_err());
        assert_eq!(
            std::fs::read_to_string(d.path().join(".forge-toolkit/rust.md")).unwrap(),
            "custom"
        );
    }
}
