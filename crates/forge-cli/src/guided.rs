//! Preview-first feature scaffolding and safe, create-only candidate files.
use anyhow::{Context, Result, ensure};
use forge_domain::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub fn safe_path(path: &Path) -> Result<PathBuf> {
    let path = std::path::absolute(path)?;
    ensure!(
        !path.components().any(|c| matches!(c, Component::ParentDir)),
        "path traversal rejected"
    );
    for p in path.ancestors() {
        if let Ok(m) = p.symlink_metadata() {
            ensure!(!m.file_type().is_symlink(), "symlink redirection rejected");
        }
    }
    Ok(path)
}
pub fn root(path: &Path) -> Result<PathBuf> {
    let path = safe_path(path)?.canonicalize()?;
    ensure!(path.join(".git").exists(), "root must be a Git project");
    Ok(path)
}
pub fn external(path: &Path, root: &Path) -> Result<PathBuf> {
    let path = safe_path(path)?;
    ensure!(
        !path.starts_with(root) && !path.ancestors().any(|p| p.join(".git").exists()),
        "operational files must be outside Git projects"
    );
    Ok(path)
}
pub fn check_file(path: &Path, body: &str) -> Result<()> {
    safe_path(path)?;
    if path.try_exists()? {
        ensure!(
            std::fs::read_to_string(path)? == body,
            "existing file differs; preserving {}",
            path.display()
        );
    }
    Ok(())
}
pub fn create(path: &Path, body: &str) -> Result<()> {
    check_file(path, body)?;
    if path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    create_unix(path, body)?;
    #[cfg(not(unix))]
    anyhow::bail!("guided writes require the supported Unix runtime");
    Ok(())
}
#[cfg(unix)]
fn create_unix(path: &Path, body: &str) -> Result<()> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd, OwnedFd},
            unix::ffi::OsStrExt,
        },
    };
    let parent = path.parent().context("missing parent")?;
    // Keep each verified directory open. A concurrent symlink replacement cannot redirect
    // subsequent mkdir/open calls outside the originally selected directory chain.
    let start = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    ensure!(start >= 0, "unable to open filesystem root");
    let mut directory = unsafe { OwnedFd::from_raw_fd(start) };
    for component in parent.components() {
        let Component::Normal(name) = component else {
            continue;
        };
        let name = CString::new(name.as_bytes())?;
        let open = || unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        let mut fd = open();
        if fd < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            let made = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) };
            ensure!(
                made == 0
                    || std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists,
                "cannot create candidate directory"
            );
            fd = open();
        }
        ensure!(fd >= 0, "candidate parent unavailable or symlinked");
        directory = unsafe { OwnedFd::from_raw_fd(fd) };
    }
    let name = CString::new(path.file_name().context("missing file name")?.as_bytes())?;
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    ensure!(
        fd >= 0,
        "candidate already exists, unsafe or not writable; existing file preserved"
    );
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    file.write_all(body.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub fn stack(root: &Path) -> Vec<&'static str> {
    [
        ("Cargo.toml", "Rust / Cargo"),
        (
            "package.json",
            "JavaScript / npm (inspect scripts before execution)",
        ),
        ("pnpm-lock.yaml", "pnpm"),
        ("pyproject.toml", "Python"),
        ("go.mod", "Go"),
    ]
    .into_iter()
    .filter_map(|(f, s)| root.join(f).is_file().then_some(s))
    .collect()
}
pub fn manifest(root: &Path, id: &str, tasks: Vec<Task>) -> Manifest {
    Manifest {
        schema_version: 1,
        release_id: id.into(),
        repos: BTreeMap::from([(
            "project".into(),
            Repo {
                path: root.into(),
                specs: vec![id.into()],
            },
        )]),
        tasks,
        gates: vec![],
        memory: Memory::default(),
        automation: Automation::default(),
        context: ContextPolicy::default(),
        budget: Budget::default(),
        execution: Default::default(),
        review: Default::default(),
        source: None,
    }
}
pub fn task(id: String, description: String, dependencies: Vec<String>) -> Task {
    Task {
        acceptance: vec![Acceptance::Human {
            id: format!("review-{id}"),
        }],
        id,
        repo: "project".into(),
        description,
        dependencies,
        priority: 100,
        executor: Executor::Codex {
            permission: Permission::ReadOnly,
            model: None,
        },
        timeout_seconds: 1800,
        max_attempts: 1,
        context: None,
        budget: Budget::default(),
    }
}
pub fn feature(root_path: &Path, id: &str, title: &str, ops: &Path, apply: bool) -> Result<Value> {
    feature_candidate(root_path, id, title, ops, apply, true)
}
pub fn feature_candidate(
    root_path: &Path,
    id: &str,
    title: &str,
    ops: &Path,
    apply: bool,
    check: bool,
) -> Result<Value> {
    let root = root(root_path)?;
    let ops = external(ops, &root)?;
    ensure!(
        id.len() >= 5
            && id.len() <= 60
            && id.as_bytes()[..3].iter().all(u8::is_ascii_digit)
            && id.as_bytes()[3] == b'-'
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !id.ends_with('-'),
        "feature ID must be NNN-lowercase-slug"
    );
    ensure!(
        !title.trim().is_empty() && title.len() <= 200 && !title.contains(['\n', '\r', '\0']),
        "title must be one nonempty line"
    );
    let conventions = stack(&root);
    let t1 = format!("{id}-T01");
    let t2 = format!("{id}-T02");
    let m = manifest(
        &root,
        id,
        vec![
            task(
                t1.clone(),
                format!(
                    "Implement {title}; define observable behavior in specs/{id}/spec.md; attach pinned external evaluator before granting write permissions."
                ),
                vec![],
            ),
            task(
                t2.clone(),
                format!(
                    "Verify {title} against external evidence; human review remains revision-bound."
                ),
                vec![t1.clone()],
            ),
        ],
    );
    m.validate()?;
    let base = root.join("specs").join(id);
    let files = vec![
        (
            base.join("spec.md"),
            format!(
                "# {id} — {title}\n\n## Intent\n\n{title}\n\n## Acceptance\n\n- {id}-AC1: Demonstrate the requested behavior with a pinned external evaluator, including failure handling and regression evidence. Evaluator and exact observable outcome require operator review before execution.\n- {id}-AC2: Preserve existing files and native permissions. Review the evaluated revision explicitly.\n\n## Authority\n\nCandidate only. No acceptance recorded; no execution or publication authorized.\n"
            ),
        ),
        (
            base.join("plan.md"),
            format!(
                "# Plan {id}\n\nDetected stack: {}.\n\n1. Review project instructions, interfaces and test scripts.\n2. Implement the scoped behavior without installing dependencies implicitly.\n3. Run operator-pinned external regression checks and inspect changes.\n\nUnknown requirements: exact behavior, commands and evaluator hashes must be reviewed before execution. Operational candidate is external; memory optional; automation disabled.\n",
                if conventions.is_empty() {
                    "unknown; inspect project".into()
                } else {
                    conventions.join(", ")
                }
            ),
        ),
        (
            base.join("tasks.md"),
            format!(
                "# Tasks {id}\n\n- [ ] {t1}: Implement {title}. Acceptance: {id}-AC1. <!-- forge: {{\"dependencies\": [], \"priority\": 100}} -->\n- [ ] {t2}: Verify external evidence and review revision. Acceptance: {id}-AC2. <!-- forge: {{\"dependencies\": [\"{t1}\"], \"priority\": 101}} -->\n"
            ),
        ),
        (
            ops.join("releases").join(format!("{id}.yaml")),
            serde_yaml::to_string(&m)?,
        ),
    ];
    // Validate every conflict before writing any part of a candidate.
    if check {
        for (path, body) in &files {
            check_file(path, body)?;
        }
    }
    if apply {
        for (path, body) in &files {
            create(path, body)?;
        }
    }
    Ok(
        json!({"schema_version":1,"status":if apply {"created"} else {"preview"},"feature_id":id,"stack":conventions,"files":files.iter().map(|(p,b)|json!({"path":p,"content":b})).collect::<Vec<_>>(),"manifest":m,"execution":"not_started","acceptance":"requires_operator_evidence","memory":"optional"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("project with spaces");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let ops = base.join("operations");
        (t, root, ops)
    }
    #[test]
    fn feature_preview_does_not_modify_project() {
        let (_t, r, o) = setup();
        let v = feature(&r, "001-greeting", "Greeting", &o, false).unwrap();
        assert_eq!(v["status"], "preview");
        assert!(!r.join("specs").exists());
        assert!(!o.exists());
    }
    #[test]
    fn feature_creation_is_idempotent_and_preserves_files() {
        let (_t, r, o) = setup();
        std::fs::write(r.join("unrelated"), "keep").unwrap();
        feature(&r, "001-greeting", "Greeting", &o, true).unwrap();
        let first = std::fs::read(r.join("specs/001-greeting/tasks.md")).unwrap();
        feature(&r, "001-greeting", "Greeting", &o, true).unwrap();
        assert_eq!(
            std::fs::read(r.join("specs/001-greeting/tasks.md")).unwrap(),
            first
        );
        std::fs::write(r.join("specs/001-greeting/spec.md"), "human edit").unwrap();
        assert!(feature(&r, "001-greeting", "Greeting", &o, true).is_err());
        assert_eq!(
            std::fs::read_to_string(r.join("specs/001-greeting/spec.md")).unwrap(),
            "human edit"
        );
        assert_eq!(
            std::fs::read_to_string(r.join("unrelated")).unwrap(),
            "keep"
        );
    }
    #[test]
    fn feature_ids_and_dependencies_are_stable() {
        let (_t, r, o) = setup();
        let a = feature(&r, "001-greeting", "Greeting", &o, false).unwrap();
        let b = feature(&r, "001-greeting", "Greeting", &o, false).unwrap();
        assert_eq!(a, b);
        let m: Manifest = serde_json::from_value(a["manifest"].clone()).unwrap();
        assert_eq!(
            m.validate().unwrap(),
            vec!["001-greeting-T01", "001-greeting-T02"]
        );
        assert!(!m.automation.enabled);
        assert!(m.automation.publish.is_none());
        assert!(!m.memory.required);
    }
    #[test]
    fn feature_rejects_traversal_conflicts_and_internal_ops() {
        let (_t, r, o) = setup();
        for id in ["../evil", "001-../../escape", "001-X", "001-", "001_bad"] {
            assert!(feature(&r, id, "Title", &o, true).is_err());
        }
        assert!(feature(&r, "001-safe", "Title", &r.join("ops"), true).is_err());
        std::fs::create_dir_all(o.join("releases")).unwrap();
        std::fs::write(o.join("releases/001-safe.yaml"), "conflict").unwrap();
        assert!(feature(&r, "001-safe", "Title", &o, true).is_err());
        assert!(!r.join("specs").exists());
    }
    #[cfg(unix)]
    #[test]
    fn feature_rejects_symlink_redirection() {
        let (_t, r, o) = setup();
        std::fs::create_dir_all(&o).unwrap();
        std::os::unix::fs::symlink(&o, r.join("specs")).unwrap();
        assert!(feature(&r, "001-safe", "Title", &o, true).is_err());
        assert!(!o.join("001-safe").exists());
    }
}
