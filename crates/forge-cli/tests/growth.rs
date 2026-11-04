#![cfg(unix)]
mod common;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
struct Fixture {
    _dir: tempfile::TempDir,
    base: PathBuf,
    repo: PathBuf,
    ops: PathBuf,
    bin: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new(human: bool, bug: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let repo = base.join("project with spaces");
        fs::create_dir(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
            vec![
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgSign=false",
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
        fs::write(repo.join("preserved.txt"), "keep").unwrap();
        let bin = base.join("bin");
        fs::create_dir(&bin).unwrap();
        fs::write(bin.join("codex"),r#"#!/usr/bin/env python3
import os,json
from pathlib import Path
p=Path(os.environ['CALL_COUNTER']);p.write_text(p.read_text()+'call\n' if p.exists() else 'call\n')
if os.environ.get('WRITE_OUTPUT')=='yes' and not p.with_name('prose-only').exists():Path('output.txt').write_text('verified')
print(json.dumps({'type':'thread.started','thread_id':'fixture'}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':100,'cached_input_tokens':0,'output_tokens':10}}))
"#).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin.join("codex"), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let ops = base.join("ops");
        let input = base.join("request.json");
        let check = json!({"id":"regression","argv":[common::python(),"{checks}/verify.py","{repo}"],"before_exit":if bug{json!(1)}else{Value::Null},"timeout_seconds":10});
        let request = json!({"schema_version":1,"id":"001-fix","title":"Fix fixture","request":"Create output.txt with exactly verified; preserve preserved.txt.","recipe":if bug{"bug"}else{"feature"},"checks":[check],"files":{"verify.py":"import pathlib,sys\np=pathlib.Path(sys.argv[1]);sys.exit(0 if (p/'output.txt').is_file() and (p/'output.txt').read_text()=='verified' and (p/'preserved.txt').read_text()=='keep' else 1)\n"},"human_gates":if human{vec!["owner-review"]}else{vec![]}});
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        Self {
            _dir: dir,
            base,
            repo,
            ops,
            bin,
            input,
        }
    }
    fn call(&self, args: Vec<String>) -> (bool, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .arg("--json")
            .args(args)
            .current_dir(&self.base)
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("CALL_COUNTER", self.base.join("calls"))
            .env("WRITE_OUTPUT", "yes")
            .output()
            .unwrap();
        let v = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.success(), v)
    }
    fn args(&self, verb: &str) -> Vec<String> {
        vec![
            "workflow".into(),
            verb.into(),
            "001-fix".into(),
            "--root".into(),
            self.repo.display().to_string(),
            "--ops-root".into(),
            self.ops.display().to_string(),
        ]
    }
    fn prepare(&self, apply: bool) -> Value {
        let mut args = self.args("prepare");
        args[2] = self.input.display().to_string();
        if apply {
            args.push("--apply".into());
        }
        let (ok, v) = self.call(args);
        assert!(ok, "{v}");
        v
    }
    fn start(&self, digest: &str) -> (bool, Value) {
        let mut args = self.args("start");
        args.extend(["--approve-scope".into(), digest.into()]);
        self.call(args)
    }
    fn calls(&self) -> usize {
        fs::read_to_string(self.base.join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}
#[test]
fn preview_never_writes_or_calls_a_client() {
    let f = Fixture::new(false, false);
    let p = f.prepare(false);
    assert_eq!(p["status"], "preview");
    assert!(!f.ops.exists());
    assert!(!f.repo.join("specs").exists());
    assert_eq!(f.calls(), 0);
}
#[test]
fn exact_scope_runs_external_gates_and_replay_is_refused() {
    let f = Fixture::new(false, true);
    let p = f.prepare(true);
    let digest = p["scope_digest"].as_str().unwrap();
    assert!(!f.start(&"0".repeat(64)).0);
    assert_eq!(f.calls(), 0);
    let (ok, v) = f.start(digest);
    assert!(ok, "{v}");
    assert_eq!(v["run"]["status"], "passed");
    assert_eq!(f.calls(), 1);
    assert_eq!(
        fs::read_to_string(f.repo.join("preserved.txt")).unwrap(),
        "keep"
    );
    assert!(!f.start(digest).0);
    assert_eq!(f.calls(), 1);
    let (ok, status) = f.call(f.args("diagnose"));
    assert!(ok, "{status}");
    assert_eq!(status["effective_status"], "passed");
    assert_eq!(status["mutation"], false);
}
#[test]
fn changing_source_contract_or_evaluator_blocks_before_spawn() {
    for kind in ["source", "manifest", "checks", "binary"] {
        let f = Fixture::new(false, false);
        let p = f.prepare(true);
        match kind {
            "source" => fs::write(f.repo.join("unexpected.txt"), "change").unwrap(),
            "manifest" => {
                let path = f.ops.join("workflows/001-fix/release.json");
                let mut m: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                m["tasks"][0]["max_attempts"] = json!(2);
                fs::write(path, serde_json::to_vec(&m).unwrap()).unwrap();
            }
            "checks" => {
                fs::write(f.ops.join("workflows/001-fix/checks.json"), "{}").unwrap();
            }
            _ => {
                fs::write(
                    Path::new(p["candidate"]["evaluator"].as_str().unwrap()),
                    "replaced",
                )
                .unwrap();
            }
        }
        assert!(!f.start(p["scope_digest"].as_str().unwrap()).0, "{kind}");
        assert_eq!(f.calls(), 0);
    }
}
#[test]
fn scope_consent_never_satisfies_a_declared_human_gate() {
    let f = Fixture::new(true, false);
    let p = f.prepare(true);
    let (ok, v) = f.start(p["scope_digest"].as_str().unwrap());
    assert!(!ok);
    assert_eq!(v["run"]["status"], "awaiting_approval");
    assert_eq!(f.calls(), 1);
}
#[test]
fn stale_output_cannot_keep_success_exit_code() {
    let f = Fixture::new(false, false);
    let p = f.prepare(true);
    assert!(f.start(p["scope_digest"].as_str().unwrap()).0);
    fs::write(f.repo.join("output.txt"), "stale").unwrap();
    let (ok, v) = f.call(f.args("status"));
    assert!(!ok);
    assert_eq!(v["effective_status"], "blocked");
    assert_eq!(v["report"]["run"]["status"], "passed");
    assert_eq!(f.calls(), 1);
}
#[test]
fn interrupted_preparation_is_diagnosed_without_replaying_commands() {
    let f = Fixture::new(false, false);
    let p = f.prepare(true);
    let db = f.ops.join("workflows/001-fix/state/state.sqlite");
    let store = forge_domain::Store::open(&db, false).unwrap();
    store
        .set_metadata(
            "workflow-scope",
            &json!({"digest":p["scope_digest"],"phase":"preparing","acceptance_granted":false}),
        )
        .unwrap();
    drop(store);
    let (ok, _) = f.call(f.args("resume"));
    assert!(!ok);
    assert_eq!(f.calls(), 0);
    let (_, v) = f.call(f.args("diagnose"));
    assert_eq!(v["scope"]["phase"], "preparing");
}
#[test]
fn external_regression_tampering_and_existing_spec_conflicts_are_preserved() {
    let f = Fixture::new(false, false);
    let p = f.prepare(true);
    let dir = f.repo.join("specs/001-fix");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("spec.md"), "unrelated customization").unwrap();
    assert!(!f.start(p["scope_digest"].as_str().unwrap()).0);
    assert_eq!(
        fs::read_to_string(dir.join("spec.md")).unwrap(),
        "unrelated customization"
    );
    assert_eq!(f.calls(), 0);
}

#[test]
fn agent_prose_cannot_satisfy_external_verification() {
    let f = Fixture::new(false, false);
    let p = f.prepare(true);
    fs::write(f.base.join("prose-only"), "all done, all gates passed").unwrap();
    let (ok, v) = f.start(p["scope_digest"].as_str().unwrap());
    assert!(!ok);
    assert_eq!(v["run"]["status"], "blocked");
    assert_eq!(f.calls(), 1);
    assert!(!f.repo.join("output.txt").exists());
}

#[test]
fn frozen_regression_changes_invalidate_current_evidence() {
    let f = Fixture::new(false, false);
    let p = f.prepare(true);
    assert!(f.start(p["scope_digest"].as_str().unwrap()).0);
    fs::write(
        f.ops.join("workflows/001-fix/regressions/verify.py"),
        "raise SystemExit(0)\n",
    )
    .unwrap();
    let (ok, v) = f.call(f.args("status"));
    assert!(!ok);
    assert!(v["status"] == "blocked" || v["effective_status"] == "blocked");
    assert_eq!(f.calls(), 1);
}

#[test]
fn environment_error_is_not_the_guided_bug_baseline() {
    let f = Fixture::new(false, true);
    let mut request: Value = serde_json::from_slice(&fs::read(&f.input).unwrap()).unwrap();
    request["checks"][0]["before_exit"] = json!(42);
    request["files"]["verify.py"] = json!("raise RuntimeError('fixture environment failure')\n");
    fs::write(&f.input, serde_json::to_vec(&request).unwrap()).unwrap();
    let p = f.prepare(true);
    assert!(!f.start(p["scope_digest"].as_str().unwrap()).0);
    assert_eq!(f.calls(), 0);
    let (ok, s) = f.call(f.args("diagnose"));
    assert!(!ok, "blocked diagnosis must retain a failure exit: {s}");
    assert_eq!(s["baseline"]["status"], "blocked");
    assert!(!f.call(f.args("resume")).0);
    assert_eq!(f.calls(), 0);
}
#[test]
fn export_is_preview_by_default_and_never_includes_project_data() {
    let f = Fixture::new(false, true);
    let p = f.prepare(true);
    assert!(f.start(p["scope_digest"].as_str().unwrap()).0);
    let (ok, v) = f.call(f.args("export"));
    assert!(ok, "{v}");
    assert_eq!(v["status"], "preview");
    let serialized = v.to_string();
    for forbidden in [
        f.repo.to_str().unwrap(),
        "preserved.txt",
        "verify.py",
        "prompt",
        "stderr",
        "stdout",
    ] {
        assert!(!serialized.contains(forbidden), "{forbidden}");
    }
    let out = f.base.join("report-v1.json");
    let mut args = f.args("export");
    args.extend([
        "--output".into(),
        out.display().to_string(),
        "--human-minutes".into(),
        "7".into(),
    ]);
    let (ok, v) = f.call(args.clone());
    assert!(ok, "{v}");
    assert_eq!(v["report"]["metrics"]["human_minutes"], 7);
    assert!(out.exists());
    assert!(!f.call(args).0);
}

#[test]
fn rust_and_typescript_external_regressions_fail_before_and_pass_after() {
    for language in ["rust", "typescript"] {
        let f = Fixture::new(false, true);
        let (file, before, after, verifier, argv) = if language == "rust" {
            ("lib.rs","pub fn add(a:i32,b:i32)->i32{a-b}","pub fn add(a:i32,b:i32)->i32{a+b}",
             "import pathlib,sys,tempfile,subprocess,json\nr=pathlib.Path(sys.argv[1])\nwith tempfile.TemporaryDirectory() as d:\n p=pathlib.Path(d); src=p/'check.rs'; src.write_text('#[path='+json.dumps(str(r/'lib.rs'))+'] mod subject; fn main(){std::process::exit(if subject::add(1,1)==2 {0}else{42});}')\n c=subprocess.run(['rustc',str(src),'-o',str(p/'check')]); sys.exit(c.returncode if c.returncode else subprocess.run([str(p/'check')]).returncode)\n".to_string(), vec![common::python(),"{checks}/verify.py","{repo}"])
        } else {
            ("app.ts","export function add(a:number,b:number):number{return a-b}","export function add(a:number,b:number):number{return a+b}",
             "import pathlib,sys,subprocess\nr=pathlib.Path(sys.argv[1]);code='import {add} from '+repr((r/'app.ts').as_uri())+';process.exit(add(1,1)===2?0:42)'\nsys.exit(subprocess.run(['node','--input-type=module','-e',code]).returncode)\n".to_string(), vec![common::python(),"{checks}/verify.py","{repo}"])
        };
        fs::write(f.repo.join(file), before).unwrap();
        let client = fs::read_to_string(f.bin.join("codex")).unwrap().replace(
            "print(json.dumps({'type':'thread.started'",
            &format!(
                "Path({file:?}).write_text({after:?})\nprint(json.dumps({{'type':'thread.started'"
            ),
        );
        fs::write(f.bin.join("codex"), client).unwrap();
        let mut request: Value = serde_json::from_slice(&fs::read(&f.input).unwrap()).unwrap();
        request["checks"][0]["before_exit"] = json!(42);
        request["checks"][0]["argv"] = json!(argv);
        request["files"]["verify.py"] = json!(verifier);
        fs::write(&f.input, serde_json::to_vec(&request).unwrap()).unwrap();
        let p = f.prepare(true);
        let (ok, v) = f.start(p["scope_digest"].as_str().unwrap());
        assert!(ok, "{language}: {v}");
        assert_eq!(v["run"]["status"], "passed");
        assert_eq!(f.calls(), 1);
        let (ok, s) = f.call(f.args("status"));
        assert!(ok, "{s}");
        assert_eq!(s["baseline"]["evidence"]["actual_exit"], 42);
    }
}
