//! Per-user supervisor definitions. Rendering and preview never contact a supervisor.
use anyhow::{Context, Result, ensure};
use clap::{Args, ValueEnum};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Macos,
    Linux,
    Windows,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Action {
    Plan,
    Apply,
    Status,
    Start,
    Stop,
    Remove,
}
#[derive(Args)]
pub struct Options {
    #[arg(value_enum, default_value = "plan")]
    action: Action,
    #[arg(long)]
    executable: PathBuf,
    #[arg(long)]
    releases_dir: PathBuf,
    /// Store supervisor definitions separately (e.g. an ownership-enabled macOS volume).
    #[arg(long)]
    definition_dir: Option<PathBuf>,
    /// Cross-platform rendering only; mutation must use the native platform.
    #[arg(long, value_enum)]
    platform: Option<Platform>,
}
#[derive(Debug)]
struct Plan {
    platform: Platform,
    id: String,
    file: PathBuf,
    content: String,
}

#[cfg(not(windows))]
pub async fn host(_releases: &Path, _state: &Path) -> Result<Value> {
    anyhow::bail!("service-host is only available on Windows")
}

#[cfg(windows)]
pub async fn host(releases: &Path, state: &Path) -> Result<Value> {
    use forge_adapters::process;
    use fs2::FileExt;
    use std::time::Duration;
    use tokio::sync::watch;
    external(releases)?;
    external(state)?;
    ensure!(releases.is_dir(), "release queue must exist");
    fs::create_dir_all(state)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(state.join("service-host.lock"))?;
    lock.try_lock_exclusive()
        .context("another service host owns this state")?;
    let argv = vec![
        text(&std::env::current_exe()?)?.into(),
        "--state-dir".into(),
        text(state)?.into(),
        "--service-logging".into(),
        "daemon".into(),
        "--releases-dir".into(),
        text(releases)?.into(),
    ];
    let (cancel, rx) = watch::channel(false);
    let work = async {
        for attempt in 1..=3 {
            let code = process::service_child(&argv, state, rx.clone()).await?;
            if code == Some(0) || code.is_none() {
                return Ok(json!({"status":"passed","attempts":attempt}));
            }
            ensure!(
                code != Some(2),
                "daemon reported a blocked contract; explicit reconciliation required"
            );
            if attempt == 3 {
                anyhow::bail!(
                    "daemon failed after three service attempts; explicit service start required"
                );
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
        unreachable!()
    };
    tokio::pin!(work);
    tokio::select! {
        result = &mut work => result,
        _ = process::interrupt() => {
            let _ = cancel.send(true);
            // Dropping the child future closes its Job even on an abrupt scheduler stop.
            Ok(json!({"status":"passed","cancelled":true}))
        }
    }
}
fn hash(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn native() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(windows) {
        Platform::Windows
    } else {
        Platform::Linux
    }
}
fn text(p: &Path) -> Result<&str> {
    let s = p.to_str().context("paths must be Unicode")?;
    ensure!(
        !s.chars().any(char::is_control),
        "control characters in path"
    );
    Ok(s)
}
/// Reject symbolic links and Windows reparse points in every existing component.
fn safe_path(p: &Path) -> Result<()> {
    ensure!(
        p.is_absolute(),
        "absolute paths required (including explicit --state-dir)"
    );
    text(p)?;
    ensure!(
        !p.components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir)),
        "non-normal path"
    );
    for part in p.ancestors() {
        match fs::symlink_metadata(part) {
            Ok(m) => {
                ensure!(
                    !m.file_type().is_symlink(),
                    "redirected path refused: {}",
                    part.display()
                );
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    ensure!(m.file_attributes() & 0x400 == 0, "reparse point refused");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
fn external(p: &Path) -> Result<()> {
    safe_path(p)?;
    ensure!(
        !p.ancestors().any(|a| a.join(".git").exists()),
        "operational paths must be outside Git worktrees"
    );
    Ok(())
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn unit(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$")
    )
}
/// Windows CRT quoting, including trailing backslashes before the closing quote.
fn windows_arg(s: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in s.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        out.push_str(&"\\".repeat(if c == '"' { slashes * 2 + 1 } else { slashes }));
        slashes = 0;
        out.push(c);
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}
fn render(platform: Platform, id: &str, argv: &[String]) -> String {
    let marker = format!(
        "illumitech-forge-service-v1:{}",
        hash(&serde_json::to_string(argv).unwrap())
    );
    match platform {
        Platform::Macos => format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>{id}</string><key>Comment</key><string>{marker}</string><key>ProgramArguments</key><array>{}</array><key>RunAtLoad</key><true/><key>KeepAlive</key><true/><key>ThrottleInterval</key><integer>10</integer><key>StandardOutPath</key><string>/dev/null</string><key>StandardErrorPath</key><string>/dev/null</string></dict></plist>\n",
            argv.iter()
                .map(|s| format!("<string>{}</string>", xml(s)))
                .collect::<String>()
        ),
        Platform::Linux => format!(
            "# {marker}\n[Unit]\nDescription=Illumitech Forge user daemon\n[Service]\nType=simple\nExecStart={}\nRestart=always\nRestartSec=10\nKillMode=control-group\nStandardOutput=null\nStandardError=null\n",
            argv.iter().map(|s| unit(s)).collect::<Vec<_>>().join(" ")
        ),
        // No triggers: registration alone never schedules a run. Interactive token only, no password.
        Platform::Windows => format!(
            "<?xml version=\"1.0\" encoding=\"UTF-16\"?>\n<Task version=\"1.2\" xmlns=\"http://schemas.microsoft.com/windows/2004/02/mit/task\"><RegistrationInfo><Description>{marker}</Description></RegistrationInfo><Triggers/><Principals><Principal id=\"User\"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals><Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><ExecutionTimeLimit>PT0S</ExecutionTimeLimit></Settings><Actions Context=\"User\"><Exec><Command>{}</Command><Arguments>{}</Arguments></Exec></Actions></Task>\n",
            xml(&argv[0]),
            xml(&argv[1..]
                .iter()
                .map(|s| windows_arg(s))
                .collect::<Vec<_>>()
                .join(" "))
        ),
    }
}
fn plan(options: &Options, state: &Path) -> Result<Plan> {
    safe_path(&options.executable)?;
    external(&options.releases_dir)?;
    external(state)?;
    if matches!(options.action, Action::Plan | Action::Apply | Action::Start) {
        ensure!(options.executable.is_file(), "executable must exist");
        ensure!(options.releases_dir.is_dir(), "releases-dir must exist");
    }
    let platform = options.platform.unwrap_or_else(native);
    let id = format!("org.illumitech.forge.{}", &hash(text(state)?)[..20]);
    let mut argv = vec![
        text(&options.executable)?.into(),
        "--state-dir".into(),
        text(state)?.into(),
        "--service-logging".into(),
        "daemon".into(),
        "--releases-dir".into(),
        text(&options.releases_dir)?.into(),
    ];
    if platform == Platform::Windows {
        // On-demand Task Scheduler failure restarts were not reliable in native CI.
        // The directly executed host owns bounded retries and a kill-on-close Job.
        argv.remove(3);
        argv[3] = "service-host".into();
    }
    // Staged outside login-discovery directories. apply cannot activate at next login either.
    let extension = match platform {
        Platform::Macos => "plist",
        Platform::Linux => "service",
        Platform::Windows => "xml",
    };
    let definition_dir = options
        .definition_dir
        .clone()
        .unwrap_or_else(|| state.join("services"));
    external(&definition_dir)?;
    let file = definition_dir.join(format!("{id}.{extension}"));
    safe_path(&file)?;
    Ok(Plan {
        platform,
        content: render(platform, &id, &argv),
        id,
        file,
    })
}
fn definition_bytes(p: &Plan) -> Vec<u8> {
    if p.platform == Platform::Windows {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(p.content.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    } else {
        p.content.as_bytes().to_vec()
    }
}
fn intact(p: &Plan) -> Result<bool> {
    safe_path(&p.file)?;
    if !p.file.exists() {
        return Ok(false);
    }
    ensure!(
        fs::read(&p.file)? == definition_bytes(p),
        "modified/unrelated definition preserved; restore the reviewed definition before control/removal"
    );
    Ok(true)
}
fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    safe_path(path)?;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    if let Err(e) = f.write_all(bytes).and_then(|_| f.sync_all()) {
        drop(f);
        let _ = fs::remove_file(path);
        return Err(e.into());
    }
    Ok(())
}
fn install(p: &Plan) -> Result<bool> {
    if intact(p)? {
        return Ok(false);
    }
    fs::create_dir_all(p.file.parent().unwrap())?;
    create(&p.file, &definition_bytes(p))?;
    Ok(true)
}
trait Supervisor {
    fn user_config_dir(&self) -> Result<PathBuf> {
        let home = PathBuf::from(
            std::env::var_os("HOME").context("HOME required for systemd user units")?,
        );
        Ok(std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config")))
    }
    fn call(&self, program: &str, args: &[String]) -> Result<(bool, String)>;
}
struct Native;
impl Supervisor for Native {
    fn call(&self, program: &str, args: &[String]) -> Result<(bool, String)> {
        let resolved = match program {
            "systemctl" => PathBuf::from("/usr/bin/systemctl"),
            "schtasks.exe" => {
                PathBuf::from(std::env::var_os("SystemRoot").context("SystemRoot required")?)
                    .join("System32/schtasks.exe")
            }
            _ => PathBuf::from(program),
        };
        let output = Command::new(resolved)
            .args(args)
            .output()
            .with_context(|| format!("{program}: native user session unavailable"))?;
        // schtasks emits UTF-16 XML on some Windows versions.
        let bytes = &output.stdout;
        let out = if bytes.starts_with(&[0xff, 0xfe]) || bytes.get(1) == Some(&0) {
            let offset = usize::from(bytes.starts_with(&[0xff, 0xfe])) * 2;
            String::from_utf16(
                &bytes[offset..]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )?
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        };
        Ok((output.status.success(), out))
    }
}
fn invoke(s: &impl Supervisor, program: &str, args: &[&str]) -> Result<(bool, String)> {
    s.call(
        program,
        &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
    )
}
fn checked(s: &impl Supervisor, program: &str, args: &[&str]) -> Result<String> {
    let (ok, out) = invoke(s, program, args)?;
    ensure!(
        ok,
        "{program} {} failed; inspect native user session (no administrator fallback)",
        args.first().unwrap_or(&"")
    );
    Ok(out)
}
fn target(p: &Plan, s: &impl Supervisor) -> Result<String> {
    let uid = checked(s, "/usr/bin/id", &["-u"])?;
    ensure!(
        uid.trim().chars().all(|c| c.is_ascii_digit()) && !uid.trim().is_empty(),
        "invalid user ID"
    );
    Ok(format!("gui/{}/{}", uid.trim(), p.id))
}
fn task_snapshot(p: &Plan) -> PathBuf {
    p.file.with_extension("task-owned.xml")
}
fn windows_owned(p: &Plan, s: &impl Supervisor) -> Result<bool> {
    let snapshot = task_snapshot(p);
    safe_path(&snapshot)?;
    let (exists, xml) = invoke(s, "schtasks.exe", &["/Query", "/TN", &p.id, "/XML"])?;
    if !exists {
        ensure!(
            !snapshot.exists(),
            "registered task missing or inaccessible; preserve receipt for reconciliation"
        );
        return Ok(false);
    }
    ensure!(
        snapshot.exists() && fs::read_to_string(snapshot)? == xml,
        "modified/unrelated native task preserved"
    );
    Ok(true)
}
fn linux_link(p: &Plan, s: &impl Supervisor) -> Result<PathBuf> {
    let link = s
        .user_config_dir()?
        .join("systemd/user")
        .join(format!("{}.service", p.id));
    // The leaf is our only permitted link; every parent must be unredirected.
    safe_path(link.parent().unwrap())?;
    if let Ok(meta) = fs::symlink_metadata(&link) {
        ensure!(
            meta.file_type().is_symlink() && fs::read_link(&link)? == p.file,
            "modified/unrelated systemd unit preserved"
        );
    }
    Ok(link)
}
fn control(p: &Plan, action: Action, s: &impl Supervisor) -> Result<Value> {
    ensure!(intact(p)?, "service not installed; run apply first");
    let output = match p.platform {
        Platform::Macos => {
            let target = target(p, s)?;
            let (loaded, status) = invoke(s, "/bin/launchctl", &["print", &target])?;
            if !loaded {
                checked(
                    s,
                    "/bin/launchctl",
                    &["print", target.rsplit_once('/').unwrap().0],
                )?;
            }
            if loaded {
                ensure!(
                    status
                        .lines()
                        .any(|line| line.trim() == format!("path = {}", p.file.display())),
                    "modified/unrelated launchd job preserved"
                );
            }
            match action {
                Action::Start if !loaded => {
                    let started = checked(
                        s,
                        "/bin/launchctl",
                        &[
                            "bootstrap",
                            target.rsplit_once('/').unwrap().0,
                            text(&p.file)?,
                        ],
                    );
                    if started.is_err() {
                        let (present, current) = invoke(s, "/bin/launchctl", &["print", &target])?;
                        if present {
                            ensure!(
                                current
                                    .lines()
                                    .any(|line| line.trim()
                                        == format!("path = {}", p.file.display())),
                                "partial start left unrelated job; preserved"
                            );
                            checked(s, "/bin/launchctl", &["bootout", &target])?;
                        }
                    }
                    started?
                }
                Action::Stop | Action::Remove if loaded => {
                    checked(s, "/bin/launchctl", &["bootout", &target])?
                }
                _ => status,
            }
        }
        Platform::Linux => {
            let link = linux_link(p, s)?;
            let name = format!("{}.service", p.id);
            let (available, inventory) = invoke(
                s,
                "systemctl",
                &[
                    "--user",
                    "show",
                    &name,
                    "--property=LoadState,FragmentPath,DropInPaths",
                ],
            )?;
            ensure!(
                available || inventory.lines().any(|line| line == "LoadState=not-found"),
                "systemd user session unavailable"
            );
            ensure!(
                inventory
                    .lines()
                    .any(|line| line.starts_with("FragmentPath="))
                    && inventory
                        .lines()
                        .any(|line| line.starts_with("DropInPaths=")),
                "incomplete systemd inventory"
            );
            for line in inventory.lines() {
                if let Some(path) = line.strip_prefix("FragmentPath=") {
                    ensure!(
                        path.is_empty() || path == text(&p.file)? || path == text(&link)?,
                        "unrelated loaded systemd unit preserved"
                    );
                }
                if let Some(paths) = line.strip_prefix("DropInPaths=") {
                    ensure!(
                        paths.is_empty(),
                        "modified systemd unit has drop-ins; preserved"
                    );
                }
            }
            if action == Action::Start {
                let fresh = fs::symlink_metadata(&link).is_err();
                checked(s, "systemctl", &["--user", "link", text(&p.file)?])?;
                let started = checked(s, "systemctl", &["--user", "daemon-reload"])
                    .and_then(|_| checked(s, "systemctl", &["--user", "start", &name]));
                if started.is_err() && fresh {
                    // Stop a potentially partially started job before unlinking its definition.
                    checked(s, "systemctl", &["--user", "stop", &name])?;
                    // Only remove the exact newly created link, never a changed replacement.
                    if linux_link(p, s).is_ok() {
                        let _ = fs::remove_file(&link);
                        let _ = checked(s, "systemctl", &["--user", "daemon-reload"]);
                    }
                }
                started?
            } else if action == Action::Stop || action == Action::Remove {
                if inventory
                    .lines()
                    .any(|line| line.starts_with("FragmentPath=") && line != "FragmentPath=")
                {
                    checked(s, "systemctl", &["--user", "stop", &name])?;
                }
                if fs::symlink_metadata(&link).is_ok() {
                    linux_link(p, s)?;
                    fs::remove_file(link)?;
                    checked(s, "systemctl", &["--user", "daemon-reload"])?;
                }
                String::new()
            } else {
                checked(
                    s,
                    "systemctl",
                    &[
                        "--user",
                        "show",
                        &name,
                        "--property=LoadState,ActiveState,SubState,Result,FragmentPath",
                    ],
                )?
            }
        }
        Platform::Windows => {
            let owned = windows_owned(p, s)?;
            if action == Action::Start {
                if !owned {
                    checked(
                        s,
                        "schtasks.exe",
                        &["/Create", "/TN", &p.id, "/XML", text(&p.file)?],
                    )?;
                    let capture = checked(s, "schtasks.exe", &["/Query", "/TN", &p.id, "/XML"])
                        .and_then(|xml| create(&task_snapshot(p), xml.as_bytes()));
                    if let Err(e) = capture {
                        // This invocation created the task without /F; no preexisting task was adopted.
                        let _ = checked(s, "schtasks.exe", &["/Delete", "/TN", &p.id, "/F"]);
                        return Err(
                            e.context("registration rolled back; inspect task if rollback failed")
                        );
                    }
                }
                let run = checked(s, "schtasks.exe", &["/Run", "/TN", &p.id]);
                if run.is_err() && !owned && windows_owned(p, s)? {
                    checked(s, "schtasks.exe", &["/Delete", "/TN", &p.id, "/F"])?;
                    fs::remove_file(task_snapshot(p))?;
                }
                run?
            } else if owned && (action == Action::Remove || action == Action::Stop) {
                // End may report a non-running task. Delete still removes only an intact owned task.
                let ended = invoke(s, "schtasks.exe", &["/End", "/TN", &p.id])?;
                if action == Action::Remove {
                    windows_owned(p, s)?;
                    checked(s, "schtasks.exe", &["/Delete", "/TN", &p.id, "/F"])?;
                    fs::remove_file(task_snapshot(p))?;
                } else {
                    ensure!(ended.0, "task stop failed; inspect scheduler state");
                }
                String::new()
            } else if owned {
                checked(
                    s,
                    "schtasks.exe",
                    &["/Query", "/TN", &p.id, "/FO", "LIST", "/V"],
                )?
            } else {
                "not_registered_or_inaccessible".into()
            }
        }
    };
    if action == Action::Remove {
        ensure!(intact(p)?, "definition disappeared");
        fs::remove_file(&p.file)?;
    }
    Ok(
        json!({"status":"passed","action":format!("{action:?}").to_lowercase(),"supervisor":forge_adapters::process::redact(&output)}),
    )
}
pub fn execute(options: &Options, state: &Path) -> Result<Value> {
    let p = plan(options, state)?;
    if options.action == Action::Plan {
        return Ok(
            json!({"status":"preview","id":p.id,"path":p.file,"definition":p.content,"sha256":hash(&p.content),"activation":"explicit start only","rollback":"stop, then remove; state and logs retained"}),
        );
    }
    ensure!(
        p.platform == native(),
        "cross-platform mutation/control refused"
    );
    if options.action != Action::Status {
        fs::create_dir_all(p.file.parent().unwrap())?;
        let lock_path = p.file.with_extension("operation.lock");
        safe_path(&lock_path)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        fs2::FileExt::try_lock_exclusive(&lock)
            .context("another service operation owns this state")?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let receipt = p
            .file
            .with_extension(format!("{stamp}-{}.receipt.json", std::process::id()));
        let intent = json!({"schema_version":1,"action":format!("{:?}", options.action),"definition_sha256":hash(&p.content),"phase":"intent"});
        create(&receipt, &serde_json::to_vec_pretty(&intent)?)?;
        let result = mutate(&p, options.action, &Native);
        let outcome = match &result {
            Ok(value) => value.clone(),
            Err(error) => {
                json!({"status":"failed","error":forge_adapters::process::redact(&format!("{error:#}"))})
            }
        };
        create(
            &receipt.with_extension("result.json"),
            &serde_json::to_vec_pretty(&outcome)?,
        )?;
        fs2::FileExt::unlock(&lock)?;
        return result;
    }
    if !intact(&p)? {
        return Ok(
            json!({"status":"passed","installed":false,"supervisor_state":"not_queried_without_owned_definition"}),
        );
    }
    status_detail(control(&p, options.action, &Native)?, state)
}
fn status_detail(mut result: Value, state: &Path) -> Result<Value> {
    // A supervisor may start a process before its SQLite schema is initialized.
    // Inspection reports unavailable state explicitly instead of claiming activity.
    let daemon = match crate::daemon::control(state, "status", None) {
        Ok(daemon) => daemon,
        Err(error) => {
            result["installed"] = json!(true);
            result["heartbeat_stale"] = json!(true);
            result["runtime_state"] = json!("unavailable");
            result["daemon"] = json!({});
            result["daemon_error"] = json!(forge_adapters::process::redact(&format!("{error:#}")));
            return Ok(result);
        }
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let stale = daemon["heartbeat"]["at_epoch"]
        .as_u64()
        .is_none_or(|at| now.saturating_sub(at) > 30);
    result["installed"] = json!(true);
    result["heartbeat_stale"] = json!(stale);
    result["runtime_state"] = json!(if daemon["control"]["paused"] == true {
        "paused"
    } else if daemon["last_cycle"]["status"] == "blocked" {
        "failed"
    } else if stale {
        "heartbeat_stale"
    } else if daemon["heartbeat"]["running"] == true {
        "active"
    } else {
        "stopped"
    });
    result["daemon"] = daemon;
    Ok(result)
}
fn mutate(p: &Plan, action: Action, supervisor: &impl Supervisor) -> Result<Value> {
    if action == Action::Apply {
        let created = install(p)?;
        return Ok(
            json!({"status":"passed","installed":true,"created":created,"activation":"unchanged; apply never starts or enables"}),
        );
    }
    if !intact(p)? {
        ensure!(action == Action::Remove, "service not installed");
        return Ok(
            json!({"status":"passed","installed":false,"supervisor_state":"not_queried_without_owned_definition"}),
        );
    }
    control(p, action, supervisor)
}

/// One active file plus two archives, each at most 1 MiB; no native unbounded log sink.
#[derive(Clone)]
pub struct Logs {
    state: PathBuf,
    lock: std::sync::Arc<std::sync::Mutex<()>>,
}
impl Logs {
    pub fn new(state: &Path) -> Result<Self> {
        external(state)?;
        let dir = state.join("service-logs");
        safe_path(&dir)?;
        fs::create_dir_all(&dir)?;
        Ok(Self {
            state: dir,
            lock: Default::default(),
        })
    }
    pub fn write(&self, stream: &str, bytes: &[u8]) -> Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| anyhow::anyhow!("log lock poisoned"))?;
        let redacted = forge_adapters::process::redact(&String::from_utf8_lossy(bytes));
        let mut end = redacted.len().min(1024 * 1024);
        while !redacted.is_char_boundary(end) {
            end -= 1;
        }
        let bytes = &redacted.as_bytes()[..end];
        let paths = [
            self.state.join(format!("{stream}.log")),
            self.state.join(format!("{stream}.1.log")),
            self.state.join(format!("{stream}.2.log")),
        ];
        for p in &paths {
            safe_path(p)?;
        }
        if paths[0].metadata().map(|m| m.len()).unwrap_or(0) + bytes.len() as u64 > 1024 * 1024 {
            if paths[2].exists() {
                fs::remove_file(&paths[2])?;
            }
            if paths[1].exists() {
                fs::rename(&paths[1], &paths[2])?;
            }
            if paths[0].exists() {
                fs::rename(&paths[0], &paths[1])?;
            }
        }
        let mut opts = fs::OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        opts.open(&paths[0])?.write_all(bytes)?;
        Ok(())
    }
}
pub struct LogEvent {
    logs: Logs,
    bytes: Vec<u8>,
}
impl Write for LogEvent {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        // Limit buffering even for one extremely large formatted event.
        let remaining = (1024 * 1024_usize).saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&bytes[..remaining.min(bytes.len())]);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Drop for LogEvent {
    fn drop(&mut self) {
        let _ = self.logs.write("stderr", &self.bytes);
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Logs {
    type Writer = LogEvent;
    fn make_writer(&'a self) -> Self::Writer {
        LogEvent {
            logs: self.clone(),
            bytes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Fake {
        config: Option<PathBuf>,
        calls: RefCell<Vec<Vec<String>>>,
        replies: RefCell<std::collections::VecDeque<(bool, String)>>,
    }
    impl Fake {
        fn new(replies: &[(bool, &str)]) -> Self {
            Self {
                config: None,
                calls: Default::default(),
                replies: RefCell::new(replies.iter().map(|(ok, s)| (*ok, (*s).into())).collect()),
            }
        }
    }
    impl Supervisor for Fake {
        fn user_config_dir(&self) -> Result<PathBuf> {
            self.config
                .clone()
                .context("test must provide isolated config directory")
        }
        fn call(&self, program: &str, args: &[String]) -> Result<(bool, String)> {
            self.calls.borrow_mut().push(
                std::iter::once(program.to_owned())
                    .chain(args.iter().cloned())
                    .collect(),
            );
            Ok(self
                .replies
                .borrow_mut()
                .pop_front()
                .expect("unexpected supervisor mutation"))
        }
    }
    fn fixture(platform: Platform) -> (tempfile::TempDir, Plan) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let argv = vec![
            "/opt/Forge space/日本語 & %.exe".into(),
            "--state-dir".into(),
            "/ops/state space".into(),
            "daemon".into(),
        ];
        (
            temp,
            Plan {
                platform,
                id: "org.illumitech.forge.fixture".into(),
                file: root.join("definition"),
                content: render(platform, "org.illumitech.forge.fixture", &argv),
            },
        )
    }
    #[test]
    fn platform_escaping_and_no_shell() {
        assert_eq!(xml("a&<>'\"日本語"), "a&amp;&lt;&gt;&apos;&quot;日本語");
        assert_eq!(unit("a% $x \\ \""), "\"a%% $$x \\\\ \\\"\"");
        assert_eq!(windows_arg("C:\\space path\\"), "\"C:\\space path\\\\\"");
        assert_eq!(windows_arg("a\\\"b"), "\"a\\\\\\\"b\"");
        for platform in [Platform::Macos, Platform::Linux, Platform::Windows] {
            let (_temp, p) = fixture(platform);
            assert!(p.content.contains("illumitech-forge-service-v1:"));
            assert!(!p.content.contains("/bin/sh"));
            assert!(!p.content.contains("powershell"));
            assert!(!p.content.contains("[Install]"));
            if platform == Platform::Windows {
                assert!(p.content.contains("<Triggers/>"));
                assert!(p.content.contains("<RunLevel>LeastPrivilege</RunLevel>"));
                assert!(definition_bytes(&p).starts_with(&[0xff, 0xfe]));
            }
        }
    }
    #[test]
    fn apply_is_idempotent_without_supervisor_and_preserves_changes() {
        let (_temp, p) = fixture(Platform::Macos);
        let fake = Fake::new(&[]);
        assert_eq!(mutate(&p, Action::Apply, &fake).unwrap()["created"], true);
        assert_eq!(mutate(&p, Action::Apply, &fake).unwrap()["created"], false);
        fs::write(&p.file, "operator modification").unwrap();
        for action in [Action::Apply, Action::Start, Action::Stop, Action::Remove] {
            assert!(mutate(&p, action, &fake).is_err());
        }
        assert_eq!(fs::read_to_string(p.file).unwrap(), "operator modification");
        assert!(fake.calls.borrow().is_empty());
    }
    #[test]
    fn create_never_overwrites_existing_file() {
        let (_temp, p) = fixture(Platform::Macos);
        fs::write(&p.file, "unrelated").unwrap();
        assert!(create(&p.file, b"replacement").is_err());
        assert_eq!(fs::read_to_string(&p.file).unwrap(), "unrelated");
    }
    #[test]
    fn launchd_remove_is_idempotent_and_preserves_operational_data() {
        let (_temp, p) = fixture(Platform::Macos);
        install(&p).unwrap();
        let data = p.file.parent().unwrap().join("state.sqlite");
        fs::write(&data, "untouched").unwrap();
        let fake = Fake::new(&[(true, "501"), (false, ""), (true, "GUI domain available")]);
        mutate(&p, Action::Remove, &fake).unwrap();
        mutate(&p, Action::Remove, &fake).unwrap();
        assert_eq!(fs::read_to_string(data).unwrap(), "untouched");
        assert_eq!(fake.calls.borrow().len(), 3);
    }
    #[test]
    fn unrelated_loaded_launchd_job_is_preserved() {
        let (_temp, p) = fixture(Platform::Macos);
        install(&p).unwrap();
        let fake = Fake::new(&[(true, "501"), (true, "path = /someone/else.plist")]);
        assert!(control(&p, Action::Remove, &fake).is_err());
        assert!(p.file.exists());
        assert_eq!(fake.calls.borrow().len(), 2);
    }
    #[test]
    fn windows_unrelated_task_is_never_adopted() {
        let (_temp, p) = fixture(Platform::Windows);
        install(&p).unwrap();
        let fake = Fake::new(&[(true, "unrelated task XML")]);
        assert!(control(&p, Action::Start, &fake).is_err());
        assert_eq!(fake.calls.borrow().len(), 1);
    }
    #[test]
    fn windows_failed_start_rolls_back_only_new_task() {
        let (_temp, p) = fixture(Platform::Windows);
        install(&p).unwrap();
        let fake = Fake::new(&[
            (false, ""),
            (true, "created"),
            (true, "owned XML"),
            (false, "run failed"),
            (true, "owned XML"),
            (true, "deleted"),
        ]);
        assert!(control(&p, Action::Start, &fake).is_err());
        assert!(!task_snapshot(&p).exists());
        assert!(intact(&p).unwrap());
        let calls = fake.calls.borrow();
        assert_eq!(calls[1][1], "/Create");
        assert!(!calls[1].contains(&"/F".into()));
        assert_eq!(calls.last().unwrap()[1], "/Delete");
    }
    #[test]
    fn windows_failed_start_retains_existing_task_and_receipt() {
        let (_temp, p) = fixture(Platform::Windows);
        install(&p).unwrap();
        create(&task_snapshot(&p), b"owned XML").unwrap();
        let fake = Fake::new(&[(true, "owned XML"), (false, "run failed")]);
        assert!(control(&p, Action::Start, &fake).is_err());
        assert!(task_snapshot(&p).exists());
        assert_eq!(fake.calls.borrow().len(), 2);
    }
    #[test]
    fn windows_modified_task_and_missing_task_preserve_receipt() {
        let (_temp, p) = fixture(Platform::Windows);
        install(&p).unwrap();
        create(&task_snapshot(&p), b"original XML").unwrap();
        for reply in [(true, "modified XML"), (false, "")] {
            assert!(control(&p, Action::Remove, &Fake::new(&[reply])).is_err());
            assert!(task_snapshot(&p).exists());
            assert!(p.file.exists());
        }
    }
    #[test]
    fn plan_and_absent_status_do_not_create_state() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let executable = root.join("forge 日本語");
        fs::write(&executable, "fixture").unwrap();
        let state = root.join("not created");
        let mut options = Options {
            action: Action::Plan,
            executable,
            releases_dir: root.clone(),
            definition_dir: None,
            platform: None,
        };
        assert_eq!(execute(&options, &state).unwrap()["status"], "preview");
        options.action = Action::Status;
        assert_eq!(execute(&options, &state).unwrap()["installed"], false);
        assert!(!state.exists());
        let definitions = root.join("separate definitions");
        options.definition_dir = Some(definitions.clone());
        options.action = Action::Plan;
        let preview = execute(&options, &state).unwrap();
        assert!(Path::new(preview["path"].as_str().unwrap()).starts_with(&definitions));
        assert!(!definitions.exists());
        options.action = Action::Apply;
        assert_eq!(execute(&options, &state).unwrap()["created"], true);
        assert!(
            !state.exists(),
            "definition installation must not create operational state"
        );
        assert_eq!(execute(&options, &state).unwrap()["created"], false);
        options.definition_dir = Some(PathBuf::from("relative"));
        assert!(execute(&options, &state).is_err());
        options.definition_dir = None;
        assert!(execute(&options, Path::new("relative")).is_err());
        fs::create_dir(root.join(".git")).unwrap();
        assert!(execute(&options, &state).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlinked_destinations_and_ancestors_are_refused() {
        use std::os::unix::fs::symlink;
        let (_temp, p) = fixture(Platform::Macos);
        let root = p.file.parent().unwrap();
        let other = root.join("other");
        fs::write(&other, "unrelated").unwrap();
        symlink(&other, &p.file).unwrap();
        assert!(install(&p).is_err());
        assert_eq!(fs::read_to_string(other).unwrap(), "unrelated");
        let link = root.join("redirect");
        symlink(root, &link).unwrap();
        assert!(safe_path(&link.join("new")).is_err());
        assert!(safe_path(&root.join("newline\n")).is_err());
    }
    #[test]
    fn initializing_or_corrupt_database_is_explicitly_unavailable() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let db = root.join("state.sqlite");
        for bytes in [b"".as_slice(), b"not a database".as_slice()] {
            fs::write(&db, bytes).unwrap();
            let value = status_detail(json!({"status":"passed"}), root).unwrap();
            assert_eq!(value["runtime_state"], "unavailable");
            assert_eq!(value["heartbeat_stale"], true);
            assert!(value["daemon_error"].is_string());
            assert_eq!(fs::read(&db).unwrap(), bytes);
        }
    }
    #[test]
    fn logs_rotate_three_bounded_files_and_redact_before_storage() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let logs = Logs::new(&root).unwrap();
        for _ in 0..6 {
            logs.write("stderr", &vec![b'x'; 800_000]).unwrap();
        }
        logs.write("stdout", b"authorization: Bearer private\npublic")
            .unwrap();
        let dir = root.join("service-logs");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 4);
        for file in fs::read_dir(&dir).unwrap() {
            assert!(file.unwrap().metadata().unwrap().len() <= 1024 * 1024);
        }
        let output = fs::read_to_string(dir.join("stdout.log")).unwrap();
        assert!(!output.contains("private"));
        assert!(output.contains("public"));
    }
    #[test]
    fn linux_dropins_or_unrelated_loaded_unit_are_preserved() {
        let (_temp, p) = fixture(Platform::Linux);
        install(&p).unwrap();
        for inventory in [
            "FragmentPath=/unrelated.service\nDropInPaths=",
            "FragmentPath=\nDropInPaths=/override.conf",
        ] {
            let mut fake = Fake::new(&[(true, inventory)]);
            fake.config = Some(p.file.parent().unwrap().join("config"));
            assert!(control(&p, Action::Start, &fake).is_err());
            assert_eq!(fake.calls.borrow().len(), 1);
            assert!(intact(&p).unwrap());
        }
    }
    #[test]
    fn linux_failed_start_rolls_back_without_enable() {
        let (_temp, p) = fixture(Platform::Linux);
        install(&p).unwrap();
        let mut fake = Fake::new(&[
            (false, "LoadState=not-found\nFragmentPath=\nDropInPaths="),
            (true, "linked"),
            (true, "reloaded"),
            (false, "failed start"),
            (true, "stopped"),
            (true, "reloaded"),
        ]);
        fake.config = Some(p.file.parent().unwrap().join("config"));
        assert!(control(&p, Action::Start, &fake).is_err());
        assert!(intact(&p).unwrap());
        let calls = fake.calls.borrow();
        assert_eq!(calls.len(), 6);
        assert_eq!(calls[4][2], "stop");
        assert!(!calls.iter().flatten().any(|arg| arg == "enable"));
    }
    #[test]
    fn macos_partial_bootstrap_is_rolled_back() {
        let (_temp, p) = fixture(Platform::Macos);
        install(&p).unwrap();
        let loaded = format!("path = {}", p.file.display());
        let fake = Fake::new(&[
            (true, "501"),
            (false, ""),
            (true, "GUI domain available"),
            (false, "partial bootstrap"),
            (true, &loaded),
            (true, "unloaded"),
        ]);
        assert!(control(&p, Action::Start, &fake).is_err());
        assert_eq!(fake.calls.borrow().last().unwrap()[1], "bootout");
        assert!(intact(&p).unwrap());
    }
}
