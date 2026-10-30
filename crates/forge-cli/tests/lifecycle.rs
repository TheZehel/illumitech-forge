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
