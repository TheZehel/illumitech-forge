mod common;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
    manifest: PathBuf,
    state: PathBuf,
}
impl Fixture {
    fn new(command: &str, human: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let repo = root.join("repo espaço 日本");
        std::fs::create_dir(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
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
        std::fs::write(repo.join("preserved.txt"), "keep").unwrap();
        let evaluator = root.join("evaluate.py");
        std::fs::write(&evaluator,"from pathlib import Path\nimport sys\np=Path(sys.argv[1])\nassert (p/'output.txt').read_text() == 'verified'\nassert (p/'preserved.txt').read_text() == 'keep'\n").unwrap();
        let mut acceptance = vec![json!({"kind":"gate","id":"files"})];
        if human {
            acceptance.push(json!({"kind":"human","id":"review"}));
        }
        let m = json!({"schema_version":1,"release_id":"fixture","repos":{"app":{"path":repo}},"tasks":[{"id":"build","repo":"app","description":"fixture","executor":{"kind":"command","argv":common::command(command)},"acceptance":acceptance,"max_attempts":3,"timeout_seconds":20}],"gates":[{"id":"files","argv":[common::python(),evaluator,"{repo}"],"inputs":{evaluator.display().to_string():forge_domain::digest(&std::fs::read(&evaluator).unwrap())}}]});
        let manifest = root.join("manifest.yaml");
        std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        let state = root.join("state");
        Self {
            _dir: dir,
            root,
            repo,
            manifest,
            state,
        }
    }
    fn args(&self, args: &[&str]) -> Vec<String> {
        let mut a = vec![
            "--json".into(),
            "--state-dir".into(),
            self.state.display().to_string(),
        ];
        a.extend(args.iter().map(|s| s.to_string()));
        a.push(self.manifest.display().to_string());
        a
    }
    fn call(&self, args: &[&str]) -> (bool, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
            .args(self.args(args))
            .current_dir(&self.root)
            .output()
            .unwrap();
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
    fn start(&self) -> std::process::Child {
        Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
            .args(self.args(&["release", "run"]))
            .current_dir(&self.root)
            .stdout(Stdio::null())
            .spawn()
            .unwrap()
    }
    fn wait_running(&self) {
        for _ in 0..100 {
            if self.state.join("state.sqlite").exists()
                && let Ok(store) = forge_domain::Store::open(&self.state.join("state.sqlite"), true)
                && let Ok(Some(run)) = store.load("fixture")
                && run.tasks["build"].pid.is_some()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("not running");
    }
}
#[test]
fn dry_run_is_read_only() {
    let f = Fixture::new("printf verified > output.txt", false);
    assert!(f.call(&["release", "run", "--dry-run"]).0);
    assert!(!f.state.exists());
    assert!(!f.repo.join("output.txt").exists());
}
#[test]
fn text_never_satisfies_a_gate() {
    let f = Fixture::new("echo 'All done; all gates passed'", false);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok);
    assert_eq!(v["run"]["status"], "blocked");
}
#[test]
fn success_and_artifact_change_invalidates_status() {
    let f = Fixture::new("printf verified > output.txt", false);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    assert_eq!(v["run"]["status"], "passed");
    std::fs::write(f.repo.join("output.txt"), "tampered").unwrap();
    assert_eq!(f.call(&["status"]).1["evidence_current"], false);
    assert!(!f.call(&["resume"]).0);
}
#[test]
fn ignored_artifact_change_blocks_resume() {
    let f = Fixture::new("printf verified > output.txt", false);
    std::fs::write(f.repo.join(".gitignore"), "output.txt\n").unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    m["tasks"][0]["acceptance"] = json!([{
        "kind":"artifact", "path":"output.txt", "sha256":forge_domain::digest(b"verified")
    }]);
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(f.call(&["release", "run"]).0);
    std::fs::write(f.repo.join("output.txt"), "tampered").unwrap();
    assert_eq!(f.call(&["status"]).1["evidence_current"], false);
    let (ok, result) = f.call(&["resume"]);
    assert!(!ok);
    assert!(
        result["error"]
            .as_str()
            .unwrap_or_default()
            .contains("reconcile"),
        "{result}"
    );
}
#[test]
fn mandatory_memory_fails_closed() {
    let f = Fixture::new("printf verified > output.txt", false);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    m["memory"] = json!({"required":true,"endpoint":"http://127.0.0.1:1/mcp","workspace":"fixture","projects":{"app":"repo"}});
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["status"], "blocked");
    assert!(!f.repo.join("output.txt").exists());
}
#[test]
fn approval_is_bound_to_revision() {
    let f = Fixture::new("printf verified > output.txt", true);
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let fingerprint = v["run"]["tasks"]["build"]["revision"]["fingerprint"]
        .as_str()
        .unwrap();
    assert!(
        f.call(&[
            "release",
            "approve",
            "--task",
            "build",
            "--gate",
            "review",
            "--reviewer",
            "fixture-human",
            "--fingerprint",
            fingerprint
        ])
        .0
    );
    let (ok, v) = f.call(&["resume"]);
    assert!(ok, "{v}");
}
#[test]
fn changed_manifest_blocks_resume() {
    let f = Fixture::new("printf verified > output.txt", true);
    f.call(&["release", "run"]);
    let s = std::fs::read_to_string(&f.manifest).unwrap();
    std::fs::write(&f.manifest, format!("{s}\n")).unwrap();
    assert!(!f.call(&["resume"]).0);
}
#[test]
fn lock_conflict_cancel_and_control() {
    let f = Fixture::new("sleep 10; printf verified > output.txt", false);
    let mut child = f.start();
    f.wait_running();
    assert!(!f.call(&["resume"]).0);
    assert!(f.call(&["cancel"]).0);
    assert!(!child.wait().unwrap().success());
    let (_, v) = f.call(&["status"]);
    assert_eq!(v["run"]["status"], "cancelled");
    assert!(!f.repo.join("output.txt").exists());
}
#[test]
fn pause_finishes_current_task_then_resumes() {
    let f = Fixture::new("sleep 1; printf verified > output.txt", false);
    let mut child = f.start();
    f.wait_running();
    assert!(f.call(&["pause"]).0);
    child.wait().unwrap();
    assert_eq!(f.call(&["status"]).1["run"]["status"], "paused");
    let (ok, v) = f.call(&["resume"]);
    assert!(ok, "{v}");
}
#[test]
fn evaluator_inside_repo_rejected() {
    let f = Fixture::new("true", false);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    let inside = f.repo.join("eval.py");
    std::fs::write(&inside, "pass").unwrap();
    m["gates"][0]["inputs"] = json!({inside.display().to_string():forge_domain::digest(b"pass")});
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(!f.call(&["release", "inspect"]).0);
}
#[test]
fn retries_are_bounded() {
    let f = Fixture::new("exit 1", false);
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["tasks"]["build"]["attempts"], 3);
    assert_eq!(v["run"]["status"], "blocked");
}
#[test]
fn state_cannot_live_in_agent_worktree() {
    let f = Fixture::new("true", false);
    let args = [
        "--json",
        "--state-dir",
        f.repo.to_str().unwrap(),
        "release",
        "inspect",
        f.manifest.to_str().unwrap(),
    ];
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
            .args(args)
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}
#[cfg(unix)]
#[test]
fn artifact_symlink_escape_rejected() {
    let f = Fixture::new("true", false);
    let secret = f.root.join("outside");
    std::fs::write(&secret, "verified").unwrap();
    std::os::unix::fs::symlink(secret, f.repo.join("output.txt")).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    m["tasks"][0]["acceptance"] =
        json!([{"kind":"artifact","path":"output.txt","sha256":forge_domain::digest(b"verified")}]);
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(!f.call(&["release", "run"]).0);
}

#[test]
fn crash_never_silently_restarts_orphan() {
    let f = Fixture::new("sleep 15; printf verified > output.txt", false);
    let mut child = f.start();
    f.wait_running();
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let pid = store.load("fixture").unwrap().unwrap().tasks["build"]
        .pid
        .unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    let (_, orphan) = f.call(&["report"]);
    assert_eq!(orphan["duration_estimate"]["status"], "unavailable");
    assert!(orphan["duration_estimate"]["observed_active_seconds"].is_null());
    let (ok, v) = f.call(&["resume"]);
    assert!(!ok, "{v}");
    assert!(v["error"].as_str().unwrap().contains("orphan"));
    #[cfg(unix)]
    Command::new("sh")
        .args(["-c", &format!("kill -9 -{pid}")])
        .status()
        .unwrap();
    #[cfg(windows)]
    {
        for _ in 0..100 {
            if !forge_adapters::process::pid_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("Job left orphan alive");
    }
}

#[test]
fn evaluator_tampering_invalidates_inspection() {
    let f = Fixture::new("printf verified > output.txt", false);
    std::fs::write(f.root.join("evaluate.py"), "# altered evaluator").unwrap();
    assert!(!f.call(&["release", "inspect"]).0);
    assert!(!f.state.exists());
}

#[cfg(windows)]
#[test]
fn artifact_junction_escape_rejected_without_symlink_privilege() {
    let f = Fixture::new("true", false);
    let outside = f.root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("output.txt"), "verified").unwrap();
    let link = f.repo.join("linked");
    assert!(
        Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .status()
            .unwrap()
            .success()
    );
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    m["tasks"][0]["acceptance"] = json!([{"kind":"artifact","path":"linked/output.txt","sha256":forge_domain::digest(b"verified")}]);
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(!f.call(&["release", "run"]).0);
    std::fs::remove_dir(link).unwrap();
}

#[test]
fn optional_observability_preserves_execution_failure_and_dry_run() {
    let f = Fixture::new("exit 1", false);
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    manifest["automation"] = json!({"observability":{"langsmith":"optional"}});
    std::fs::write(&f.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    for (args, dry) in [
        (vec!["release", "run", "--dry-run"], true),
        (vec!["release", "run"], false),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
            .args(f.args(&args))
            .current_dir(&f.root)
            .env("LANGSMITH_TRACING", "true")
            .env("LANGSMITH_API_KEY", "fixture-only")
            .env(
                "LANGSMITH_ENDPOINT",
                format!("http://{}", listener.local_addr().unwrap()),
            )
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        if dry {
            assert!(output.status.success());
            assert!(!f.state.exists());
        } else {
            assert!(!output.status.success());
            assert_eq!(value["run"]["status"], "blocked");
            assert_eq!(value["run"]["tasks"]["build"]["attempts"], 3);
        }
        // Production binaries never allow HTTP, even on loopback.
        assert!(listener.accept().is_err());
    }
}

// FORGE-020 regressions exercise the actual CLI and external, temporary stores.
#[test]
fn duration_inspect_dry_run_and_missing_status_are_read_only() {
    let f = Fixture::new("printf verified > output.txt", false);
    let (_, inspect) = f.call(&["release", "inspect"]);
    let (_, dry) = f.call(&["release", "run", "--dry-run"]);
    assert_eq!(
        inspect["duration_estimate"]["remaining_seconds"],
        dry["duration_estimate"]["remaining_seconds"]
    );
    for args in [vec!["status"], vec!["report"]] {
        let (ok, v) = f.call(&args);
        assert!(ok, "{v}");
        assert_eq!(v["status"], "not_run");
        assert_eq!(v["duration_estimate"]["counts"]["gate_invocations"], 2);
        assert_eq!(v["duration_estimate"]["confidence"], "low");
    }
    assert!(!f.state.exists());
    assert!(!f.repo.join("output.txt").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
        .args([
            "--state-dir",
            f.state.to_str().unwrap(),
            "release",
            "inspect",
            f.manifest.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let human = String::from_utf8(output.stdout).unwrap();
    assert!(human.contains("Estimated active work remaining:"));
    assert!(human.contains("confidence: low"));
    assert!(!f.state.exists());
}
#[test]
fn duration_initial_forecast_and_all_final_gates_are_persisted() {
    let f = Fixture::new("printf verified > output.txt", false);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    let initial = v["run"]["duration"]["initial_estimate"].clone();
    assert_eq!(initial["phase"], "planning");
    assert_eq!(initial["manifest_hash"], v["run"]["manifest_hash"]);
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let (history, error) = store.duration_history();
    assert!(error.is_none());
    assert_eq!(history.len(), 4);
    assert_eq!(history.iter().filter(|m| m.phase == "gate").count(), 2);
    assert_eq!(history.iter().filter(|m| m.final_evaluation).count(), 1);
    assert!(
        history
            .iter()
            .all(|m| m.elapsed_ms.is_some() && m.outcome == "passed")
    );
    let bytes_before = std::fs::read(f.state.join("state.sqlite")).unwrap();
    let (_, report) = f.call(&["report"]);
    assert_eq!(report["duration_comparison"]["initial_estimate"], initial);
    assert_eq!(report["duration_comparison"]["status"], "complete");
    assert!(report["duration_comparison"]["coverage"].is_boolean());
    assert!(report["duration_comparison"]["error_seconds"].is_number());
    assert!(report["duration_comparison"]["accuracy_target"].is_null());
    assert_eq!(
        report["duration_estimate"]["remaining_seconds"],
        json!({"lower":0.0,"upper":0.0})
    );
    assert!(report["duration_revisions"].as_array().unwrap().len() > 3);
    assert_eq!(
        bytes_before,
        std::fs::read(f.state.join("state.sqlite")).unwrap()
    );
}
#[test]
fn duration_historical_samples_are_local_and_not_invented() {
    let f = Fixture::new("printf verified > output.txt", false);
    for id in ["one", "two", "three"] {
        let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
        m["release_id"] = json!(id);
        std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        let (ok, v) = f.call(&["release", "run"]);
        assert!(ok, "{v}");
    }
    let (_, v) = f.call(&["release", "inspect"]);
    assert_eq!(v["duration_estimate"]["history"]["successful"], 9);
    assert_eq!(v["duration_estimate"]["status"], "estimated");
    assert_eq!(
        v["duration_estimate"]["basis"],
        "local_history_and_heuristics"
    );
    let conn = rusqlite::Connection::open(f.state.join("state.sqlite")).unwrap();
    conn.execute("UPDATE duration_measurements SET body='invalid' WHERE rowid=(SELECT max(rowid) FROM duration_measurements)", []).unwrap();
    let (_, v) = f.call(&["release", "inspect"]);
    assert_eq!(v["duration_estimate"]["history"]["available"], false);
    assert_eq!(v["duration_estimate"]["history"]["successful"], 0);
    assert_eq!(v["duration_estimate"]["basis"], "heuristic");
    assert!(
        v["duration_estimate"]["history"]["reason"]
            .as_str()
            .unwrap()
            .contains("corrupt")
    );
}
#[test]
fn duration_failures_and_retries_preserve_individual_measurements() {
    let f = Fixture::new("exit 1", false);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok);
    assert_eq!(
        v["duration_estimate"]["waiting_for"],
        "operator_intervention"
    );
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let (history, reason) = store.duration_history();
    assert!(reason.is_none());
    assert_eq!(history.len(), 4);
    let history: Vec<_> = history.iter().filter(|m| m.phase == "execution").collect();
    assert_eq!(
        history.iter().map(|m| m.attempt).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
    assert!(
        history
            .iter()
            .all(|m| m.outcome == "failed" && m.elapsed_ms.is_some())
    );
    let (_, report) = f.call(&["report"]);
    assert_eq!(report["duration_estimate"]["history"]["failed"], 3);
    assert_eq!(report["duration_comparison"]["status"], "partial");
    assert!(report["duration_comparison"]["coverage"].is_null());
}
#[test]
fn duration_human_wait_keeps_initial_and_active_time_until_resume() {
    let f = Fixture::new("printf verified > output.txt", true);
    let (_, v) = f.call(&["release", "run"]);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let initial = v["run"]["duration"]["initial_estimate"].clone();
    let (_, a) = f.call(&["status"]);
    let (_, b) = f.call(&["report"]);
    assert_eq!(
        a["duration_estimate"]["observed_active_seconds"],
        b["duration_estimate"]["observed_active_seconds"]
    );
    assert_eq!(
        a["duration_estimate"]["remaining_seconds"],
        b["duration_estimate"]["remaining_seconds"]
    );
    assert_eq!(b["duration_estimate"]["waiting_for"], "human_approval");
    assert_eq!(b["duration_estimate"]["conditional"], true);
    assert!(b["duration_estimate"]["wall_clock_eta"].is_null());
    let fingerprint = v["run"]["tasks"]["build"]["revision"]["fingerprint"]
        .as_str()
        .unwrap();
    assert!(
        f.call(&[
            "release",
            "approve",
            "--task",
            "build",
            "--gate",
            "review",
            "--reviewer",
            "Fixture",
            "--fingerprint",
            fingerprint
        ])
        .0
    );
    let (ok, resumed) = f.call(&["resume"]);
    assert!(ok, "{resumed}");
    assert_eq!(resumed["run"]["duration"]["initial_estimate"], initial);
    assert_eq!(resumed["run"]["tasks"]["build"]["attempts"], 1);
    assert!(
        resumed["duration_estimate"]["observed_active_seconds"]
            .as_f64()
            .unwrap()
            >= b["duration_estimate"]["observed_active_seconds"]
                .as_f64()
                .unwrap()
    );
}
#[test]
fn duration_live_cancel_and_pause_resume_keep_observations() {
    let f = Fixture::new("sleep 1; printf verified > output.txt", false);
    let mut child = f.start();
    f.wait_running();
    let (_, live) = f.call(&["status"]);
    assert_eq!(live["duration_estimate"]["phase"], "execution");
    assert!(live["duration_estimate"]["in_progress_elapsed_seconds"].is_number());
    assert!(f.call(&["pause"]).0);
    child.wait().unwrap();
    let (_, paused) = f.call(&["report"]);
    assert_eq!(paused["duration_estimate"]["waiting_for"], "paused");
    let initial = paused["run"]["duration"]["initial_estimate"].clone();
    assert!(f.call(&["resume"]).0);
    let (_, done) = f.call(&["report"]);
    assert_eq!(done["run"]["duration"]["initial_estimate"], initial);
    assert_eq!(done["run"]["tasks"]["build"]["attempts"], 1);

    let f = Fixture::new("sleep 10; printf verified > output.txt", false);
    let mut child = f.start();
    f.wait_running();
    assert!(f.call(&["cancel"]).0);
    assert!(!child.wait().unwrap().success());
    let (_, v) = f.call(&["report"]);
    assert_eq!(v["run"]["status"], "cancelled");
    assert_eq!(v["duration_estimate"]["history"]["cancelled"], 1);
    assert!(v["duration_estimate"]["observed_active_seconds"].is_number());
    assert!(v["duration_estimate"]["wall_clock_eta"].is_null());
}
#[test]
fn duration_timeout_and_reconciliation_archive_previous_generation() {
    let f = Fixture::new("sleep 10; printf verified > output.txt", false);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    m["tasks"][0]["timeout_seconds"] = json!(1);
    m["tasks"][0]["max_attempts"] = json!(1);
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(!f.call(&["release", "run"]).0);
    let (_, failed) = f.call(&["report"]);
    assert_eq!(failed["duration_estimate"]["history"]["timed_out"], 1);
    let initial = failed["run"]["duration"]["initial_estimate"].clone();
    m["tasks"][0]["executor"]["argv"] = json!(common::command("printf verified > output.txt"));
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    let (_, changed) = f.call(&["status"]);
    assert!(changed["duration_estimate"]["remaining_seconds"].is_null());
    assert!(!f.call(&["resume"]).0);
    let (ok, v) = f.call(&["resume", "--reconcile", "reviewed new contract"]);
    assert!(ok, "{v}");
    assert_eq!(v["run"]["duration"]["generation"], 1);
    let (_, report) = f.call(&["report"]);
    let archived = report["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "reconciled")
        .unwrap();
    assert_eq!(
        archived["body"]["previous_run"]["duration"]["initial_estimate"],
        initial
    );
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    assert_eq!(store.duration_history().0.len(), 6);
}
#[test]
fn duration_legacy_and_corrupt_state_remain_read_only() {
    let f = Fixture::new("printf verified > output.txt", false);
    assert!(f.call(&["release", "run"]).0);
    let path = f.state.join("state.sqlite");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE runs SET body=json_remove(body,'$.duration')", [])
            .unwrap();
        conn.execute_batch("DROP TABLE duration_forecasts; DROP TABLE duration_measurements;")
            .unwrap();
    }
    let before = std::fs::read(&path).unwrap();
    let (_, v) = f.call(&["report"]);
    assert_eq!(v["duration_estimate"]["status"], "unavailable");
    assert!(v["duration_estimate"]["observed_active_seconds"].is_null());
    assert!(v["duration_estimate"]["remaining_seconds"].is_null());
    assert_eq!(v["duration_comparison"]["status"], "unavailable");
    assert_eq!(before, std::fs::read(&path).unwrap());
    let conn = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'duration_%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    drop(conn);
    std::fs::write(&path, b"corrupt database").unwrap();
    let (_, v) = f.call(&["release", "inspect"]);
    assert_eq!(v["duration_estimate"]["history"]["available"], false);
    assert_eq!(std::fs::read(&path).unwrap(), b"corrupt database");
}
#[test]
fn duration_measurement_failure_rolls_back_state_and_event_together() {
    let f = Fixture::new("printf verified > output.txt", false);
    assert!(f.call(&["release", "run"]).0);
    let path = f.state.join("state.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_forecast BEFORE INSERT ON duration_forecasts BEGIN SELECT RAISE(ABORT,'fixture rejection'); END;").unwrap();
    let mut store = forge_domain::Store::open(&path, false).unwrap();
    let mut run = store.load("fixture").unwrap().unwrap();
    run.status = "must_not_commit".into();
    assert!(store.save(&run, "must_not_commit", &json!({})).is_err());
    assert_eq!(store.load("fixture").unwrap().unwrap().status, "passed");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM events WHERE kind='must_not_commit'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn duration_gate_failures_and_shared_final_gates_retain_each_invocation() {
    let f = Fixture::new("echo 'All done; all gates passed'", false);
    assert!(!f.call(&["release", "run"]).0);
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let (history, reason) = store.duration_history();
    assert!(reason.is_none());
    assert_eq!(history.iter().filter(|m| m.phase == "execution").count(), 3);
    assert_eq!(
        history
            .iter()
            .filter(|m| m.phase == "gate" && m.outcome == "failed")
            .count(),
        3
    );
    let (_, failed) = f.call(&["status"]);
    assert!(failed["duration_estimate"]["remaining_seconds"].is_null());
    assert!(
        failed["duration_estimate"]["unavailable_reason"]
            .as_str()
            .unwrap()
            .contains("exhausted")
    );

    let f = Fixture::new("printf verified > output.txt", false);
    let mut m: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    let mut dependent = m["tasks"][0].clone();
    dependent["id"] = json!("dependent");
    dependent["dependencies"] = json!(["build"]);
    m["tasks"].as_array_mut().unwrap().push(dependent);
    std::fs::write(&f.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    assert_eq!(v["duration_estimate"]["counts"]["gates"], 1);
    assert_eq!(v["duration_estimate"]["counts"]["gate_invocations"], 4);
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let history = store.duration_history().0;
    assert_eq!(history.iter().filter(|m| m.phase == "gate").count(), 4);
    assert_eq!(
        history
            .iter()
            .filter(|m| m.phase == "gate" && m.final_evaluation)
            .count(),
        2
    );
}
#[test]
fn duration_legacy_resume_does_not_fabricate_initial_or_total() {
    let f = Fixture::new("printf verified > output.txt", false);
    assert!(f.call(&["release", "run"]).0);
    let conn = rusqlite::Connection::open(f.state.join("state.sqlite")).unwrap();
    conn.execute("UPDATE runs SET body=json_remove(body,'$.duration')", [])
        .unwrap();
    conn.execute_batch("DROP TABLE duration_forecasts; DROP TABLE duration_measurements;")
        .unwrap();
    drop(conn);
    let (ok, v) = f.call(&["resume"]);
    assert!(ok, "{v}");
    assert!(v["run"]["duration"]["initial_estimate"].is_null());
    assert!(v["duration_estimate"]["observed_active_seconds"].is_null());
    let (_, report) = f.call(&["report"]);
    assert_eq!(report["duration_comparison"]["status"], "unavailable");
}
#[test]
fn duration_history_and_revision_queries_are_bounded() {
    let f = Fixture::new("printf verified > output.txt", false);
    assert!(f.call(&["release", "run"]).0);
    let path = f.state.join("state.sqlite");
    let store = forge_domain::Store::open(&path, true).unwrap();
    let m = store
        .duration_history()
        .0
        .into_iter()
        .find(|m| m.phase == "execution")
        .unwrap();
    let body = serde_json::to_string(&m).unwrap();
    let forecast = serde_json::to_string(
        &store
            .load("fixture")
            .unwrap()
            .unwrap()
            .duration
            .unwrap()
            .current_estimate,
    )
    .unwrap();
    let mut conn = rusqlite::Connection::open(&path).unwrap();
    let tx = conn.transaction().unwrap();
    for sequence in 0..205_i64 {
        tx.execute(
            "INSERT INTO duration_measurements VALUES('history',0,?1,?2,?3)",
            rusqlite::params![sequence, m.context, body],
        )
        .unwrap();
        tx.execute("INSERT INTO duration_forecasts(release_id,generation,reason,body) VALUES('fixture',0,'fixture',?1)", [&forecast]).unwrap();
    }
    tx.commit().unwrap();
    assert_eq!(store.duration_history().0.len(), 200);
    assert_eq!(store.duration_forecasts("fixture").unwrap().len(), 100);
    // A shared store with no run for the selected release is still a read-only plan query.
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&f.manifest).unwrap()).unwrap();
    manifest["release_id"] = json!("not_started");
    std::fs::write(&f.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let (_, v) = f.call(&["status"]);
    assert_eq!(v["status"], "not_run");
    assert!(store.load("not_started").unwrap().is_none());
}
