//! Disposable CLI regressions; no provider calls, remote state or coordinator fixtures.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Command};
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
    manifest: PathBuf,
    state: PathBuf,
    gate: PathBuf,
    count: PathBuf,
}
impl Fixture {
    fn new(cache: bool, human: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let repo = root.join("project");
        std::fs::create_dir(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&repo)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(repo.join("preserved"), "keep").unwrap();
        std::fs::write(repo.join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(repo.join("ignored.txt"), "first").unwrap();
        let gate = root.join("gate.py");
        let count = root.join("counter");
        std::fs::write(&gate,format!("from pathlib import Path\np=Path({:?})\np.write_text(str(int(p.read_text())+1) if p.exists() else '1')\n",count.to_str().unwrap())).unwrap();
        let mut acceptance = vec![json!({"kind":"gate","id":"proof"})];
        if human {
            acceptance.push(json!({"kind":"human","id":"review"}));
        }
        let m = json!({"schema_version":1,"release_id":"m06","repos":{"app":{"path":repo}},"tasks":[{"id":"work","repo":"app","description":"Local test","executor":{"kind":"command","argv":["python3","-c","pass"]},"max_attempts":3,"acceptance":acceptance}],"gates":[{"id":"proof","argv":["python3",gate],"inputs":{gate.to_str().unwrap():forge_domain::digest(&std::fs::read(&gate).unwrap())},"cache":{"enabled":cache,"ttl_seconds":300}}]});
        let manifest = root.join("release.yaml");
        std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        let state = root.join("state");
        Self {
            _dir: dir,
            root,
            repo,
            manifest,
            state,
            gate,
            count,
        }
    }
    fn edit(&self, f: impl FnOnce(&mut Value)) {
        let mut m: Value = serde_json::from_slice(&std::fs::read(&self.manifest).unwrap()).unwrap();
        f(&mut m);
        std::fs::write(&self.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    }
    fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"));
        c.arg("--json")
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .arg(&self.manifest)
            .current_dir(&self.root);
        c
    }
    fn call(&self, args: &[&str]) -> (bool, Value) {
        let out = self.cmd(args).output().unwrap();
        (
            out.status.success(),
            serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
                panic!(
                    "{} {}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                )
            }),
        )
    }
    fn counter(&self) -> u32 {
        std::fs::read_to_string(&self.count)
            .unwrap()
            .parse()
            .unwrap()
    }
}
#[test]
fn budget_stops_before_new_client_calls() {
    let f = Fixture::new(false, false);
    let bin = f.root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let shim = bin.join("codex");
    let marker = f.root.join("spawned");
    std::fs::write(
        &shim,
        format!(
            "#!/usr/bin/env python3\nfrom pathlib import Path\nPath({:?}).write_text('spawned')\n",
            marker.to_str().unwrap()
        ),
    )
    .unwrap();
    std::fs::set_permissions(shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    f.edit(|m| {
        m["budget"] = json!({"max_tokens":0,"actions":"off"});
        m["tasks"][0]["executor"] = json!({"kind":"codex","permission":"read_only"});
    });
    let (ok, v) = f.call(&["budget", "inspect"]);
    assert!(ok, "{v}");
    assert!(v["available_balance"].is_null());
    assert!(!f.state.exists());
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let out = f
        .cmd(&["release", "run"])
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["run"]["status"], "blocked");
    assert!(!marker.exists());
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 0);
}
#[test]
fn cache_reuses_only_opted_in_identical_gates() {
    let f = Fixture::new(true, false);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    assert_eq!(f.counter(), 1);
    let (_, v) = f.call(&["report"]);
    let events = v["events"].as_array().unwrap();
    assert!(
        events.iter().any(|e| e["kind"] == "gate_cache"
            && e["body"]["result"] == "hit"
            && e["body"]["reason"] == "identical_verified_inputs"),
        "{events:?}"
    );
    let f = Fixture::new(false, false);
    assert!(f.call(&["release", "run"]).0);
    assert_eq!(f.counter(), 2);
}
#[test]
fn cache_invalidates_changed_inputs_and_evaluators() {
    let f = Fixture::new(true, true);
    assert!(!f.call(&["release", "run"]).0);
    assert_eq!(f.counter(), 1);
    std::fs::write(f.repo.join("ignored.txt"), "changed ignored artifact").unwrap();
    assert!(!f.call(&["resume"]).0);
    assert_eq!(f.counter(), 2);
    std::fs::set_permissions(&f.gate, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!f.call(&["resume"]).0);
    assert_eq!(f.counter(), 3);
    std::fs::write(&f.gate, "# changed evaluator\n").unwrap();
    let (ok, v) = f.call(&["resume"]);
    assert!(!ok);
    assert!(v["error"].as_str().unwrap().contains("digest mismatch"));
}
#[test]
fn cache_never_transfers_human_approval() {
    let f = Fixture::new(true, true);
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let fp = v["run"]["tasks"]["work"]["revision"]["fingerprint"]
        .as_str()
        .unwrap();
    assert!(
        f.call(&[
            "release",
            "approve",
            "--task",
            "work",
            "--gate",
            "review",
            "--reviewer",
            "Fixture",
            "--fingerprint",
            fp
        ])
        .0
    );
    std::fs::write(f.repo.join("preserved"), "changed").unwrap();
    assert!(!f.call(&["resume"]).0);
    let (_, v) = f.call(&["resume", "--reconcile", "source changed"]);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    assert_ne!(v["run"]["tasks"]["work"]["revision"]["fingerprint"], fp);
}
#[test]
fn cache_reports_provenance_and_corruption() {
    let f = Fixture::new(true, true);
    assert!(!f.call(&["release", "run"]).0);
    let db = rusqlite::Connection::open(f.state.join("state.sqlite")).unwrap();
    db.execute(
        "UPDATE metadata SET body='{}' WHERE key LIKE 'gate-cache:%'",
        [],
    )
    .unwrap();
    drop(db);
    let (_, v) = f.call(&["resume"]);
    assert_eq!(f.counter(), 2);
    let (_, report) = f.call(&["report"]);
    assert!(report.to_string().contains("corrupt_cache"), "{v}");
}
#[test]
fn cache_expires_and_environment_changes_invalidate() {
    let f = Fixture::new(true, true);
    assert!(!f.call(&["release", "run"]).0);
    let out = f
        .cmd(&["resume"])
        .env("FORGE_CACHE_TEST_ENV", "different")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(f.counter(), 2);
    // TTL is part of the manifest policy: changing it needs reconciliation and a fresh evaluation.
    f.edit(|m| m["gates"][0]["cache"]["ttl_seconds"] = json!(1));
    let (_, v) = f.call(&["resume", "--reconcile", "short TTL policy"]);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let count = f.counter();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    f.call(&["resume"]);
    assert_eq!(f.counter(), count + 1);
}
#[test]
fn time_and_attempt_budgets_apply_to_commands_without_inventing_tokens() {
    let f = Fixture::new(false, false);
    f.edit(|m| m["budget"] = json!({"max_active_seconds":0}));
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 0);
    assert!(!f.count.exists());
    let f = Fixture::new(false, false);
    f.edit(|m| {
        m["budget"] = json!({"max_attempts":1});
        m["tasks"][0]["executor"]["argv"] = json!(["python3", "-c", "raise SystemExit(1)"]);
    });
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 1);
    assert!(v["token_usage"]["total_tokens"].is_null());
    let (_, v) = f.call(&["resume", "--reconcile", "attempt limit retained"]);
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 0);
}

#[test]
fn failed_planning_calls_consume_budget_even_without_a_candidate() {
    let f = Fixture::new(false, false);
    f.edit(|m| m["budget"] = json!({"max_tokens":18}));
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workbench-native.py");
    let plan = |output: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"));
        cmd.arg("--json")
            .arg("--state-dir")
            .arg(&f.state)
            .args(["plan", "propose", "--root"])
            .arg(&f.repo)
            .args(["--request", "INVALID_PLAN", "--ops-root"])
            .arg(f.root.join("candidates"))
            .arg("--output")
            .arg(f.root.join(output))
            .arg("--app-server")
            .arg(&fixture)
            .arg("--manifest")
            .arg(&f.manifest);
        cmd.output().unwrap()
    };
    assert!(!plan("first.json").status.success());
    assert!(!f.root.join("first.json").exists());
    let (_, v) = f.call(&["budget", "inspect"]);
    assert_eq!(v["observed"]["tokens"]["total_tokens"], 18);
    assert_eq!(v["observed"]["attempts"], 1);
    let out = plan("second.json");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .contains("token budget exhausted")
    );
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let calls = store.token_calls("m06", 100, 0).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["outcome"], "failed");
}
#[test]
fn missing_native_usage_blocks_a_transient_retry_under_budget() {
    let f = Fixture::new(false, false);
    let bin = f.root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let shim = bin.join("codex");
    let marker = f.root.join("count-native");
    let code = format!(
        "#!/usr/bin/env python3\nimport json,sys\nfrom pathlib import Path\np=Path({:?})\np.write_text(str(int(p.read_text())+1) if p.exists() else '1')\nprint(json.dumps({{'type':'turn.failed','error':{{'code':'server_error'}}}}))\nsys.exit(1)\n",
        marker.to_str().unwrap()
    );
    std::fs::write(&shim, code).unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    f.edit(|m| {
        m["budget"] = json!({"max_tokens":1000});
        m["tasks"][0]["executor"] = json!({"kind":"codex","permission":"read_only"});
    });
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let out = f
        .cmd(&["release", "run"])
        .env("PATH", std::env::join_paths(paths).unwrap())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["run"]["status"], "blocked");
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "1");
    assert!(v["token_usage"]["total_tokens"].is_null());
    assert!(v.to_string().contains("token usage unknown"));
}
