//! Disposable native-protocol regressions. These do not call a model or prove native acceptance.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
    bin: PathBuf,
    manifest: PathBuf,
    state: PathBuf,
}
impl Fixture {
    fn new(mode: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let repo = root.join("project");
        let bin = root.join("bin");
        let state = root.join("state");
        std::fs::create_dir(&repo).unwrap();
        std::fs::create_dir(&bin).unwrap();
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
        let script = format!(
            r#"#!/usr/bin/env python3
import json,sys,time
from pathlib import Path
mode={mode:?}
p=Path('invocations.txt')
n=int(p.read_text())+1 if p.exists() else 1
p.write_text(str(n))
Path('received-prompt.txt').write_text(sys.argv[-1])
def event(v): print(json.dumps(v),flush=True)
event({{'type':'thread.started','thread_id':'fixture-session-'+str(n)}})
event({{'type':'turn.started'}})
event({{'type':'item.completed','item':{{'id':'item_1','type':'agent_message','text':'token=fixture-only'}}}})
usage={{'input_tokens':10*n,'cached_input_tokens':2,'output_tokens':3}}
if mode=='no-terminal':
    print('All done; tests passed',flush=True)
    sys.exit(0)
if mode in ['timeout','cancel']:
    event({{'type':'turn.failed','error':{{'code':'server_error'}},'usage':usage}})
    Path('ready').write_text('ready')
    time.sleep(60)
if mode in ['permission','auth'] or (mode=='retry' and n<3):
    code={{'permission':'permission_denied','auth':'authentication_failed'}}.get(mode,'server_error')
    event({{'type':'turn.failed','error':{{'code':code}},'usage':usage}})
    sys.exit(1)
Path('output.txt').write_text('verified')
terminal={{'type':'turn.completed','usage':usage}}
event(terminal)
event(terminal)
"#
        );
        let exe = bin.join("codex");
        std::fs::write(&exe, script).unwrap();
        std::fs::set_permissions(exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let manifest = root.join("manifest.yaml");
        let m = json!({"schema_version":1,"release_id":"economy","repos":{"app":{"path":repo}},"tasks":[{"id":"work","repo":"app","description":"Implement the required fixture output. Keep every instruction.","executor":{"kind":"codex","permission":"workspace_write"},"max_attempts":3,"timeout_seconds":2,"acceptance":[{"kind":"artifact","path":"output.txt","sha256":forge_domain::digest(b"verified")}]}]});
        std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        Self {
            _dir: dir,
            root,
            repo,
            bin,
            manifest,
            state,
        }
    }
    fn edit(&self, f: impl FnOnce(&mut Value)) {
        let mut m: Value = serde_json::from_slice(&std::fs::read(&self.manifest).unwrap()).unwrap();
        f(&mut m);
        std::fs::write(&self.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut paths = vec![self.bin.clone()];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        let mut command = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"));
        command
            .arg("--json")
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .arg(&self.manifest)
            .env("PATH", std::env::join_paths(paths).unwrap())
            .current_dir(&self.root);
        command
    }
    fn call(&self, args: &[&str]) -> (bool, Value) {
        let out = self.command(args).output().unwrap();
        let v = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
            panic!(
                "stdout={} stderr={}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        });
        (out.status.success(), v)
    }
}
#[test]
fn transient_retries_persist_usage_and_report_pages_do_not_limit_totals() {
    let f = Fixture::new("retry");
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 3);
    assert_eq!(v["token_usage"]["input_tokens"], 60);
    assert_eq!(v["token_usage"]["total_tokens"], 69);
    let (_, r) = f.call(&["report", "--limit", "1"]);
    assert_eq!(r["token_calls"].as_array().unwrap().len(), 1);
    assert_eq!(r["token_usage"]["total_tokens"], 69);
    assert_eq!(r["token_calls"][0]["attempt"], 3);
    assert_eq!(r["task_token_usage"]["work"]["total_tokens"], 69);
    let (_, r) = f.call(&["report", "--limit", "1", "--offset", "2"]);
    assert_eq!(r["token_calls"][0]["attempt"], 1);
    assert_eq!(r["token_calls"][0]["outcome"], "transient");
    let log: Value = serde_json::from_slice(
        &std::fs::read(f.state.join("logs/economy/generation-0/work-1.json")).unwrap(),
    )
    .unwrap();
    let stdout = log["stdout"].as_str().unwrap();
    assert!(!stdout.contains("fixture-only"));
    assert!(
        stdout
            .lines()
            .all(|l| serde_json::from_str::<Value>(l).is_ok())
    );
    assert!(
        f.state
            .join("logs/economy/generation-0/work-3.json")
            .exists()
    );
}
#[test]
fn permission_auth_and_protocol_blocks_do_not_retry_or_transfer_approval() {
    for (mode, kind) in [
        ("permission", "permission"),
        ("auth", "authentication"),
        ("no-terminal", "contract"),
    ] {
        let f = Fixture::new(mode);
        let (ok, v) = f.call(&["release", "run"]);
        assert!(!ok, "{v}");
        assert_eq!(v["run"]["tasks"]["work"]["attempts"], 1);
        assert_eq!(v["run"]["status"], "blocked");
        let (_, r) = f.call(&["report"]);
        assert_eq!(r["token_calls"][0]["outcome"], kind);
        let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
        assert!(
            store
                .duration_history()
                .0
                .iter()
                .filter(|m| m.phase == "execution")
                .all(|m| m.outcome != "passed")
        );
        let detail = r["run"]["tasks"]["work"]["evidence"][0]["detail"]
            .as_str()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(detail).unwrap()["retryable"],
            false
        );
        assert!(!f.call(&["resume"]).0);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("invocations.txt")).unwrap(),
            "1"
        );
        if mode == "no-terminal" {
            assert!(r["token_usage"]["total_tokens"].is_null());
            assert_eq!(r["token_usage"]["unknown_calls"], 1);
        }
    }
}
#[test]
fn timeout_usage_is_partial_and_reconciliation_keeps_previous_calls_and_logs() {
    let f = Fixture::new("timeout");
    f.edit(|m| m["tasks"][0]["timeout_seconds"] = json!(1));
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 1);
    assert!(v["token_usage"]["total_tokens"].is_null());
    assert_eq!(v["token_usage"]["observed"]["total_tokens"], 13);
    let (_, v) = f.call(&["resume", "--reconcile", "fixture retry after inspection"]);
    assert_eq!(v["token_usage"]["calls"], 2);
    assert_eq!(v["token_usage"]["observed"]["total_tokens"], 36);
    assert!(
        f.state
            .join("logs/economy/generation-0/work-1.json")
            .exists()
    );
    assert!(
        f.state
            .join("logs/economy/generation-1/work-1.json")
            .exists()
    );
}
#[test]
fn context_preserves_instructions_and_project_isolation() {
    let f = Fixture::new("success");
    std::fs::write(
        f.repo.join("AGENTS.md"),
        "Root instructions: preserve EVERY acceptance and configured model.\n",
    )
    .unwrap();
    std::fs::create_dir_all(f.repo.join("specs/feature")).unwrap();
    for file in ["spec.md", "plan.md", "tasks.md"] {
        std::fs::write(
            f.repo.join("specs/feature").join(file),
            format!("Mandatory contract {file}\n"),
        )
        .unwrap();
    }
    std::fs::create_dir(f.repo.join("src")).unwrap();
    std::fs::write(
        f.repo.join("src/AGENTS.md"),
        "Nested instructions: keep scoped policy.\n",
    )
    .unwrap();
    std::fs::write(f.repo.join("src/small.rs"), "optional small file\n").unwrap();
    std::fs::write(
        f.repo.join("large.txt"),
        "large optional context\n".repeat(5000),
    )
    .unwrap();
    f.edit(|m|{m["repos"]["app"]["specs"]=json!(["feature"]);m["context"]=json!({"profile":"bounded","budget_tokens":2000,"sources":["src/small.rs","large.txt"]});});
    let (ok, plan) = f.call(&[
        "context",
        "plan",
        "--task",
        "work",
        "--budget-tokens",
        "2000",
    ]);
    assert!(ok, "{plan}");
    assert!(!f.state.exists());
    assert!(!f.repo.join("invocations.txt").exists());
    let sources = plan["context_plan"]["sources"].as_array().unwrap();
    assert!(
        sources
            .iter()
            .filter(|s| s["mandatory"] == true)
            .all(|s| s["included"] == true)
    );
    assert!(
        sources
            .iter()
            .any(|s| s["path"] == "src/AGENTS.md" && s["mandatory"] == true)
    );
    assert!(
        sources
            .iter()
            .any(|s| s["path"] == "large.txt" && s["included"] == false)
    );
    assert!(plan["context_plan"]["estimated_tokens"].as_u64().unwrap() <= 2000);
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    let prompt = std::fs::read_to_string(f.repo.join("received-prompt.txt")).unwrap();
    for required in [
        "Root instructions",
        "Nested instructions",
        "Mandatory contract spec.md",
        "Mandatory contract plan.md",
        "Mandatory contract tasks.md",
        "Acceptance contract:",
        "Never modify orchestration state",
        "large.txt",
    ] {
        assert!(prompt.contains(required), "missing {required}");
    }
    assert!(!prompt.contains("large optional context"));
    // A link outside the root must not become recoverable context or leak content.
    let outside = f.root.join("other-project.txt");
    std::fs::write(&outside, "OTHER PROJECT PRIVATE CONTENT").unwrap();
    std::os::unix::fs::symlink(outside, f.repo.join("outside.txt")).unwrap();
    f.edit(|m| m["context"]["sources"] = json!(["outside.txt"]));
    let (ok, v) = f.call(&[
        "context",
        "plan",
        "--task",
        "work",
        "--budget-tokens",
        "2000",
    ]);
    assert!(!ok, "{v}");
    assert_eq!(v["context_plan"]["status"], "blocked");
    assert!(!v.to_string().contains("OTHER PROJECT PRIVATE CONTENT"));
}
#[test]
fn oversize_context_blocks_before_invocation_and_full_is_the_default() {
    let f = Fixture::new("success");
    let (_, inspect) = f.call(&["release", "inspect"]);
    assert!(inspect["tasks"][0]["context_plan"].is_null());
    let args = inspect["tasks"][0]["argv"].as_array().unwrap();
    assert!(
        !args
            .iter()
            .any(|a| a == "--model" || a == "--ignore-user-config" || a == "--approve-for-me")
    );
    std::fs::write(
        f.repo.join("AGENTS.md"),
        "mandatory instructions\n".repeat(3000),
    )
    .unwrap();
    f.edit(|m| m["context"] = json!({"profile":"bounded","budget_tokens":100}));
    let (ok, p) = f.call(&[
        "context",
        "plan",
        "--task",
        "work",
        "--budget-tokens",
        "100",
    ]);
    assert!(!ok, "{p}");
    assert_eq!(p["context_plan"]["sources"][0]["included"], true);
    assert!(
        p["context_plan"]["mandatory_estimated_tokens"]
            .as_u64()
            .unwrap()
            > 100
    );
    assert!(!f.state.exists());
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 0);
    assert!(!f.repo.join("invocations.txt").exists());
    assert_eq!(v["token_usage"]["calls"], 0);
}

#[test]
fn cancellation_keeps_observed_usage_without_retrying_or_claiming_completion() {
    let f = Fixture::new("cancel");
    f.edit(|m| m["tasks"][0]["timeout_seconds"] = json!(20));
    let mut child = f
        .command(&["release", "run"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut ready = false;
    for _ in 0..100 {
        if f.repo.join("ready").exists() {
            ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    if !ready {
        let _ = child.kill();
        let _ = child.wait();
        panic!("fixture did not start");
    }
    assert!(f.call(&["cancel"]).0);
    assert!(!child.wait().unwrap().success());
    let (_, v) = f.call(&["report"]);
    assert_eq!(v["run"]["status"], "cancelled");
    assert_eq!(v["token_calls"][0]["outcome"], "cancelled");
    assert!(v["token_usage"]["total_tokens"].is_null());
    assert_eq!(v["token_usage"]["observed"]["total_tokens"], 13);
    assert_eq!(v["token_usage"]["calls"], 1);
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 1);
}
#[test]
fn failed_evidence_does_not_trigger_redundant_native_calls() {
    let f = Fixture::new("success");
    f.edit(|m| {
        m["tasks"][0]["acceptance"][0]["sha256"] = json!(forge_domain::digest(b"different"))
    });
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["status"], "blocked");
    assert_eq!(v["run"]["tasks"]["work"]["attempts"], 1);
    assert_eq!(v["token_usage"]["total_tokens"], 13);
    let (_, report) = f.call(&["report"]);
    assert_eq!(report["token_usage"]["calls"], 1);
}
#[test]
fn bounded_context_rejects_nested_projects_and_task_override_remains_explicit() {
    let f = Fixture::new("success");
    std::fs::create_dir_all(f.repo.join("nested/.git")).unwrap();
    std::fs::write(
        f.repo.join("nested/private.md"),
        "another project's content",
    )
    .unwrap();
    f.edit(|m| {
        m["context"] =
            json!({"profile":"bounded","budget_tokens":2000,"sources":["nested/private.md"]})
    });
    let (ok, p) = f.call(&[
        "context",
        "plan",
        "--task",
        "work",
        "--budget-tokens",
        "2000",
    ]);
    assert!(!ok, "{p}");
    assert_eq!(p["context_plan"]["status"], "blocked");
    assert!(!p.to_string().contains("another project's content"));
    f.edit(|m| m["tasks"][0]["context"] = json!({"profile":"full"}));
    let (_, p) = f.call(&["release", "inspect"]);
    assert!(p["tasks"][0]["context_plan"].is_null());
    assert!(!f.state.exists());
}
