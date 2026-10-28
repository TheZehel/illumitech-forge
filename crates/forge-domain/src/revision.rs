use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Revision {
    pub commit: String,
    pub worktree: PathBuf,
    pub fingerprint: String,
}
pub fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()?;
    ensure!(
        out.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?)
}
pub fn revision(root: &Path) -> Result<Revision> {
    use sha2::{Digest, Sha256};
    let commit = git(root, &["rev-parse", "HEAD"])?.trim().to_string();
    let paths = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    let mut names: Vec<_> = paths.split('\0').filter(|p| !p.is_empty()).collect();
    names.sort();
    names.dedup();
    let mut hash = Sha256::new();
    hash.update(commit.as_bytes());
    hash.update(git(root, &["diff", "--cached", "--binary"])?);
    for name in names {
        let p = root.join(name);
        hash.update(name.as_bytes());
        hash.update([0]);
        match std::fs::symlink_metadata(&p) {
            Ok(meta) if meta.file_type().is_symlink() => {
                hash.update(b"link");
                hash.update(std::fs::read_link(p)?.as_os_str().as_encoded_bytes());
            }
            Ok(meta) if meta.is_file() => {
                ensure!(
                    meta.len() <= 64 * 1024 * 1024,
                    "file too large for bounded fingerprint: {name}"
                );
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    hash.update(meta.permissions().mode().to_le_bytes());
                }
                hash.update(std::fs::read(p)?);
            }
            Ok(_) => hash.update(b"non-file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => hash.update(b"deleted"),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(Revision {
        commit,
        worktree: root.to_path_buf(),
        fingerprint: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    })
}
pub fn revisions(roots: &BTreeMap<String, PathBuf>) -> Result<BTreeMap<String, Revision>> {
    roots
        .iter()
        .map(|(k, p)| Ok((k.clone(), revision(p)?)))
        .collect()
}
pub struct WorktreeLocks {
    _files: Vec<std::fs::File>,
}
impl WorktreeLocks {
    pub fn acquire(roots: &BTreeMap<String, PathBuf>) -> Result<Self> {
        let mut paths = Vec::new();
        for root in roots.values() {
            let d = git(root, &["rev-parse", "--absolute-git-dir"])?;
            paths.push(Path::new(d.trim()).join("illumitech-forge.lock"));
        }
        paths.sort();
        paths.dedup();
        let mut files = Vec::new();
        for p in paths {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(p)?;
            fs2::FileExt::try_lock_exclusive(&file)
                .map_err(|_| anyhow::anyhow!("worktree locked by another Forge instance"))?;
            files.push(file);
        }
        Ok(Self { _files: files })
    }
}
