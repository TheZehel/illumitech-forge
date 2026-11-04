mod budget;
mod comparison;
mod compatibility;
mod context;
mod daemon;
mod dashboard;
mod demo;
mod duration;
mod engine;
mod findings;
mod guided;
mod importing;
mod installation;
mod issue;
mod launch;
mod native;
mod observability;
mod operational;
mod parallel;
mod planning;
mod project;
mod publication;
mod release_version;
mod review;
mod service;
mod snapshots;
mod templates;
mod terminal_chat;
mod validation_cache;
mod workbench;
mod workbench_http;
mod workflow;
use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
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
    #[arg(long, global = true, hide = true)]
    service_logging: bool,
    #[arg(long, global = true, default_value = ".forge-state")]
    state_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Prepare and execute a reviewed scope; acceptance stays with external gates.
    Workflow {
        #[command(subcommand)]
        command: workflow::Command,
    },
    /// Describe versioned public contracts without opening state or a native client.
    Contracts,
    /// Diagnose or explicitly update a task-local installation (preview by default).
    Installation {
        #[command(subcommand)]
        command: installation::Command,
    },
    /// Prepare a disposable native example; no agent or Python required.
    Demo {
        #[command(subcommand)]
        command: demo::Command,
    },
    /// Consistent, verified state snapshots and fresh-target recovery.
    State {
        #[command(subcommand)]
        command: snapshots::Command,
    },
    /// Import exactly one selected GitHub issue as a preview-first candidate.
    Issue {
        #[command(subcommand)]
        command: issue::Command,
    },
    /// Independent read-only native review; never grants release acceptance.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    Templates {
        #[command(subcommand)]
        command: templates::Command,
    },
    Budget {
        #[command(subcommand)]
        command: BudgetCommand,
    },
    /// Local authenticated UI; Ctrl+C shuts down the native bridge.
    Workbench {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long)]
        app_server: Option<PathBuf>,
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    /// Interactive Illumitech terminal conversation using native authentication and permissions.
    Chat {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        app_server: Option<PathBuf>,
    },
    Feature {
        #[command(subcommand)]
        command: FeatureCommand,
    },
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Internal bounded Windows daemon supervisor, started only by service start.
    #[command(hide = true)]
    ServiceHost {
        #[arg(long)]
        releases_dir: PathBuf,
    },
    /// Preview/install/control a per-user daemon; defaults to preview.
    Service(service::Options),
    /// Painel local somente leitura; Ctrl+C encerra o servidor.
    Dashboard {
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
    Daemon {
        #[arg(long)]
        releases_dir: PathBuf,
        #[arg(long)]
        once: bool,
        #[arg(long, default_value_t = 60)]
        interval_seconds: u64,
        #[arg(long)]
        dry_run: bool,
    },
    DaemonControl {
        #[command(subcommand)]
        command: DaemonControl,
    },
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Explicitly create optional ai-memory routing for this workspace.
        #[arg(long)]
        memory_workspace: Option<String>,
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
        #[arg(long, default_value_t = 100)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    Context {
        #[command(subcommand)]
        command: ContextCommand,
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
enum ReviewCommand {
    Plan {
        manifest: PathBuf,
    },
    Run {
        manifest: PathBuf,
        #[arg(long)]
        app_server: Option<PathBuf>,
    },
}
#[derive(Subcommand)]
enum BudgetCommand {
    Inspect { manifest: PathBuf },
}
#[derive(Subcommand)]
enum FeatureCommand {
    New {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Subcommand)]
enum PlanCommand {
    Propose {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        request: String,
        #[arg(long)]
        ops_root: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        app_server: Option<PathBuf>,
        #[arg(long)]
        previous: Option<PathBuf>,
        /// Attach planning usage to an explicit release budget.
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    Validate {
        path: PathBuf,
        #[arg(long)]
        root: PathBuf,
    },
}
#[derive(Subcommand)]
enum ContextCommand {
    /// Preview bounded context locally; never invokes a model or writes state.
    Plan {
        manifest: PathBuf,
        #[arg(long)]
        task: String,
        #[arg(long)]
        budget_tokens: u64,
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
    /// Compute the next stable version from reviewed changes; --target enforces an exact gate.
    Version {
        #[arg(long)]
        changes: PathBuf,
        #[arg(long)]
        target: Option<String>,
    },
    ImportSpec {
        #[arg(long)]
        root: PathBuf,
        #[arg(long, required = true)]
        spec: Vec<String>,
        #[arg(long)]
        release_id: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    DraftPr {
        manifest: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
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
        /// Check this visibility; never change the remote repository.
        #[arg(long, value_enum, default_value = "PRIVATE")]
        expected_visibility: publication::ExpectedVisibility,
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
enum DaemonControl {
    Status,
    Pause {
        #[arg(long)]
        reason: Option<String>,
    },
    Resume,
    Retry {
        #[arg(long)]
        release: String,
        #[arg(long)]
        reason: String,
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
    /// Compare observed consumption and delivery outcomes; never invokes a model.
    Compare {
        baseline: PathBuf,
        candidate: PathBuf,
    },
    Run {
        manifest: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
pub async fn run() {
    let (program, argv) = launch::arguments(std::env::args_os().collect());
    let mut command = Cli::command();
    if program == "forge" {
        command = command.name("forge").after_help("Sem subcomando: abre o chat no projeto atual. Use forge --root PATH para escolher outra pasta.");
    }
    let arguments = command.get_matches_from(argv);
    let mut cli = Cli::from_arg_matches(&arguments).unwrap_or_else(|e| e.exit());
    let explicit_state =
        arguments.value_source("state_dir") == Some(clap::parser::ValueSource::CommandLine);
    let explicit_root = arguments
        .subcommand_matches("chat")
        .is_some_and(|a| a.value_source("root") == Some(clap::parser::ValueSource::CommandLine));
    let prepared = launch::prepare(&mut cli, explicit_state, explicit_root);
    let logs = if cli.service_logging {
        match service::Logs::new(&cli.state_dir) {
            Ok(logs) => Some(logs),
            Err(e) => {
                eprintln!("service logging refused: {e}");
                std::process::exit(2);
            }
        }
    } else {
        None
    };
    let writer = tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr);
    let writer = logs
        .clone()
        .map(tracing_subscriber::fmt::writer::BoxMakeWriter::new)
        .unwrap_or(writer);
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(!cli.service_logging)
        .with_writer(writer)
        .init();
    let result = if let Err(error) = prepared {
        Err(error)
    } else if matches!(
        &cli.command,
        Command::State {
            command: snapshots::Command::Snapshot { .. }
        }
    ) && !explicit_state
    {
        Err(anyhow::anyhow!(
            "state snapshot requires explicit --state-dir"
        ))
    } else {
        dispatch(&cli).await
    };
    let (value, code) = match result {
        Ok(v) => {
            let status = v["effective_status"]
                .as_str()
                .or_else(|| v["run"]["status"].as_str())
                .or_else(|| v["status"].as_str());
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
            json!({"schema_version":1,"status":"blocked","error_code":"operation_blocked","error":process::redact(&format!("{e:#}"))}),
            2,
        ),
    };
    let value = compatibility::envelope(value);
    if let Some(logs) = logs {
        if logs
            .write("stdout", serde_json::to_string(&value).unwrap().as_bytes())
            .is_err()
        {
            std::process::exit(2);
        }
    } else if cli.json {
        println!("{}", serde_json::to_string(&value).unwrap());
    } else {
        if let Some(summary) = duration::human(&value) {
            println!("{summary}");
        }
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    }
    std::process::exit(code);
}
async fn dispatch(cli: &Cli) -> Result<Value> {
    let ctx = |p: &Path| engine::Context::load(p, &cli.state_dir);
    match &cli.command {
        Command::Workflow { command } => workflow::dispatch(command).await,
        Command::Contracts => Ok(compatibility::contracts()),
        Command::Installation { command } => installation::dispatch(command).await,
        Command::Demo { command } => demo::dispatch(command).await,
        Command::State { command } => snapshots::dispatch(command, &cli.state_dir),
        Command::Issue { command } => issue::dispatch(command).await,
        Command::Review { command } => match command {
            ReviewCommand::Plan { manifest } => review::plan(&ctx(manifest)?),
            ReviewCommand::Run {
                manifest,
                app_server,
            } => review::run(&ctx(manifest)?, app_server.as_deref()).await,
        },
        Command::Templates { command } => templates::dispatch(command),
        Command::Budget {
            command: BudgetCommand::Inspect { manifest },
        } => budget::inspect(&ctx(manifest)?),
        Command::Workbench {
            root,
            port,
            app_server,
            manifest,
        } => {
            workbench::serve(
                root,
                &cli.state_dir,
                *port,
                app_server.clone(),
                manifest.clone(),
            )
            .await
        }
        Command::Chat { root, app_server } => {
            anyhow::ensure!(!cli.json, "chat is interactive; omit --json");
            terminal_chat::run(root, &cli.state_dir, app_server.clone()).await
        }
        Command::Feature { command } => match command {
            FeatureCommand::New {
                root,
                id,
                title,
                ops_root,
                apply,
                ..
            } => guided::feature(root, id, title, ops_root, *apply),
        },
        Command::Plan { command } => match command {
            PlanCommand::Validate { path, root } => planning::validate_file(path, root),
            PlanCommand::Propose {
                root,
                request,
                ops_root,
                output,
                dry_run,
                app_server,
                previous,
                manifest,
            } => {
                let reservation = if let Some(path) = manifest.as_ref().filter(|_| !*dry_run) {
                    let c = ctx(path)?;
                    let root = guided::root(root)?;
                    anyhow::ensure!(
                        c.roots.values().all(|r| r == &root),
                        "planning manifest project mismatch"
                    );
                    let mut nonce = [0u8; 16];
                    getrandom::fill(&mut nonce)
                        .map_err(|e| anyhow::anyhow!("secure operation ID: {e}"))?;
                    Some(budget::Reservation::begin(
                        &c,
                        &format!("planning:{}", forge_domain::digest(&nonce)),
                        "planning",
                    )?)
                } else {
                    None
                };
                planning::propose_reserved(
                    root,
                    request,
                    ops_root,
                    output,
                    *dry_run,
                    app_server.as_deref(),
                    previous.as_deref(),
                    reservation,
                )
                .await
            }
        },
        Command::ServiceHost { releases_dir } => service::host(releases_dir, &cli.state_dir).await,
        Command::Service(options) => service::execute(options, &cli.state_dir),
        Command::Dashboard { port } => dashboard::serve(&cli.state_dir, *port).await,
        Command::Daemon {
            releases_dir,
            once,
            interval_seconds,
            dry_run,
        } => {
            daemon::run(
                releases_dir,
                &cli.state_dir,
                *once,
                *interval_seconds,
                *dry_run,
            )
            .await
        }
        Command::DaemonControl { command } => match command {
            DaemonControl::Status => daemon::control(&cli.state_dir, "status", None),
            DaemonControl::Pause { reason } => {
                daemon::control(&cli.state_dir, "pause", reason.as_deref())
            }
            DaemonControl::Resume => daemon::control(&cli.state_dir, "resume", None),
            DaemonControl::Retry { release, reason } => {
                daemon::retry(&cli.state_dir, release, reason)
            }
        },
        Command::Init {
            path,
            memory_workspace,
        } => project::init(path, memory_workspace.as_deref()),
        Command::Doctor => doctor().await,
        Command::Spec { command } => match command {
            SpecCommand::Init { id, title, root } => project::spec_init(root, id, title),
            SpecCommand::Check { root } => project::spec_check(root),
        },
        Command::Release { command } => match command {
            ReleaseCommand::Version { changes, target } => {
                release_version::inspect(changes, target.as_deref())
            }
            ReleaseCommand::ImportSpec {
                root,
                spec,
                release_id,
                output,
            } => importing::import_spec(root, spec, release_id, output.as_deref()),
            ReleaseCommand::DraftPr { manifest, dry_run } => {
                publication::draft_pr(&ctx(manifest)?, *dry_run).await
            }
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
                importing::import_loop(input, output.as_deref())
            }
            ReleaseCommand::Publish {
                manifest,
                expected_visibility,
                repository,
                tag,
                assets,
                notes,
                dry_run,
            } => {
                publication::release(
                    &ctx(manifest)?,
                    repository,
                    tag,
                    assets,
                    notes,
                    *dry_run,
                    *expected_visibility,
                )
                .await
            }
        },
        Command::Status { manifest } => engine::status(&ctx(manifest)?, false),
        Command::Report {
            manifest,
            limit,
            offset,
        } => engine::status_window(&ctx(manifest)?, true, *limit, *offset),
        Command::Context {
            command:
                ContextCommand::Plan {
                    manifest,
                    task,
                    budget_tokens,
                },
        } => context::plan(&ctx(manifest)?, task, *budget_tokens),
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
            command:
                EvalCommand::Compare {
                    baseline,
                    candidate,
                },
        } => comparison::files(baseline, candidate),
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
    forge_adapters::clients::find_executable(name)
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
        json!({"schema_version":1,"installation":installation::dispatch(&installation::Command::Doctor).await?,"platform":std::env::consts::OS,"operational_support":cfg!(unix),"windows_candidate":cfg!(windows),"authenticated_windows_acceptance":false,"checks":checks}),
    )
}
