//! Host-local native file/SQLite regressions. These never establish remote/platform acceptance.
use forge_domain::{Store, digest};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    state: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        Self {
            state: base.join("state"),
            base,
            _temp: t,
        }
    }
    fn call(&self, args: &[&str], ok: bool) -> Value {
        let o = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
            .args(["--json", "--state-dir"])
            .arg(&self.state)
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            o.status.success(),
            ok,
            "args={args:?}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap()
    }
    fn initialize(&self) {
        let s = Store::open(&self.state.join("state.sqlite"), false).unwrap();
        s.set_metadata("checkpoint", &json!({"history":"retained"}))
            .unwrap();
        fs::create_dir(self.state.join("logs")).unwrap();
        fs::write(self.state.join("logs/task.json"), b"historical artifact").unwrap();
    }
    fn install(&self, artifact: &Path, prefix: &Path, apply: bool, ok: bool) -> Value {
        self.call(
            &[
                "installation",
                "update",
                "--artifact",
                artifact.to_str().unwrap(),
                "--sha256",
                &digest(&fs::read(artifact).unwrap()),
                "--prefix",
                prefix.to_str().unwrap(),
                if apply { "--apply" } else { "--dry-run" },
            ],
            ok,
        )
    }
    fn snapshot(&self, output: &Path, ok: bool) -> Value {
        self.call(
            &["state", "snapshot", "--output", output.to_str().unwrap()],
            ok,
        )
    }
    fn restore(&self, snapshot: &Path, target: &Path, apply: bool, ok: bool) -> Value {
        self.call(
            &[
                "state",
                "restore",
                "--snapshot",
                snapshot.to_str().unwrap(),
                "--target-state-dir",
                target.to_str().unwrap(),
                if apply { "--apply" } else { "--dry-run" },
            ],
            ok,
        )
    }
}
fn executable() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_illumitech-forge"))
}
fn name() -> &'static str {
    if cfg!(windows) {
        "illumitech-forge.exe"
    } else {
        "illumitech-forge"
    }
}
#[test]
fn update_verifies_source_and_checksums() {
    let f = Fixture::new();
    let prefix = f.base.join("installation");
    let artifact = executable();
    let doctor = f.call(&["installation", "doctor"], true);
    assert!(doctor["path_order"].is_array());
    assert!(!f.state.exists());
    let preview = f.install(&artifact, &prefix, false, true);
    assert_eq!(preview["status"], "preview");
    assert!(!prefix.exists());
    f.install(&artifact, &prefix, true, true);
    let target = prefix.join("bin").join(name());
    let before = digest(&fs::read(&target).unwrap());
    f.call(
        &[
            "installation",
            "update",
            "--artifact",
            artifact.to_str().unwrap(),
            "--sha256",
            &"0".repeat(64),
            "--prefix",
            prefix.to_str().unwrap(),
            "--apply",
        ],
        false,
    );
    assert_eq!(digest(&fs::read(&target).unwrap()), before);
    assert_eq!(
        f.install(&artifact, &prefix, true, true)["status"],
        "unchanged"
    );
    let unknown = f.base.join("unknown");
    fs::create_dir_all(unknown.join("bin")).unwrap();
    fs::write(unknown.join("bin").join(name()), "unrelated product").unwrap();
    f.install(&artifact, &unknown, true, false);
    assert_eq!(
        fs::read(unknown.join("bin").join(name())).unwrap(),
        b"unrelated product"
    );
    let foreign = f.base.join("foreign");
    fs::write(&foreign, [0; 64]).unwrap();
    f.install(&foreign, &prefix, true, false);
    assert_eq!(digest(&fs::read(&target).unwrap()), before);
}
#[test]
fn path_conflicts_and_interrupted_update_are_recoverable() {
    let f = Fixture::new();
    let prefix = f.base.join("prefix");
    let artifact = executable();
    f.install(&artifact, &prefix, true, true);
    let receipt_path = prefix.join(".forge-current.json");
    let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    // Crash after atomic binary replacement, before first receipt publication.
    fs::remove_file(&receipt_path).unwrap();
    fs::write(
        prefix.join(".forge-update.json"),
        json!({"previous":null,"next":receipt}).to_string(),
    )
    .unwrap();
    assert_eq!(
        f.install(&artifact, &prefix, false, true)["recovery"],
        "finalize_receipt"
    );
    assert!(!receipt_path.exists());
    f.install(&artifact, &prefix, true, true);
    assert!(receipt_path.exists());
    // Crash after receipt publication but before removing the journal is also idempotent.
    fs::write(
        prefix.join(".forge-update.json"),
        json!({"previous":null,"next":receipt}).to_string(),
    )
    .unwrap();
    f.install(&artifact, &prefix, true, true);
    assert!(!prefix.join(".forge-update.json").exists());
    // Crash after generation/journal creation but before the first binary swap.
    fs::remove_file(&receipt_path).unwrap();
    fs::remove_file(prefix.join("bin").join(name())).unwrap();
    fs::write(
        prefix.join(".forge-update.json"),
        json!({"previous":null,"next":receipt}).to_string(),
    )
    .unwrap();
    assert_eq!(
        f.install(&artifact, &prefix, false, true)["recovery"],
        "retry_atomic_swap"
    );
    assert!(!prefix.join("bin").join(name()).exists());
    f.install(&artifact, &prefix, true, true);
    let backup = prefix
        .join(".forge-installations")
        .join(receipt["sha256"].as_str().unwrap())
        .join(name());
    fs::write(&backup, "tampered backup").unwrap();
    let before = fs::read(prefix.join("bin").join(name())).unwrap();
    f.install(&artifact, &prefix, true, false);
    assert_eq!(fs::read(prefix.join("bin").join(name())).unwrap(), before);
}
#[test]
fn consistent_wal_snapshot_and_fresh_restore() {
    let f = Fixture::new();
    f.initialize();
    // Keep raw SQLite writer connections open: committed rows remain only in WAL.
    let raw = Connection::open(f.state.join("state.sqlite")).unwrap();
    raw.execute_batch("PRAGMA wal_autocheckpoint=0; INSERT INTO metadata VALUES('wal-only','42');")
        .unwrap();
    let wb = Connection::open(f.state.join("workbench.sqlite")).unwrap();
    wb.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE sessions(id TEXT); INSERT INTO sessions VALUES('historical');").unwrap();
    let output = f.base.join("snapshot");
    let report = f.snapshot(&output, true);
    assert_eq!(report["restore_probe"], "passed");
    let target = f.base.join("restored");
    let inventory_before = fs::read_dir(&output)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    let database_before = fs::read(output.join("state.sqlite")).unwrap();
    f.restore(&output, &target, false, true);
    assert_eq!(
        fs::read_dir(&output)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<std::collections::BTreeSet<_>>(),
        inventory_before
    );
    assert_eq!(
        fs::read(output.join("state.sqlite")).unwrap(),
        database_before
    );
    assert!(!target.exists());
    f.restore(&output, &target, true, true);
    let read = Store::open(&target.join("state.sqlite"), true).unwrap();
    assert_eq!(read.metadata("wal-only").unwrap(), Some(json!(42)));
    assert_eq!(
        fs::read(target.join("logs/task.json")).unwrap(),
        b"historical artifact"
    );
    let c = Connection::open(target.join("workbench.sqlite")).unwrap();
    assert_eq!(
        c.query_row("SELECT id FROM sessions", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "historical"
    );
    f.restore(&output, &target, true, false);
    assert!(
        raw.query_row("SELECT body FROM metadata WHERE key='wal-only'", [], |r| {
            r.get::<_, String>(0)
        })
        .is_ok()
    );
}
#[test]
fn migration_failure_restores_matching_backup() {
    let f = Fixture::new();
    f.initialize();
    let snapshot = f.base.join("snapshot");
    f.snapshot(&snapshot, true);
    // Simulate a future migrator's state. The old executable refuses it before any schema writes.
    let db = f.state.join("state.sqlite");
    Connection::open(&db)
        .unwrap()
        .execute_batch("PRAGMA user_version=99")
        .unwrap();
    assert!(Store::open(&db, false).is_err());
    let before = fs::read(&db).unwrap();
    let target = f.base.join("rollback");
    f.restore(&snapshot, &target, true, true);
    assert!(Store::open(&target.join("state.sqlite"), false).is_ok());
    assert_eq!(fs::read(&db).unwrap(), before);
    let c = Connection::open(snapshot.join("state.sqlite")).unwrap();
    c.execute_batch("PRAGMA user_version=99").unwrap();
    drop(c);
    // Rehashing does not make a newer schema supported.
    let p = snapshot.join("snapshot.json");
    let mut m: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    m["files"]["state.sqlite"] = json!(digest(&fs::read(snapshot.join("state.sqlite")).unwrap()));
    fs::write(p, m.to_string()).unwrap();
    let invalid = f.base.join("invalid");
    f.restore(&snapshot, &invalid, true, false);
    assert!(!invalid.exists());
}
#[test]
fn active_writers_tamper_and_partial_snapshots_fail_closed() {
    let f = Fixture::new();
    f.initialize();
    let output = f.base.join("snapshot");
    let store = Store::open(&f.state.join("state.sqlite"), false).unwrap();
    f.snapshot(&output, false);
    assert!(!output.exists());
    drop(store);
    let raw = Connection::open(f.state.join("state.sqlite")).unwrap();
    raw.execute_batch("BEGIN IMMEDIATE").unwrap();
    f.snapshot(&output, false);
    assert!(!output.exists());
    raw.execute_batch("ROLLBACK").unwrap();
    drop(raw);
    let store = Store::open(&f.state.join("state.sqlite"), false).unwrap();
    store
        .set_metadata("independent-review:test", &json!({"status":"running"}))
        .unwrap();
    drop(store);
    f.snapshot(&output, false);
    assert!(!output.exists());
    let store = Store::open(&f.state.join("state.sqlite"), false).unwrap();
    store.remove_metadata("independent-review:test").unwrap();
    drop(store);
    f.snapshot(&output, true);
    fs::write(output.join("logs/task.json"), "tamper").unwrap();
    let target = f.base.join("restore");
    f.restore(&output, &target, true, false);
    assert!(!target.exists());
    fs::remove_file(output.join("snapshot.json")).unwrap();
    f.restore(&output, &target, false, false);
    assert!(!target.exists());
}
#[cfg(unix)]
#[test]
fn symlink_traversal_and_unknown_targets_are_refused() {
    use std::os::unix::fs::symlink;
    let mut f = Fixture::new();
    f.initialize();
    let linked = f.base.join("link");
    symlink(&f.state, &linked).unwrap();
    let output = f.base.join("snapshot");
    let original = std::mem::replace(&mut f.state, linked.clone());
    f.snapshot(&output, false);
    f.state = original;
    let linked_prefix = f.base.join("prefix");
    symlink(&f.state, &linked_prefix).unwrap();
    f.install(&executable(), &linked_prefix, true, false);
    let artifact_link = f.base.join("artifact");
    symlink(executable(), &artifact_link).unwrap();
    f.install(&artifact_link, &f.base.join("install"), true, false);
    f.snapshot(&output, true);
    fs::remove_file(output.join("logs/task.json")).unwrap();
    symlink(
        f.state.join("logs/task.json"),
        output.join("logs/task.json"),
    )
    .unwrap();
    f.restore(&output, &f.base.join("restore"), true, false);
}

fn native_fixture(base: &Path, id: &str, body: &str) -> PathBuf {
    let source = base.join(format!("{id}.rs"));
    let artifact = base.join(format!("{id}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&source, body).unwrap();
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", id, "-C", "debuginfo=0"])
        .arg(&source)
        .arg("-o")
        .arg(&artifact)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    artifact
}
#[test]
fn downgrade_requires_explicit_opt_in_and_probes_are_bounded() {
    let f = Fixture::new();
    let prefix = f.base.join("install");
    let old = native_fixture(
        &f.base,
        "old_artifact",
        r#"fn main(){println!("illumitech-forge 0.2.0");}"#,
    );
    let new = native_fixture(
        &f.base,
        "new_artifact",
        r#"fn main(){println!("illumitech-forge 0.4.0");}"#,
    );
    f.install(&old, &prefix, true, true);
    f.install(&new, &prefix, true, true);
    let target = prefix.join("bin").join(name());
    let before = fs::read(&target).unwrap();
    let old_backup = prefix
        .join(".forge-installations")
        .join(digest(&fs::read(&old).unwrap()))
        .join(name());
    fs::write(&old_backup, b"tampered previous generation").unwrap();
    f.install(&new, &prefix, true, false);
    assert_eq!(fs::read(&target).unwrap(), before);
    fs::copy(&old, &old_backup).unwrap();
    f.install(&old, &prefix, true, false);
    assert_eq!(fs::read(&target).unwrap(), before);
    f.call(
        &[
            "installation",
            "update",
            "--artifact",
            old.to_str().unwrap(),
            "--sha256",
            &digest(&fs::read(&old).unwrap()),
            "--prefix",
            prefix.to_str().unwrap(),
            "--allow-downgrade",
            "--apply",
        ],
        true,
    );
    assert_eq!(fs::read(&target).unwrap(), fs::read(&old).unwrap());
    assert!(
        prefix
            .join(".forge-installations")
            .join(digest(&fs::read(&new).unwrap()))
            .join(name())
            .is_file()
    );
    assert_eq!(
        fs::read_dir(prefix.join(".forge-update-history"))
            .unwrap()
            .count(),
        3
    );
    let failed = native_fixture(
        &f.base,
        "failed_artifact",
        r#"fn main(){println!("unrelated product");}"#,
    );
    f.install(&failed, &prefix, true, false);
    assert_eq!(fs::read(&target).unwrap(), fs::read(&old).unwrap());
    let hung = native_fixture(
        &f.base,
        "hung_artifact",
        r#"fn main(){std::thread::sleep(std::time::Duration::from_secs(30));}"#,
    );
    let start = std::time::Instant::now();
    f.install(&hung, &prefix, true, false);
    assert!(start.elapsed().as_secs() < 15);
    assert_eq!(fs::read(&target).unwrap(), fs::read(&old).unwrap());
}
#[test]
fn supervisor_ownership_and_explicit_snapshot_scope_are_required() {
    use fs2::FileExt;
    let f = Fixture::new();
    f.initialize();
    let snapshot = f.base.join("snapshot");
    for name in ["daemon.lock", "service-host.lock"] {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(f.state.join(name))
            .unwrap();
        lock.try_lock_exclusive().unwrap();
        f.snapshot(&snapshot, false);
        assert!(!snapshot.exists());
        drop(lock);
    }
    let out = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
        .current_dir(&f.base)
        .args([
            "--json",
            "state",
            "snapshot",
            "--output",
            snapshot.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .contains("explicit --state-dir")
    );
    assert!(!snapshot.exists());
}
#[cfg(unix)]
#[test]
fn doctor_reports_ordered_symlink_conflicts_without_executing_them() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    let first = f.base.join("first");
    let second = f.base.join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let fake = first.join(name());
    let marker = f.base.join("executed");
    fs::write(&fake, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(executable(), second.join(name())).unwrap();
    let path = std::env::join_paths([first, second]).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
        .args(["--json", "installation", "doctor"])
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["competing_binaries"].as_array().unwrap().len(), 2);
    assert_eq!(v["competing_binaries"][0]["path_index"], 0);
    assert_eq!(v["competing_binaries"][1]["symlink"], true);
    assert!(!marker.exists());
    assert!(!f.state.exists());
}
