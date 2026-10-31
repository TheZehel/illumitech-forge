//! Disposable behavioral regressions. Native fixtures test protocol/control boundaries;
//! they are not evidence of provider, native platform or coordinator acceptance.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Fixture {
    _dir: tempfile::TempDir,
    base: PathBuf,
    a: PathBuf,
    b: PathBuf,
    manifest: PathBuf,
    state: PathBuf,
}
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let a = base.join("a");
        let b = base.join("b");
        for root in [&a, &b] {
            std::fs::create_dir(root).unwrap();
            git(root, &["init", "-q"]);
            git(
                root,
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "fixture",
                ],
            );
            std::fs::write(root.join("preserved.txt"), "keep\n").unwrap();
        }
        let gate = base.join("gate.py");
        std::fs::write(&gate, "pass\n").unwrap();
        let tasks:Vec<_>=["a","b"].iter().map(|id|json!({"id":id,"repo":id,"description":"Disposable local regression","executor":{"kind":"command","argv":["python3","-c","pass"]},"acceptance":[{"kind":"gate","id":"proof"}],"max_attempts":1})).collect();
        let m = json!({"schema_version":1,"release_id":"m07","execution":{"max_parallel":2},"repos":{"a":{"path":a},"b":{"path":b}},"tasks":tasks,
        "gates":[{"id":"proof","argv":["python3",gate],"inputs":{gate.to_str().unwrap():forge_domain::digest(b"pass\n")}}]});
        let manifest = base.join("release.yaml");
        std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        let state = base.join("state");
        Self {
            _dir: dir,
            base,
            a,
            b,
            manifest,
            state,
        }
    }
    fn edit(&self, f: impl FnOnce(&mut Value)) {
        let mut v: Value = serde_json::from_slice(&std::fs::read(&self.manifest).unwrap()).unwrap();
        f(&mut v);
        std::fs::write(&self.manifest, serde_json::to_vec(&v).unwrap()).unwrap();
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"));
        c.arg("--json")
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .arg(&self.manifest)
            .current_dir(&self.base);
        c
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
    fn barrier(&self, heartbeat: bool) {
        let barrier = self.base.join("barrier");
        std::fs::create_dir(&barrier).unwrap();
        self.edit(|m|for (i,id) in ["a","b"].iter().enumerate() {
            let peer=if *id=="a"{"b"}else{"a"};
            let code=format!("from pathlib import Path\nimport time,subprocess,sys\np=Path({barrier:?})\n(p/{id:?}).write_text('ready')\nfor _ in range(1000):\n if (p/{peer:?}).exists(): break\n time.sleep(.01)\nelse: raise RuntimeError('concurrent peer did not reach barrier')\nPath('result').write_text('joined')\n{}",if heartbeat {
                "subprocess.Popen([sys.executable,'-c',\"import time\\nfrom pathlib import Path\\np=Path('heartbeat')\\nfor i in range(10000):\\n p.write_text(str(i))\\n time.sleep(.01)\"])\ntime.sleep(60)"
            }else{""});
            m["tasks"][i]["executor"]["argv"]=json!(["python3","-c",code]);
        });
    }
    fn server(&self, mode: &str) -> PathBuf {
        let server = self.base.join(format!("review-{mode}.py"));
        let code = format!(
            r#"#!/usr/bin/env python3
import json,sys,re,hashlib
from pathlib import Path
root=Path.cwd()
for line in sys.stdin:
 v=json.loads(line);method=v.get('method');rid=v.get('id')
 def reply(result): print(json.dumps({{'id':rid,'result':result}}),flush=True)
 def event(method,params): print(json.dumps({{'method':method,'params':params}}),flush=True)
 if method=='initialize': reply({{'protocol':'fixture'}})
 elif method=='account/read': reply({{'account':{{'type':'chatgpt'}}}})
 elif method=='thread/start':
  assert v['params']['sandbox']=='read-only'
  assert v['params']['approvalPolicy']=='never'
  assert 'model' not in v['params']
  reply({{'thread':{{'id':'review-thread','cwd':str(root)}}}})
 elif method=='turn/start':
  if {mode:?}=='wait':
   event('turn/started',{{'threadId':'review-thread','turn':{{'id':'review-turn'}}}})
   reply({{'turn':{{'id':'review-turn'}}}})
   Path({calls:?}).write_text('waiting')
   continue
  reply({{'turn':{{'id':'review-turn'}}}})
  Path({calls:?}).write_text('called')
  if {mode:?}.startswith('write'):
   (root/'preserved.txt').write_text('malicious modification')
   if {mode:?}=='write-disconnect': sys.exit(0)
  event('thread/tokenUsage/updated',{{'threadId':'review-thread','tokenUsage':{{'total':{{'inputTokens':20,'cachedInputTokens':5,'outputTokens':3}}}}}})
  findings=[]
  if {mode:?} in ('finding','stale','escape'):
   fp=re.search(r'current fingerprint ([0-9a-f]{{64}})',v['params']['input'][0]['text']).group(1)
   findings=[{{'id':'issue','severity':'error','repo':root.name,'file':'../outside' if {mode:?}=='escape' else 'preserved.txt','line':1,'fingerprint':'0'*64 if {mode:?}=='stale' else fp,'evidence_sha256':hashlib.sha256((root/'preserved.txt').read_bytes()).hexdigest(),'message':'A supported fixture disagreement'}}]
  text=json.dumps({{'schema_version':1,'findings':findings}}) if {mode:?}!='invalid' else 'I agree; accepted'
  event('item/completed',{{'threadId':'review-thread','item':{{'type':'agentMessage','text':text}}}})
  event('turn/completed',{{'threadId':'review-thread','turn':{{'id':'review-turn','status':'completed'}}}})
 elif method=='turn/interrupt':
  Path({calls:?}+'.interrupted').write_text('native interrupt requested')
  reply({{}})
  event('turn/completed',{{'threadId':'review-thread','turn':{{'id':'review-turn','status':'interrupted'}}}})
"#,
            calls = self.base.join("native-called").to_str().unwrap()
        );
        std::fs::write(&server, code).unwrap();
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        server
    }
    fn review(&self, mode: &str) -> (bool, Value) {
        let server = self.server(mode);
        let out = self
            .command(&["review", "run"])
            .arg("--app-server")
            .arg(server)
            .output()
            .unwrap();
        (
            out.status.success(),
            serde_json::from_slice(&out.stdout).unwrap(),
        )
    }
}
fn wait_file(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "missing {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn wait_child(mut child: Child) -> Value {
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            panic!("child failed to stop");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child.wait_with_output().unwrap();
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr)))
}
#[test]
fn parallel_respects_dependencies_locks_and_budget() {
    let f = Fixture::new();
    f.barrier(false);
    f.edit(|m| {
        let mut c=m["tasks"][0].clone();c["id"]=json!("dependent");c["dependencies"]=json!(["a","b"]);
        c["executor"]["argv"]=json!(["python3","-c",format!("from pathlib import Path;assert Path('result').read_text()=='joined';assert Path({:?}).read_text()=='joined';Path('dependent').write_text('done')",f.b.join("result").to_str().unwrap())]);
        m["tasks"].as_array_mut().unwrap().push(c);
    });
    let (ok, v) = f.call(&["release", "run"]);
    assert!(ok, "{v}");
    assert_eq!(v["run"]["tasks"]["dependent"]["status"], "passed");
    let (_, report) = f.call(&["report"]);
    let events = report["events"].as_array().unwrap();
    let active = events
        .iter()
        .filter(|e| e["kind"] == "process_spawned")
        .count();
    assert!(active >= 3);
    assert_eq!(
        std::fs::read_to_string(f.a.join("preserved.txt")).unwrap(),
        "keep\n"
    );
    // A shared attempt ceiling stops admission before the second subprocess.
    let f = Fixture::new();
    f.edit(|m| m["budget"] = json!({"max_attempts":1}));
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok);
    assert_eq!(v["run"]["tasks"]["b"]["attempts"], 0);
}
#[test]
fn parallel_cancel_ends_all_owned_processes() {
    let f = Fixture::new();
    f.barrier(true);
    let child = f
        .command(&["release", "run"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_file(&f.a.join("heartbeat"));
    wait_file(&f.b.join("heartbeat"));
    let (_, live) = f.call(&["status"]);
    assert_ne!(
        live["duration_estimate"]["status"], "unavailable",
        "a live parallel owner must keep the DAG forecast observable: {live}"
    );
    let (ok, v) = f.call(&["cancel"]);
    assert!(ok, "{v}");
    let v = wait_child(child);
    assert_eq!(v["run"]["status"], "cancelled", "{v}");
    let before = [
        std::fs::read(f.a.join("heartbeat")).unwrap(),
        std::fs::read(f.b.join("heartbeat")).unwrap(),
    ];
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(before[0], std::fs::read(f.a.join("heartbeat")).unwrap());
    assert_eq!(before[1], std::fs::read(f.b.join("heartbeat")).unwrap());
    assert!(
        v["run"]["tasks"]
            .as_object()
            .unwrap()
            .values()
            .all(|s| s["pid"].is_null())
    );
}
#[test]
fn reviewer_cannot_modify_implementation_even_on_disconnection() {
    for mode in ["write", "write-disconnect"] {
        let f = Fixture::new();
        let (ok, v) = f.review(mode);
        assert!(!ok, "{v}");
        assert_eq!(v["review"]["block_reason"], "read_only_violation", "{v}");
        assert_eq!(
            std::fs::read_to_string(f.a.join("preserved.txt")).unwrap(),
            "malicious modification"
        );
        assert_eq!(
            std::fs::read_to_string(f.b.join("preserved.txt")).unwrap(),
            "keep\n"
        );
        assert_eq!(v["review"]["changed_scope"].as_array().unwrap().len(), 1);
    }
}
#[test]
fn review_findings_reference_current_revision() {
    for mode in ["stale", "escape", "invalid"] {
        let f = Fixture::new();
        let (ok, v) = f.review(mode);
        assert!(!ok, "{mode} {v}");
        assert_eq!(v["review"]["status"], "blocked");
    }
    let f = Fixture::new();
    let (ok, v) = f.review("finding");
    assert!(!ok, "{v}");
    assert_eq!(v["review"]["block_reason"], "findings_require_resolution");
    assert_eq!(v["review"]["findings"].as_array().unwrap().len(), 2);
}
#[test]
fn review_disagreement_does_not_fake_acceptance_and_rounds_are_budgeted() {
    let f = Fixture::new();
    f.edit(|m| m["review"] = json!({"max_iterations":1}));
    let (ok, v) = f.call(&["review", "plan"]);
    assert!(ok, "{v}");
    assert!(v["review_plan"]["finding_schema"].is_object());
    assert!(!f.state.exists());
    let (ok, v) = f.review("empty");
    assert!(ok, "{v}");
    assert_eq!(v["review"]["acceptance"], "unchanged");
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    assert!(store.load("m07").unwrap().is_none());
    let (ok, v) = f.review("empty");
    assert!(!ok, "{v}");
    assert!(v["error"].as_str().unwrap().contains("iteration budget"));
    let f = Fixture::new();
    f.edit(|m| m["budget"] = json!({"max_tokens":0}));
    let (ok, v) = f.review("empty");
    assert!(!ok, "{v}");
    assert!(!f.base.join("native-called").exists());
}
#[test]
fn final_gate_interrupt_and_sigterm_preserve_cancelled_state() {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        let f = Fixture::new();
        let marker = f.base.join("final-started");
        let count = f.base.join("count");
        let gate = f.base.join("final.py");
        let code = format!(
            "from pathlib import Path\nimport time\np=Path({count:?})\nn=int(p.read_text())+1 if p.exists() else 1\np.write_text(str(n))\nif n==2:\n Path({marker:?}).write_text('ready')\n time.sleep(60)\n"
        );
        std::fs::write(&gate, &code).unwrap();
        f.edit(|m| {
            m["execution"]["max_parallel"] = json!(1);
            m["tasks"].as_array_mut().unwrap().truncate(1);
            m["gates"][0]["argv"] = json!(["python3", gate]);
            m["gates"][0]["inputs"] =
                json!({gate.to_str().unwrap():forge_domain::digest(code.as_bytes())});
        });
        let child = f
            .command(&["release", "run"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        wait_file(&marker);
        unsafe {
            libc::kill(child.id() as i32, signal);
        }
        let v = wait_child(child);
        assert_eq!(v["run"]["status"], "cancelled", "{v}");
        let (_, status) = f.call(&["status"]);
        assert_eq!(status["run"]["status"], "cancelled");
    }
}
#[test]
fn gate_diagnostics_are_hashed_redacted_and_available() {
    let f = Fixture::new();
    let gate = f.base.join("diagnostic.py");
    let code = "import json,sys\nprint(json.dumps({'type':'diagnostic','token':{'nested':'sensitive-probe'},'password':['sensitive-probe'],'safe':'useful output'}))\nprint('failure detail',file=sys.stderr)\nsys.exit(7)\n";
    std::fs::write(&gate, code).unwrap();
    f.edit(|m| {
        m["tasks"].as_array_mut().unwrap().truncate(1);
        m["gates"][0]["argv"] = json!(["python3", gate]);
        m["gates"][0]["inputs"] =
            json!({gate.to_str().unwrap():forge_domain::digest(code.as_bytes())});
    });
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    let e = &v["run"]["tasks"]["a"]["evidence"][1];
    let detail: Value = serde_json::from_str(e["detail"].as_str().unwrap()).unwrap();
    let bytes = std::fs::read(f.state.join(detail["log"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(forge_domain::digest(&bytes), detail["log"]["sha256"]);
    let r: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(r["code"], 7);
    assert!(r["stdout"].as_str().unwrap().contains("useful output"));
    assert!(!String::from_utf8_lossy(&bytes).contains("sensitive-probe"));
    let (_, status) = f.call(&["status"]);
    assert_eq!(
        status["gate_diagnostics"][0]["diagnostic"]["stderr"],
        "failure detail"
    );
}

#[test]
fn integration_conflicts_preserve_unrelated_work() {
    use std::io::Write;
    let f = Fixture::new();
    let mut blob = Command::new("git")
        .arg("-C")
        .arg(&f.a)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    blob.stdin
        .take()
        .unwrap()
        .write_all(b"conflicting content")
        .unwrap();
    let out = blob.wait_with_output().unwrap();
    assert!(out.status.success());
    let hash = String::from_utf8(out.stdout).unwrap();
    let mut index = Command::new("git")
        .arg("-C")
        .arg(&f.a)
        .args(["update-index", "--index-info"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    index
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                "100644 {} 1\tconflict.txt\n100644 {} 2\tconflict.txt\n100644 {} 3\tconflict.txt\n",
                hash.trim(),
                hash.trim(),
                hash.trim()
            )
            .as_bytes(),
        )
        .unwrap();
    assert!(index.wait().unwrap().success());
    std::fs::write(f.a.join("conflict.txt"), "operator conflict preview").unwrap();
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["status"], "blocked");
    assert_eq!(v["run"]["tasks"]["a"]["attempts"], 0);
    assert_eq!(
        std::fs::read_to_string(f.a.join("preserved.txt")).unwrap(),
        "keep\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.a.join("conflict.txt")).unwrap(),
        "operator conflict preview"
    );
    let (_, v) = f.call(&["report"]);
    assert!(
        v["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "integration_conflict")
    );
}

#[test]
fn corrupt_or_missing_diagnostics_cannot_be_cached_artifacts() {
    let f = Fixture::new();
    f.edit(|m| {
        m["tasks"].as_array_mut().unwrap().truncate(1);
        m["tasks"][0]["acceptance"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"human","id":"review"}));
        m["gates"][0]["cache"] = json!({"enabled":true});
    });
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let detail: Value = serde_json::from_str(
        v["run"]["tasks"]["a"]["evidence"][1]["detail"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let log = f.state.join(detail["log"]["path"].as_str().unwrap());
    std::fs::write(&log, b"corrupt").unwrap();
    let (_, status) = f.call(&["status"]);
    assert_eq!(
        status["gate_diagnostics"][0]["diagnostic"]["available"],
        false
    );
    let (_, v) = f.call(&["resume"]);
    assert_eq!(v["run"]["status"], "awaiting_approval", "{v}");
    let detail: Value = serde_json::from_str(
        v["run"]["tasks"]["a"]["evidence"][1]["detail"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(detail["cache"], "miss");
    assert_eq!(detail["reason"], "missing_or_corrupt_diagnostic");
    std::fs::remove_file(f.state.join(detail["log"]["path"].as_str().unwrap())).unwrap();
    let (_, v) = f.call(&["resume"]);
    let detail: Value = serde_json::from_str(
        v["run"]["tasks"]["a"]["evidence"][1]["detail"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(detail["cache"], "miss");
}

#[test]
fn final_integration_rechecks_finished_source_and_serial_quality_matches() {
    for concurrency in [1, 2] {
        let f = Fixture::new();
        f.edit(|m|{
            m["execution"]["max_parallel"]=json!(concurrency);
            for task in m["tasks"].as_array_mut().unwrap() {task["executor"]["argv"]=json!(["python3","-c","from pathlib import Path;Path('result').write_text('same deterministic output')"]);}
        });
        let (ok, v) = f.call(&["release", "run"]);
        assert!(ok, "{v}");
        assert_eq!(
            std::fs::read_to_string(f.a.join("result")).unwrap(),
            "same deterministic output"
        );
        assert_eq!(
            std::fs::read_to_string(f.b.join("result")).unwrap(),
            "same deterministic output"
        );
        let (_, report) = f.call(&["report"]);
        assert_eq!(
            report["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["kind"] == "evaluated")
                .count(),
            4
        );
    }
    let f = Fixture::new();
    f.edit(|m| {
        m["tasks"][0]["executor"]["argv"] = json!([
            "python3",
            "-c",
            "from pathlib import Path;Path('result').write_text('initial')"
        ]);
        m["tasks"][0]["acceptance"] =
            json!([{"kind":"artifact","path":"result","sha256":forge_domain::digest(b"initial")}]);
        m["tasks"][1]["dependencies"] = json!(["a"]);
        m["tasks"][1]["executor"]["argv"] = json!([
            "python3",
            "-c",
            format!(
                "from pathlib import Path;Path({:?}).write_text('later source')",
                f.a.join("result").to_str().unwrap()
            )
        ]);
    });
    let (ok, v) = f.call(&["release", "run"]);
    assert!(!ok, "{v}");
    assert_eq!(v["run"]["tasks"]["a"]["status"], "blocked");
    assert_eq!(
        std::fs::read_to_string(f.a.join("result")).unwrap(),
        "later source"
    );
}

#[test]
fn crashed_coordinator_does_not_reconcile_live_owned_groups() {
    struct Groups(Vec<u32>);
    impl Drop for Groups {
        fn drop(&mut self) {
            for pid in &self.0 {
                unsafe {
                    libc::kill(-(*pid as i32), libc::SIGKILL);
                }
            }
        }
    }
    let f = Fixture::new();
    f.barrier(true);
    let mut child = f
        .command(&["release", "run"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_file(&f.a.join("heartbeat"));
    wait_file(&f.b.join("heartbeat"));
    let (_, v) = f.call(&["status"]);
    let pids = v["run"]["tasks"]
        .as_object()
        .unwrap()
        .values()
        .map(|t| t["pid"].as_u64().unwrap() as u32)
        .collect();
    let _groups = Groups(pids);
    child.kill().unwrap();
    child.wait().unwrap();
    let out = f
        .command(&["resume"])
        .args(["--reconcile", "disposable crash inspection"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        v["error"].as_str().unwrap().contains("orphan process"),
        "{v}"
    );
    assert_eq!(
        std::fs::read_to_string(f.a.join("preserved.txt")).unwrap(),
        "keep\n"
    );
}

#[test]
fn review_cancellation_interrupts_native_tools_before_postcheck() {
    for mode in ["cancel", "pause", "sigterm"] {
        let f = Fixture::new();
        let server = f.server("wait");
        let child = f
            .command(&["review", "run"])
            .arg("--app-server")
            .arg(server)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        wait_file(&f.base.join("native-called"));
        if mode == "sigterm" {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
        } else {
            assert!(f.call(&[mode]).0);
        }
        let v = wait_child(child);
        assert_eq!(v["review"]["status"], "blocked", "{v}");
        assert!(
            v["review"]["block_reason"]
                .as_str()
                .unwrap()
                .contains("cancelled")
        );
        assert!(
            f.base.join("native-called.interrupted").exists(),
            "server tool interruption must precede termination ({mode})"
        );
        assert_eq!(
            std::fs::read_to_string(f.a.join("preserved.txt")).unwrap(),
            "keep\n"
        );
        assert!(v["review"]["changed_scope"].as_array().unwrap().is_empty());
    }
}

#[test]
fn multi_root_review_reserves_every_native_invocation_without_double_counting_cache() {
    let f = Fixture::new();
    f.edit(|m| m["budget"] = json!({"max_attempts":1}));
    let (ok, v) = f.review("empty");
    assert!(!ok, "{v}");
    assert!(
        v["review"]["block_reason"]
            .as_str()
            .unwrap()
            .contains("attempt budget")
    );
    assert_eq!(v["review"]["call_ids"].as_array().unwrap().len(), 1);
    assert_eq!(v["review"]["usage"]["inputTokens"], 20);
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    assert_eq!(store.budget_attempts("m07", None).unwrap(), 1);
    let f = Fixture::new();
    let (ok, v) = f.review("empty");
    assert!(ok, "{v}");
    assert_eq!(v["review"]["call_ids"].as_array().unwrap().len(), 2);
    let store = forge_domain::Store::open(&f.state.join("state.sqlite"), true).unwrap();
    let usage = store.token_usage("m07", Some("_review")).unwrap();
    assert_eq!(usage["calls"], 2);
    assert_eq!(usage["input_tokens"], 40);
    assert_eq!(usage["cached_input_tokens"], 10);
    assert_eq!(usage["total_tokens"], 46);
    assert!(store.load("m07").unwrap().is_none());
}
