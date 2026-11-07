//! Protocol fixtures exercise real workflow/evaluator execution, not a live LLM account.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Fixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    repo: PathBuf,
    ops: PathBuf,
    input: PathBuf,
    bin: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let repo = base.join("project with spaces");
        fs::create_dir(&repo).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "--allow-empty",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(&repo)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        fs::write(repo.join("preserved.txt"), "preserve").unwrap();
        let bin = base.join("bin");
        fs::create_dir(&bin).unwrap();
        fs::write(bin.join("codex"), r#"#!/usr/bin/env python3
import json,os
from pathlib import Path
counter=Path(os.environ['CALL_COUNTER'])
calls=counter.read_text().count('call') if counter.exists() else 0
counter.write_text(('call\n')*(calls+1))
if not counter.with_name('prose-only').exists():
    Path('first.txt' if calls==0 else 'second.txt').write_text('verified')
print(json.dumps({'type':'thread.started','thread_id':'fixture'}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':100,'cached_input_tokens':0,'output_tokens':10}}))
"#).unwrap();
        fs::set_permissions(bin.join("codex"), fs::Permissions::from_mode(0o700)).unwrap();
        let ops = base.join("ops");
        let input = base.join("request.json");
        let checks:Vec<_>=[("AC1","first","first.txt"),("AC2","second","second.txt")].into_iter().map(|(id,task,file)|json!({"id":id,"argv":["python3","{checks}/verify.py","{repo}",file],"task_ids":[task],"timeout_seconds":10})).collect();
        let tasks:Vec<_>=[("first",Vec::<&str>::new(),0),("second",vec!["first"],1)].into_iter().map(|(id,deps,index)|json!({"id":id,"title":format!("Deliver {id} behavior"),"dependencies":deps,"acceptance":[{"id":checks[index]["id"],"description":"Expected output exists and unrelated work is preserved","command":checks[index]["argv"]}]})).collect();
        let request = json!({"schema_version":1,"id":"001-feature","title":"Incremental feature","request":"Deliver first.txt then second.txt with verified content; preserve preserved.txt","recipe":"feature","checks":checks,"files":{"verify.py":"import pathlib,sys\np=pathlib.Path(sys.argv[1]); f=p/sys.argv[2]\nsys.exit(0 if f.is_file() and f.read_text()=='verified' and (p/'preserved.txt').read_text()=='preserve' else 1)\n"},"proposal":{"schema_version":1,"id":"001-feature","title":"Incremental feature","summary":"Two independently validated dependent tasks","tasks":tasks}});
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        Self {
            _temp: temp,
            base,
            repo,
            ops,
            input,
            bin,
        }
    }
    fn call(&self, verb: &str, extra: &[&str]) -> (bool, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(["--json", "modules", "feature", verb])
            .arg(if verb == "prepare" {
                self.input.as_os_str()
            } else {
                std::ffi::OsStr::new("001-feature")
            })
            .arg("--root")
            .arg(&self.repo)
            .arg("--ops-root")
            .arg(&self.ops)
            .args(extra)
            .current_dir(&self.base)
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("CALL_COUNTER", self.base.join("calls"))
            .output()
            .unwrap();
        let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (output.status.success(), value)
    }
    fn calls(&self) -> usize {
        fs::read_to_string(self.base.join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
    fn edit(&self, f: impl FnOnce(&mut Value)) {
        let mut value = serde_json::from_slice(&fs::read(&self.input).unwrap()).unwrap();
        f(&mut value);
        fs::write(&self.input, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    fn prepare(&self) -> Value {
        let (ok, v) = self.call("prepare", &["--apply"]);
        assert!(ok, "{v}");
        v
    }
}
#[test]
fn feature_preview_and_admission_never_execute_or_guess_checks() {
    let f = Fixture::new();
    let (ok, v) = f.call("prepare", &[]);
    assert!(ok, "{v}");
    assert_eq!(v["status"], "preview");
    assert!(!f.ops.exists());
    assert!(!f.repo.join("specs").exists());
    f.edit(|r| r["checks"][0]["argv"] = json!(["true"]));
    assert!(!f.call("prepare", &["--apply"]).0);
    assert!(!f.ops.exists());
    assert_eq!(f.calls(), 0);
}
#[test]
fn feature_executes_dependency_order_and_checkpoints_without_replay() {
    let f = Fixture::new();
    let p = f.prepare();
    let digest = p["scope_digest"].as_str().unwrap();
    assert!(!f.call("start", &["--approve-scope", &"0".repeat(64)]).0);
    assert_eq!(f.calls(), 0);
    let (ok, v) = f.call("start", &["--approve-scope", digest]);
    assert!(ok, "{v}");
    assert_eq!(f.calls(), 2);
    let (ok, v) = f.call("checkpoint", &[]);
    assert!(ok, "{v}");
    assert_eq!(v["effective_status"], "passed");
    assert_eq!(v["checkpoint"]["authority"], "none");
    assert_eq!(v["checkpoint"]["automatic_compaction"], false);
    assert_eq!(v["continuity"]["progress"]["completed"], 2);
    assert_eq!(v["continuity"]["progress"]["total"], 2);
    assert_eq!(
        v["continuity"]["objective"],
        "Deliver first.txt then second.txt with verified content; preserve preserved.txt"
    );
    assert_eq!(v["continuity"]["authority"], "none");
    assert!(
        v["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["status"] == "passed")
    );
    assert_eq!(
        v["checkpoint"]["proposal"]["tasks"][1]["dependencies"],
        json!(["first"])
    );
    assert!(!f.call("start", &["--approve-scope", digest]).0);
    let (ok, v) = f.call("resume", &[]);
    assert!(ok, "{v}");
    assert_eq!(f.calls(), 2, "{v}");
    assert_eq!(
        fs::read_to_string(f.repo.join("preserved.txt")).unwrap(),
        "preserve"
    );
}
#[test]
fn prose_does_not_pass_first_gate_or_schedule_dependent_feature_task() {
    let f = Fixture::new();
    fs::write(f.base.join("prose-only"), "").unwrap();
    let p = f.prepare();
    let (ok, v) = f.call(
        "start",
        &["--approve-scope", p["scope_digest"].as_str().unwrap()],
    );
    assert!(!ok, "{v}");
    assert_eq!(f.calls(), 1);
    let (_, v) = f.call("checkpoint", &[]);
    assert_ne!(v["effective_status"], "passed");
    assert_eq!(v["checkpoint"]["authority"], "none");
    assert!(!f.repo.join("second.txt").exists());
}
#[test]
fn changed_source_and_cross_recipe_are_blocked_before_native_execution() {
    let f = Fixture::new();
    f.edit(|r| r["recipe"] = json!("pr"));
    assert!(!f.call("prepare", &["--apply"]).0);
    assert!(!f.ops.exists());
    f.edit(|r| r["recipe"] = json!("feature"));
    let p = f.prepare();
    fs::write(f.repo.join("new-unreviewed.txt"), "changed").unwrap();
    assert!(
        !f.call(
            "start",
            &["--approve-scope", p["scope_digest"].as_str().unwrap()]
        )
        .0
    );
    assert_eq!(f.calls(), 0);
}
#[test]
fn feature_scope_does_not_grant_declared_human_acceptance() {
    let f = Fixture::new();
    f.edit(|r| r["human_gates"] = json!(["owner-review"]));
    let p = f.prepare();
    let (ok, v) = f.call(
        "start",
        &["--approve-scope", p["scope_digest"].as_str().unwrap()],
    );
    assert!(!ok, "{v}");
    assert_eq!(f.calls(), 2);
    let (_, v) = f.call("status", &[]);
    assert_eq!(v["effective_status"], "awaiting_approval");
    assert_eq!(v["authorization"]["acceptance_granted"], false);
    assert_eq!(v["diagnostic"]["code"], "human_approval_required");
}

#[test]
fn prepared_continuity_is_read_only_and_stale_source_is_explained() {
    let f = Fixture::new();
    f.prepare();
    let state = f.ops.join("workflows/001-feature/state");
    assert!(!state.exists());
    let (ok, status) = f.call("status", &[]);
    assert!(ok, "{status}");
    assert_eq!(status["diagnostic"]["code"], "scope_approval_required");
    assert!(!state.exists());
    fs::write(f.repo.join("unreviewed.txt"), "changed").unwrap();
    let (ok, stale) = f.call("status", &[]);
    assert!(!ok, "{stale}");
    assert_eq!(stale["diagnostic"]["code"], "evidence_stale");
    assert!(!state.exists());
    assert_eq!(f.calls(), 0);
}
