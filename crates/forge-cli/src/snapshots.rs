//! Recovery artifacts are integrity-checked state copies, never fresh task acceptance.
use crate::installation::{hash, read_json, safe_path, sync_dir, valid_hash, write_json};
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use forge_domain::{StateAccess, Store};
use fs2::FileExt;
use rusqlite::{Connection, OpenFlags, backup::Backup};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Read;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Component, Path, PathBuf},
    time::Duration,
};

const DB: &str = "state.sqlite";
const MANIFEST: &str = "snapshot.json";
#[derive(Subcommand)]
pub enum Command {
    Snapshot {
        #[arg(long)]
        output: PathBuf,
    },
    Restore {
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        target_state_dir: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    snapshot_schema: u32,
    state_schema: u32,
    workbench_state_schema: Option<u32>,
    tool_version: String,
    minimum_reader_version: String,
    original_location: PathBuf,
    created_unix_ms: u64,
    database: String,
    files: BTreeMap<String, String>,
    manifest_references: Vec<Value>,
    acceptance: String,
}
pub fn dispatch(command: &Command, state: &Path) -> Result<Value> {
    match command {
        Command::Snapshot { output } => snapshot(state, output),
        Command::Restore {
            snapshot,
            target_state_dir,
            apply,
            ..
        } => restore(snapshot, target_state_dir, *apply),
    }
}
fn connection(db: &Path, write: bool) -> Result<Connection> {
    let c = Connection::open_with_flags(
        db,
        if write {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        },
    )?;
    c.busy_timeout(Duration::ZERO)?;
    Ok(c)
}
fn schema(c: &Connection) -> Result<u32> {
    let v = c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))?;
    ensure!(
        v == 1,
        "unsupported state schema; never open migrated state using an older tool"
    );
    Ok(v)
}
fn integrity(c: &Connection) -> Result<()> {
    let v: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    ensure!(v == "ok", "SQLite integrity check failed");
    Ok(())
}
fn idle(c: &Connection) -> Result<Vec<Value>> {
    let mut references = Vec::new();
    let mut statement = c.prepare("SELECT id, body FROM runs ORDER BY id")?;
    for row in statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (id, body) = row?;
        let v: Value = serde_json::from_str(&body)?;
        ensure!(
            v["status"] != "running"
                && !v["tasks"]
                    .as_object()
                    .context("invalid run tasks")?
                    .values()
                    .any(|t| t["status"] == "running" || t["pid"].as_u64().is_some()),
            "active or unreconciled native tasks block snapshot"
        );
        references.push(json!({"release_id":id,"manifest_hash":v["manifest_hash"],"repos":v["effective_manifest"]["repos"]}));
    }
    for table in ["metadata", "token_calls"] {
        let exists: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [table],
            |r| r.get(0),
        )?;
        if !exists {
            continue;
        }
        let mut statement = c.prepare(&format!("SELECT body FROM {table}"))?;
        for body in statement.query_map([], |r| r.get::<_, String>(0))? {
            let v: Value = serde_json::from_str(&body?)?;
            ensure!(
                v["status"] != "running" && v["outcome"] != "running",
                "active or unreconciled native calls block snapshot"
            );
        }
    }
    Ok(references)
}
fn inventory(root: &Path, exclude_db: bool) -> Result<BTreeMap<String, String>> {
    fn walk(
        root: &Path,
        path: &Path,
        skip: bool,
        out: &mut BTreeMap<String, String>,
        total: &mut u64,
    ) -> Result<()> {
        for e in fs::read_dir(path)? {
            let p = e?.path();
            let m = fs::symlink_metadata(&p)?;
            ensure!(
                !m.file_type().is_symlink(),
                "state/snapshot symlink refused"
            );
            if m.is_dir() {
                walk(root, &p, skip, out, total)?;
                continue;
            }
            ensure!(m.is_file(), "state contains unsupported special file");
            let relative = p
                .strip_prefix(root)?
                .to_str()
                .context("state filenames must be UTF-8")?
                .replace('\\', "/");
            if skip
                && [
                    DB,
                    "state.sqlite-wal",
                    "state.sqlite-shm",
                    "state.sqlite-journal",
                    "workbench.sqlite",
                    "workbench.sqlite-wal",
                    "workbench.sqlite-shm",
                    "workbench.sqlite-journal",
                ]
                .contains(&relative.as_str())
            {
                continue;
            }
            ensure!(
                relative != MANIFEST || !skip,
                "reserved snapshot manifest filename in source state"
            );
            if !skip && relative == MANIFEST {
                continue;
            }
            if skip {
                let mut header = [0; 16];
                if File::open(&p)?.read(&mut header)? == 16 {
                    ensure!(
                        header != *b"SQLite format 3\0",
                        "unrecognized SQLite state database; explicit backup support required"
                    );
                }
            }
            *total = total.checked_add(m.len()).context("state size overflow")?;
            ensure!(
                *total <= 4 * 1024 * 1024 * 1024 && out.len() < 100_000,
                "snapshot exceeds local recovery bounds"
            );
            out.insert(relative, hash(&p)?);
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, root, exclude_db, &mut out, &mut 0)?;
    Ok(out)
}
fn copy_files(from: &Path, to: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    for (name, expected) in files {
        let p = Path::new(name);
        ensure!(
            !name.is_empty()
                && p.components().all(|c| matches!(c, Component::Normal(_)))
                && !name.contains('\\'),
            "invalid snapshot file path"
        );
        let source = safe_path(&from.join(p))?;
        let target = safe_path(&to.join(p))?;
        fs::create_dir_all(target.parent().unwrap())?;
        fs::copy(source, &target)?;
        File::open(&target)?.sync_all()?;
        ensure!(
            hash(&target)? == *expected,
            "artifact changed during recovery copy"
        );
    }
    Ok(())
}
fn snapshot(state: &Path, output: &Path) -> Result<Value> {
    let state = safe_path(state)?;
    let output = safe_path(output)?;
    ensure!(
        state.is_dir() && !output.starts_with(&state) && !state.starts_with(&output),
        "snapshot must be outside source state"
    );
    ensure!(!output.exists(), "snapshot output must be fresh");
    let db = safe_path(&state.join(DB))?;
    let _lease = StateAccess::acquire(&db, true, false)?;
    // SQLite's own write reservation excludes non-Forge writers too. Read-only source
    // connection feeds the online backup, including committed WAL pages.
    let owner = connection(&db, true)?;
    owner.execute_batch("BEGIN IMMEDIATE")?;
    schema(&owner)?;
    integrity(&owner)?;
    let refs = idle(&owner)?;
    // A daemon owns artifact/log writes between its short database connections.
    let mut _supervisors = Vec::new();
    for name in ["daemon.lock", "service-host.lock"] {
        let path = safe_path(&state.join(name))?;
        if path.exists() {
            ensure!(path.is_file(), "invalid supervisor lock");
            let f = File::open(path)?;
            f.try_lock_exclusive()
                .context("stop the per-user supervisor before snapshot")?;
            _supervisors.push(f);
        }
    }
    let workbench_path = safe_path(&state.join("workbench.sqlite"))?;
    let workbench_owner = if workbench_path.exists() {
        let c = connection(&workbench_path, true)?;
        c.execute_batch("BEGIN IMMEDIATE")?;
        ensure!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? == 0,
            "unsupported workbench schema"
        );
        integrity(&c)?;
        Some(c)
    } else {
        None
    };
    let artifacts = inventory(&state, true)?;
    let parent = output.parent().context("snapshot parent required")?;
    ensure!(parent.is_dir(), "snapshot parent must exist");
    let stage = tempfile::tempdir_in(parent)?;
    let stage_path = stage.path().canonicalize()?;
    let source = connection(&db, false)?;
    let mut copied = Connection::open(stage_path.join(DB))?;
    Backup::new(&source, &mut copied)?.run_to_completion(64, Duration::from_millis(10), None)?;
    // Recovery artifacts are sealed standalone databases. Leaving WAL mode in the
    // copied header would let a read-only preview create fresh -wal/-shm sidecars.
    copied.execute_batch("PRAGMA journal_mode=DELETE;")?;
    schema(&copied)?;
    integrity(&copied)?;
    drop(copied);
    drop(source);
    if workbench_owner.is_some() {
        let source = connection(&workbench_path, false)?;
        let mut copy = Connection::open(stage_path.join("workbench.sqlite"))?;
        Backup::new(&source, &mut copy)?.run_to_completion(64, Duration::from_millis(10), None)?;
        copy.execute_batch("PRAGMA journal_mode=DELETE;")?;
        integrity(&copy)?;
    }
    copy_files(&state, &stage_path, &artifacts)?;
    ensure!(
        inventory(&state, true)? == artifacts,
        "state artifacts changed during snapshot"
    );
    let files = inventory(&stage_path, false)?;
    let metadata = Snapshot {
        snapshot_schema: 1,
        state_schema: 1,
        workbench_state_schema: workbench_owner.as_ref().map(|_| 0),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        minimum_reader_version: "0.3.0".into(),
        original_location: state.clone(),
        created_unix_ms: crate::duration::now_ms(),
        database: DB.into(),
        files,
        manifest_references: refs,
        acceptance: "historical checkpoint only; no new task or revision acceptance".into(),
    };
    write_json(&stage_path.join(MANIFEST), &metadata)?;
    // Exercise a separate restored copy and the real Store reader before publishing.
    let probe = tempfile::tempdir_in(parent)?;
    let probe_path = probe.path().canonicalize()?;
    copy_files(&stage_path, &probe_path, &metadata.files)?;
    verify_database(&probe_path, &metadata)?;
    let _ = Store::open(&probe_path.join(DB), true)?;
    publish_directory(&stage_path, &output)?;
    owner.execute_batch("ROLLBACK")?;
    if let Some(c) = workbench_owner {
        c.execute_batch("ROLLBACK")?;
    }
    Ok(
        json!({"schema_version":1,"status":"snapshotted","snapshot":output,"state_dir":state,"state_schema":1,"files":metadata.files,"restore_probe":"passed","consistency":"SQLite backup API including WAL; exclusive Forge lease and SQLite writer reservation","acceptance":"historical only"}),
    )
}
fn verify_database(root: &Path, m: &Snapshot) -> Result<()> {
    let c = sealed_database(&root.join(DB))?;
    ensure!(
        schema(&c)? == m.state_schema,
        "snapshot schema differs from database"
    );
    integrity(&c)?;
    idle(&c)?;
    let workbench = root.join("workbench.sqlite");
    ensure!(
        workbench.exists() == m.workbench_state_schema.is_some(),
        "workbench schema inventory mismatch"
    );
    if workbench.exists() {
        ensure!(
            m.workbench_state_schema == Some(0),
            "unsupported workbench schema"
        );
        let c = sealed_database(&workbench)?;
        ensure!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))? == 0,
            "unsupported workbench schema"
        );
        integrity(&c)?;
    }
    Ok(())
}
fn sealed_database(path: &Path) -> Result<Connection> {
    let path = safe_path(path)?;
    let mut header = [0; 20];
    File::open(&path)?.read_exact(&mut header)?;
    ensure!(
        &header[..16] == b"SQLite format 3\0" && header[18..20] == [1, 1],
        "snapshot database must be sealed without WAL/journals"
    );
    connection(&path, false)
}
fn verify(root: &Path) -> Result<Snapshot> {
    let m: Snapshot = read_json(&root.join(MANIFEST))?;
    ensure!(
        m.snapshot_schema == 1
            && m.state_schema == 1
            && m.database == DB
            && m.minimum_reader_version == "0.3.0",
        "unsupported snapshot/reader schema"
    );
    ensure!(
        m.files.contains_key(DB) && m.files.values().all(|h| valid_hash(h)),
        "invalid snapshot inventory"
    );
    ensure!(
        inventory(root, false)? == m.files,
        "snapshot artifact hash/inventory mismatch"
    );
    ensure!(
        !m.files.keys().any(|f| [
            "state.sqlite-wal",
            "state.sqlite-shm",
            "state.sqlite-journal",
            "workbench.sqlite-wal",
            "workbench.sqlite-shm",
            "workbench.sqlite-journal",
            ".forge-restore-incomplete"
        ]
        .contains(&f.as_str())),
        "unexpected recovery journal"
    );
    verify_database(root, &m)?;
    Ok(m)
}
fn restore(snapshot: &Path, target: &Path, apply: bool) -> Result<Value> {
    let snapshot = safe_path(snapshot)?;
    let target = safe_path(target)?;
    ensure!(
        !target.exists(),
        "restore target must be fresh; existing operational state is never overwritten"
    );
    ensure!(
        !target.starts_with(&snapshot) && !snapshot.starts_with(&target),
        "restore target must be separate from snapshot"
    );
    let m = verify(&snapshot)?;
    let value = json!({"schema_version":1,"status":if apply {"restored"}else{"preview"},"snapshot":snapshot,"target_state_dir":target,"state_schema":m.state_schema,"files":m.files,"tool_version":m.tool_version,"original_location":m.original_location,"acceptance":"historical checkpoint; reconcile changed inputs before execution"});
    if !apply {
        return Ok(value);
    }
    let parent = target.parent().context("restore target parent required")?;
    ensure!(parent.is_dir(), "restore target parent must exist");
    let stage = tempfile::tempdir_in(parent)?;
    let stage_path = stage.path().canonicalize()?;
    copy_files(&snapshot, &stage_path, &m.files)?;
    verify_database(&stage_path, &m)?;
    ensure!(
        inventory(&stage_path, false)? == m.files,
        "restored artifact mismatch"
    );
    let _ = Store::open(&stage_path.join(DB), true)?;
    // Inventory changes during the copy are not silently accepted.
    let again = verify(&snapshot)?;
    ensure!(again.files == m.files, "snapshot changed during restore");
    publish_directory(&stage_path, &target)?;
    Ok(value)
}
/// Publish a fully verified directory with no replacement, including concurrent empty targets.
fn publish_directory(from: &Path, to: &Path) -> Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let a = std::ffi::CString::new(from.as_os_str().as_bytes())?;
        let b = std::ffi::CString::new(to.as_os_str().as_bytes())?;
        #[cfg(target_os = "macos")]
        let result = unsafe { libc::renamex_np(a.as_ptr(), b.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                a.as_ptr(),
                libc::AT_FDCWD,
                b.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        ensure!(
            result == 0,
            "atomic fresh-target publish failed: {}",
            std::io::Error::last_os_error()
        );
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        ensure!(!to.exists(), "target already exists");
        fs::rename(from, to)?;
    }
    sync_dir(to.parent().unwrap())?;
    Ok(())
}
