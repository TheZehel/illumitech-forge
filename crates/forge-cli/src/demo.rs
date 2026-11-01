//! Disposable onboarding through the real engine. No inference, Python or remote calls.
use crate::publication::Signals;
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use forge_adapters::process;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const GREETING: &[u8] = b"Hello Forge\n";
const PRESERVED: &[u8] = b"keep\n";

#[derive(Subcommand)]
pub enum Command {
    /// Prepare a NEW directory. Inspect/dry-run/run are separate explicit commands.
    Create { directory: PathBuf },
    #[command(hide = true)]
    Write,
    #[command(hide = true)]
    Verify { root: PathBuf },
}

pub async fn dispatch(command: &Command) -> Result<Value> {
    match command {
        Command::Create { directory } => create(directory).await,
        Command::Write => {
            write_new(&std::env::current_dir()?.join("hello.txt"), GREETING)?;
            Ok(json!({"schema_version":1,"status":"written"}))
        }
        Command::Verify { root } => {
            let root = crate::installation::safe_path(root)?;
            check(&root.join("hello.txt"), GREETING)?;
            check(&root.join("preserved.txt"), PRESERVED)?;
            Ok(
                json!({"schema_version":1,"status":"passed","checks":["greeting artifact","unrelated file preserved"],"native_inference_calls":0}),
            )
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    crate::installation::safe_path(path)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("refusing to replace an existing file: {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

fn check(path: &Path, expected: &[u8]) -> Result<()> {
    crate::installation::safe_path(path)?;
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && meta.len() == expected.len() as u64,
        "demo evidence missing or changed: {}",
        path.display()
    );
    ensure!(
        fs::read(path)? == expected,
        "demo evidence differs: {}",
        path.display()
    );
    Ok(())
}

async fn create(directory: &Path) -> Result<Value> {
    // Git routing inherited from a hook must never redirect this disposable fixture.
    for key in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CONFIG",
    ] {
        ensure!(
            std::env::var_os(key).is_none(),
            "demo preparation refuses inherited {key}; use a shell without Git routing overrides"
        );
    }
    ensure!(
        !directory
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir)),
        "parent traversal refused"
    );
    let absolute = if directory.is_absolute() {
        directory.to_owned()
    } else {
        std::env::current_dir()?.join(directory)
    };
    let parent = absolute
        .parent()
        .context("demo parent directory required")?
        .canonicalize()?;
    let root = crate::installation::safe_path(
        &parent.join(
            absolute
                .file_name()
                .context("new demo directory name required")?,
        ),
    )?;
    ensure!(
        root.to_str().is_some(),
        "demo manifests require UTF-8 paths"
    );
    let binary = std::env::current_exe()?.canonicalize()?;
    let binary_name = binary.to_str().context("executable path must be UTF-8")?;
    let digest = crate::installation::hash(&binary)?;
    // Atomic ownership of a new leaf; includes dangling symlinks and empty directories.
    fs::create_dir(&root).context(
        "choose a NEW demo directory with an existing parent; existing files are preserved",
    )?;
    let root = root.canonicalize()?;
    let repo = root.join("app");
    fs::create_dir(&repo)?;
    let hooks = root.join("empty-git-hooks");
    fs::create_dir(&hooks)?;
    let signals = Signals::new();
    let hooks = hooks.to_str().context("Git hooks path must be UTF-8")?;
    let template = format!("--template={hooks}");
    // Only this synthetic fixture uses the example identity and empty hooks/templates.
    // Neither global Git settings nor native client configuration are changed.
    for args in [
        vec!["init", "-q", &template],
        vec!["config", "--local", "core.hooksPath", hooks],
        vec!["config", "--local", "commit.gpgsign", "false"],
        vec!["config", "--local", "core.fsmonitor", "false"],
        vec![
            "-c",
            "user.name=Forge Demo",
            "-c",
            "user.email=demo@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "Initialize disposable Forge example",
        ],
    ] {
        let argv: Vec<String> = [
            "git".to_owned(),
            "-c".into(),
            format!("core.hooksPath={hooks}"),
            "-c".into(),
            "commit.gpgsign=false".into(),
            "-c".into(),
            "core.fsmonitor=false".into(),
        ]
        .into_iter()
        .chain(args.into_iter().map(str::to_owned))
        .collect();
        let result = process::run(&argv, &repo, 30, signals.receiver.clone()).await?;
        ensure!(
            result.passed(),
            "demo Git preparation failed; preserved {} for inspection: {}",
            root.display(),
            result.stderr
        );
    }
    write_new(&repo.join("preserved.txt"), PRESERVED)?;
    let manifest = root.join("release.yaml");
    let mut inputs = serde_json::Map::new();
    inputs.insert(binary_name.to_owned(), json!(digest));
    let contract = json!({"schema_version":1,"release_id":"hello-forge","repos":{"app":{"path":"app"}},
        "tasks":[{"id":"HELLO","repo":"app","description":"Native no-agent demo: greeting and preservation verified by an external gate.","timeout_seconds":30,"max_attempts":1,
            "executor":{"kind":"command","argv":[binary,"demo","write"]},"acceptance":[{"kind":"gate","id":"greeting"}]}],
        "gates":[{"id":"greeting","argv":[binary,"demo","verify","{repo}"],"inputs":inputs,"timeout_seconds":30}]});
    write_new(&manifest, &serde_json::to_vec_pretty(&contract)?)?;
    write_new(&root.join("README.md"), b"# Disposable Forge demo\n\nInspect and dry-run before executing release.yaml. State and evaluator stay outside app/.\nNo agent, Python, memory or account is needed. Replacing the Forge binary changes the pinned evaluator; recreate in a NEW directory.\n")?;
    let state = root.join("state");
    let prefix = vec![json!(binary), json!("--state-dir"), json!(state)];
    let command = |args: &[&str]| {
        let mut argv = prefix.clone();
        argv.extend(args.iter().map(|v| json!(v)));
        argv.push(json!(manifest));
        argv
    };
    let mut dry_run = command(&["release", "run"]);
    dry_run.push(json!("--dry-run"));
    Ok(
        json!({"schema_version":1,"status":"prepared","directory":root,"repo":repo,"manifest":manifest,"state_dir":state,
        "executed":false,"native_inference_calls":0,"requirements":["Git"],"commands":{
        "inspect":command(&["release","inspect"]),"dry_run":dry_run,"run":command(&["release","run"]),"report":command(&["report"])}}),
    )
}
