mod engine;
mod project;
use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use forge_adapters::process;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::sync::watch;

#[derive(Parser)]
#[command(
    name = "illumitech-forge",
    version,
    about = "Releases com evidências verificáveis / Evidence-backed releases"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true, default_value = ".forge-state")]
    state_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Doctor,
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    Status {
        manifest: PathBuf,
    },
    Report {
        manifest: PathBuf,
    },
    Pause {
        manifest: PathBuf,
    },
    Cancel {
        manifest: PathBuf,
    },
    Resume {
        manifest: PathBuf,
        #[arg(long)]
        reconcile: Option<String>,
    },
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    Tools {
        #[command(subcommand)]
        command: ToolsCommand,
    },
    Eval {
        #[command(subcommand)]
        command: EvalCommand,
    },
}
#[derive(Subcommand)]
enum SpecCommand {
    Init {
        id: String,
        title: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    Check {
        #[arg(default_value = ".")]
        root: PathBuf,
    },
}
#[derive(Subcommand)]
enum ReleaseCommand {
    Inspect {
        manifest: PathBuf,
    },
    Run {
        manifest: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    Approve {
        manifest: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        gate: String,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        fingerprint: String,
    },
    ImportLoop {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Publish {
        manifest: PathBuf,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        tag: String,
        #[arg(long)]
        assets: PathBuf,
        #[arg(long)]
        notes: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
}
#[derive(Subcommand)]
enum MemoryCommand {
    Doctor { manifest: PathBuf },
    Query { manifest: PathBuf, query: String },
}
#[derive(Subcommand)]
enum ToolsCommand {
    List,
    Doctor,
    Plan {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(required = true)]
        recipes: Vec<String>,
    },
    Apply {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(required = true)]
        recipes: Vec<String>,
    },
}
#[derive(Subcommand)]
enum EvalCommand {
    Run {
        manifest: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let result = dispatch(&cli).await;
    let (value, code) = match result {
        Ok(v) => {
            let status = v["run"]["status"].as_str().or_else(|| v["status"].as_str());
            let code = if matches!(
                status,
                Some("failed" | "blocked" | "awaiting_approval" | "cancelled" | "degraded")
            ) {
                2
            } else {
                0
            };
            (v, code)
        }
        Err(e) => (
            json!({"schema_version":1,"status":"blocked","error":process::redact(&format!("{e:#}"))}),
            2,
        ),
    };
    if cli.json {
        println!("{}", serde_json::to_string(&value).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    }
    std::process::exit(code);
}
async fn dispatch(cli: &Cli) -> Result<Value> {
    let ctx = |p: &Path| engine::Context::load(p, &cli.state_dir);
    match &cli.command {
        Command::Init { path } => project::init(path),
        Command::Doctor => doctor().await,
        Command::Spec { command } => match command {
            SpecCommand::Init { id, title, root } => project::spec_init(root, id, title),
            SpecCommand::Check { root } => project::spec_check(root),
        },
        Command::Release { command } => match command {
            ReleaseCommand::Inspect { manifest } => ctx(manifest)?.inspect(),
            ReleaseCommand::Run { manifest, dry_run } => {
                let c = ctx(manifest)?;
                if *dry_run {
                    c.inspect()
                } else {
                    engine::execute(c, false, None).await
                }
            }
            ReleaseCommand::Approve {
                manifest,
                task,
                gate,
                reviewer,
                fingerprint,
            } => engine::approve(&ctx(manifest)?, task, gate, reviewer, fingerprint),
            ReleaseCommand::ImportLoop { input, output } => {
                project::import_loop(input, output.as_deref())
            }
            ReleaseCommand::Publish {
                manifest,
                repository,
                tag,
                assets,
                notes,
                dry_run,
            } => publish(&ctx(manifest)?, repository, tag, assets, notes, *dry_run).await,
        },
        Command::Status { manifest } => engine::status(&ctx(manifest)?, false),
        Command::Report { manifest } => engine::status(&ctx(manifest)?, true),
        Command::Pause { manifest } => engine::control(&ctx(manifest)?, "pause"),
        Command::Cancel { manifest } => engine::control(&ctx(manifest)?, "cancel"),
        Command::Resume {
            manifest,
            reconcile,
        } => engine::execute(ctx(manifest)?, true, reconcile.clone()).await,
        Command::Memory { command } => match command {
            MemoryCommand::Doctor { manifest } => engine::memory_check(&ctx(manifest)?, None).await,
            MemoryCommand::Query { manifest, query } => {
                engine::memory_check(&ctx(manifest)?, Some(query)).await
            }
        },
        Command::Tools { command } => match command {
            ToolsCommand::List => {
                Ok(json!({"schema_version":1,"recipes":forge_catalog::recipes()?}))
            }
            ToolsCommand::Plan { root, recipes } => {
                Ok(json!({"schema_version":1,"changes":forge_catalog::plan(root,recipes)?}))
            }
            ToolsCommand::Apply { root, recipes } => {
                Ok(json!({"schema_version":1,"changes":forge_catalog::apply(root,recipes)?}))
            }
            ToolsCommand::Doctor => {
                let mut results = Vec::new();
                for r in forge_catalog::recipes()? {
                    let discovered = r.executable.as_deref().and_then(find_executable);
                    results.push(json!({"id":r.id,"available":true,"installed_recipe":std::env::current_dir()?.join(format!(".forge-toolkit/{}.md",r.id)).is_file(),"discovered":discovered.is_some(),"tested":false,"upstream_revision":r.revision,"last_verified":r.last_verified}));
                }
                Ok(json!({"schema_version":1,"tools":results}))
            }
        },
        Command::Eval {
            command: EvalCommand::Run { manifest, dry_run },
        } => {
            let c = ctx(manifest)?;
            if *dry_run {
                c.inspect()
            } else {
                engine::execute(c, false, None).await
            }
        }
    }
}
fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|v| {
        std::env::split_paths(&v)
            .map(|p| p.join(name))
            .find(|p| p.is_file())
    })
}
async fn doctor() -> Result<Value> {
    let mut checks = Vec::new();
    for (name, args, features) in [
        ("git", vec!["--version"], vec![]),
        ("codex", vec!["exec", "--help"], vec!["--json", "--sandbox"]),
        (
            "claude",
            vec!["--help"],
            vec!["--print", "--output-format", "--permission-mode"],
        ),
    ] {
        let mut argv = vec![name.to_string()];
        argv.extend(args.iter().map(|s| s.to_string()));
        let (_tx, rx) = watch::channel(false);
        let result = process::run(&argv, &std::env::current_dir()?, 15, rx).await;
        let passed = result
            .as_ref()
            .is_ok_and(|r| r.passed() && features.iter().all(|f| r.stdout.contains(f)));
        checks.push(json!({"name":name,"discovered":find_executable(name).is_some(),"capabilities_verified":passed,"authentication_tested":false}));
    }
    Ok(
        json!({"schema_version":1,"platform":std::env::consts::OS,"operational_support":cfg!(unix),"checks":checks}),
    )
}
async fn publish(
    c: &engine::Context,
    repository: &str,
    tag: &str,
    assets: &Path,
    notes: &Path,
    dry_run: bool,
) -> Result<Value> {
    let _locks = forge_domain::WorktreeLocks::acquire(&c.roots)?;
    let run = engine::assert_publishable(c)?;
    ensure!(
        repository.split('/').count() == 2 && repository.split('/').all(forge_domain::valid_id),
        "invalid repository"
    );
    ensure!(tag == "v0.1.0", "this release contract supports v0.1.0");
    ensure!(notes.is_file(), "release notes missing");
    let checksums = std::fs::read_to_string(assets.join("SHA256SUMS"))?;
    let mut files = Vec::new();
    for name in [
        "illumitech-forge-v0.1.0-aarch64-apple-darwin.tar.gz",
        "illumitech-forge-v0.1.0-x86_64-unknown-linux-gnu.tar.gz",
    ] {
        let path = assets.join(name);
        let bytes = std::fs::read(&path).context("required release binary archive absent")?;
        let line = format!("{}  {}", forge_domain::digest(&bytes), name);
        ensure!(checksums.lines().any(|l| l == line), "checksum mismatch");
        files.push(path);
    }
    files.push(assets.join("SHA256SUMS"));
    if dry_run {
        return Ok(
            json!({"schema_version":1,"status":"passed","dry_run":true,"repository":repository,"tag":tag,"assets":files}),
        );
    }
    let (_tx, rx) = watch::channel(false);
    let view = process::run(
        &[
            "gh".into(),
            "repo".into(),
            "view".into(),
            repository.into(),
            "--json".into(),
            "visibility".into(),
        ],
        &c.base,
        30,
        rx.clone(),
    )
    .await?;
    ensure!(
        view.passed() && serde_json::from_str::<Value>(&view.stdout)?["visibility"] == "PRIVATE",
        "target repository must be private"
    );
    let mut argv = vec![
        "gh".into(),
        "release".into(),
        "create".into(),
        tag.into(),
        "--repo".into(),
        repository.into(),
        "--verify-tag".into(),
        "--title".into(),
        format!("Illumitech Forge {tag}"),
        "--notes-file".into(),
        notes.display().to_string(),
    ];
    argv.extend(files.iter().map(|p| p.display().to_string()));
    let result = process::run(&argv, &c.base, 120, rx).await?;
    ensure!(result.passed(), "publication failed: {}", result.stderr);
    let mut store = forge_domain::Store::open(&c.db, false)?;
    store.save(
        &run,
        "published",
        &json!({"repository":repository,"tag":tag,"url":result.stdout}),
    )?;
    Ok(json!({"schema_version":1,"published":true,"url":result.stdout}))
}
