mod common;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

// Readiness is a state transition, not a fixed sleep. Keep failed tests from leaving daemons alive.
struct ChildGuard(Child);
impl std::ops::Deref for ChildGuard {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}
impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            #[cfg(unix)]
            let _ = Command::new("kill")
                .args(["-TERM", &self.0.id().to_string()])
                .stderr(Stdio::null())
                .status();
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if self.0.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        ready(),
        "fixture did not reach the required state within 30 seconds"
    );
}

struct Rig {
    _dir: tempfile::TempDir,
    root: PathBuf,
    repo: PathBuf,
    state: PathBuf,
    manifest: PathBuf,
    bin: PathBuf,
}
fn git(root: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap().trim().into()
}
impl Rig {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // Git for Windows worktree creation rejects verbatim \? prefixes. TempDir
        // already supplies an absolute native path; Forge still canonicalizes at runtime.
        #[cfg(windows)]
        let root = dir.path().to_path_buf();
        #[cfg(not(windows))]
        let root = dir.path().canonicalize().unwrap();
        let base = root.join("base");
        std::fs::create_dir(&base).unwrap();
        git(&base, &["init", "-q", "-b", "main"]);
        git(&base, &["config", "user.name", "Human Fixture"]);
        git(&base, &["config", "user.email", "human@example.invalid"]);
        git(
            &base,
            &[
                "config",
                "core.hooksPath",
                root.join("no-hooks").to_str().unwrap(),
            ],
        );
        std::fs::write(base.join("output.txt"), "verified").unwrap();
        git(&base, &["add", "."]);
        git(&base, &["commit", "-qm", "Initial fixture"]);
        let repo = root.join("repo");
        git(
            &base,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "forge/pilot",
                repo.to_str().unwrap(),
            ],
        );
        let bare = root.join("remote.git");
        std::fs::create_dir(&bare).unwrap();
        git(&bare, &["init", "-q", "--bare"]);
        git(
            &repo,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/fixture/project.git",
            ],
        );
        let bin = root.join("bin");
        std::fs::create_dir(&bin).unwrap();
        // Transport fixture: keep real remote resolution intact, redirect only network operations.
        let real_git = forge_adapters::clients::find_executable("git").unwrap();
        let wrapper = format!(
            "#!/usr/bin/env python3\nimport sys,subprocess\na=sys.argv[1:]\nif a and a[0] in ('push','ls-remote') and 'origin' in a:a[a.index('origin')]={}\nsys.exit(subprocess.call([{},*a]))\n",
            serde_json::to_string(&bare).unwrap(),
            serde_json::to_string(&real_git).unwrap()
        );
        std::fs::write(bin.join("git"), wrapper).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(bin.join("git"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let fake = r#"#!/usr/bin/env python3
import sys,json,pathlib,subprocess
r=pathlib.Path(__file__).resolve().parent.parent;a=sys.argv[1:]
with (r/'gh-calls.jsonl').open('a') as f:f.write(json.dumps(a)+'\n')
if a[:2]==['repo','view']:print(json.dumps({'visibility':(r/'visibility').read_text() if (r/'visibility').exists() else 'PRIVATE'}))
elif a[:2]==['pr','list']:
 if (r/'block-list').exists():
  import time
  (r/'list-waiting').touch();time.sleep(60)
 print((r/'pr.json').read_text() if (r/'pr.json').exists() else '[]')
elif a[:2]==['pr','create']:
 if (r/'fail-create').exists():sys.exit(3)
 p={'url':'https://github.com/fixture/project/pull/1','state':'OPEN','isDraft':True,'baseRefName':'main','headRefOid':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()}
 (r/'pr.json').write_text(json.dumps([p]));print(p['url'])
elif a[:2]==['release','create']:
 assert pathlib.Path(a[a.index('--notes-file')+1]).is_absolute()
 assert all(pathlib.Path(x).is_absolute() for x in a[a.index('--notes-file')+2:])
 print('https://github.com/fixture/project/releases/tag/'+a[2])
else:sys.exit(4)
"#;
        std::fs::write(bin.join("gh"), fake).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(bin.join("gh"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        #[cfg(windows)]
        {
            common::native_shim(&bin, "git");
            common::native_shim(&bin, "gh");
        }
        let eval = root.join("evaluate.py");
        std::fs::write(&eval,"import pathlib,sys\nassert (pathlib.Path(sys.argv[1])/'output.txt').read_text()=='verified'\n").unwrap();
        let queue = root.join("queue");
        std::fs::create_dir(&queue).unwrap();
        let manifest = queue.join("pilot.yaml");
        let m = json!({"schema_version":1,"release_id":"pilot","repos":{"app":{"path":repo}},"tasks":[{"id":"work","repo":"app","description":"test","executor":{"kind":"command","argv":common::command("printf x >> executions.txt")},"max_attempts":1,"acceptance":[{"kind":"gate","id":"check"}]}],"gates":[{"id":"check","argv":[common::python(),eval,"{repo}"],"inputs":{eval.display().to_string():forge_domain::digest(&std::fs::read(&eval).unwrap())}}]});
        std::fs::write(&manifest, serde_json::to_vec(&m).unwrap()).unwrap();
        let state = root.join("state");
        Self {
            _dir: dir,
            root,
            repo,
            state,
            manifest,
            bin,
        }
    }
    fn change(&self, f: impl FnOnce(&mut Value)) {
        let mut m: Value = serde_json::from_slice(&std::fs::read(&self.manifest).unwrap()).unwrap();
        f(&mut m);
        std::fs::write(&self.manifest, serde_json::to_vec(&m).unwrap()).unwrap();
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x00000010);
        } // CREATE_NEW_CONSOLE: isolated native control-event tests

        c.args(["--json", "--state-dir", self.state.to_str().unwrap()])
            .args(args)
            .current_dir(&self.root);
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        c.env("PATH", std::env::join_paths(path).unwrap());
        c
    }
    fn call(&self, args: &[&str]) -> (bool, Value) {
        let o = self.command(args).output().unwrap();
        let v = serde_json::from_slice(&o.stdout).unwrap_or_else(|_| {
            panic!(
                "{} {}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )
        });
        (o.status.success(), v)
    }
    fn release(&self, action: &str) -> (bool, Value) {
        self.call(&["release", action, self.manifest.to_str().unwrap()])
    }
    fn daemon(&self) -> (bool, Value) {
        self.call(&[
            "daemon",
            "--releases-dir",
            self.manifest.parent().unwrap().to_str().unwrap(),
            "--once",
        ])
    }
    fn automate(&self, publish: bool) {
        self.change(|m| {m["automation"]=json!({"enabled":true});if publish {m["automation"]["publish"]=json!({"repository":"fixture/project","branch":"forge/pilot","base_branch":"main"});}});
    }
}

#[test]
fn daemon_opt_in_pause_restart_and_material_change() {
    let r = Rig::new();
    let (_, v) = r.daemon();
    assert_eq!(v["releases"][0]["status"], "disabled");
    assert!(!r.repo.join("executions.txt").exists());
    r.automate(false);
    assert!(r.call(&["daemon-control", "pause", "--reason", "review"]).0);
    assert!(r.daemon().0);
    assert!(!r.repo.join("executions.txt").exists());
    assert!(r.call(&["daemon-control", "resume"]).0);
    let (ok, v) = r.daemon();
    assert!(ok, "{v}");
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
    assert!(r.daemon().0);
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
    r.change(|m| m["tasks"][0]["description"] = json!("material change"));
    assert!(!r.daemon().0);
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
}

#[test]
fn daemon_dry_run_and_status_are_read_only_and_lock_is_exclusive() {
    let r = Rig::new();
    r.automate(false);
    assert!(
        r.call(&[
            "daemon",
            "--releases-dir",
            r.manifest.parent().unwrap().to_str().unwrap(),
            "--dry-run"
        ])
        .0
    );
    assert!(r.call(&["daemon-control", "status"]).0);
    assert!(!r.state.exists());
    let mut child = ChildGuard(
        r.command(&[
            "daemon",
            "--releases-dir",
            r.manifest.parent().unwrap().to_str().unwrap(),
            "--interval-seconds",
            "30",
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap(),
    );
    wait_until(|| {
        r.state.join("state.sqlite").exists()
            && forge_domain::Store::open(&r.state.join("state.sqlite"), true)
                .ok()
                .and_then(|s| s.load("pilot").ok().flatten())
                .is_some_and(|r| r.status == "passed")
    });
    let (ok, v) = r.daemon();
    assert!(!ok);
    assert!(v["error"].as_str().unwrap().contains("another daemon"));
    common::interrupt(child.id());
    assert!(child.wait().unwrap().success());
    assert!(r.daemon().0);
}

#[test]
fn daemon_failure_requires_explicit_intervention() {
    let r = Rig::new();
    r.automate(false);
    r.change(|m| {
        m["tasks"][0]["executor"]["argv"] =
            json!(common::command("printf x >> executions.txt; exit 1"))
    });
    assert!(!r.daemon().0);
    assert!(!r.daemon().0);
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
    let store = forge_domain::Store::open(&r.state.join("state.sqlite"), true).unwrap();
    assert_eq!(
        store.load("pilot").unwrap().unwrap().tasks["work"].attempts,
        1
    );
}

#[test]
fn draft_pr_recovers_failed_create_without_duplicate_commits_and_preserves_human_author() {
    let r = Rig::new();
    r.automate(true);
    std::fs::write(r.root.join("fail-create"), "").unwrap();
    assert!(!r.daemon().0);
    let head = git(&r.repo, &["rev-parse", "HEAD"]);
    assert_eq!(
        git(&r.repo, &["log", "-1", "--format=%an"]),
        "Human Fixture"
    );
    std::fs::remove_file(r.root.join("fail-create")).unwrap();
    let (ok, v) = r.release("draft-pr");
    assert!(ok, "{v}");
    assert_eq!(git(&r.repo, &["rev-parse", "HEAD"]), head);
    let (ok, v) = r.release("draft-pr");
    assert!(ok, "{v}");
    assert_eq!(v["reused"], true);
    assert_eq!(git(&r.repo, &["rev-parse", "HEAD"]), head);
    assert!(!git(&r.repo, &["log", "-1", "--format=%B"]).contains("Co-authored-by"));
}

#[test]
fn draft_pr_refuses_agent_authorship_and_base_checkout() {
    let r = Rig::new();
    r.automate(true);
    git(&r.repo, &["config", "user.name", "Codex"]);
    let (ok, v) = r.daemon();
    assert!(!ok);
    assert!(v.to_string().contains("human Git identity"));
    assert!(!r.repo.join("executions.txt").exists());
}

#[test]
fn generic_release_publish_is_memory_optional_and_resolves_relative_paths() {
    let r = Rig::new();
    r.change(|m| {
        m["tasks"][0]["executor"]["argv"] = json!(common::command("true"));
        m["tasks"][0]["acceptance"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"human","id":"owner"}));
    });
    git(&r.repo, &["tag", "v2.3.4"]);
    git(
        &r.repo,
        &[
            "push",
            r.root.join("remote.git").to_str().unwrap(),
            "refs/tags/v2.3.4",
        ],
    );
    let (_, v) = r.release("run");
    assert_eq!(v["run"]["status"], "awaiting_approval");
    let fingerprint = v["run"]["tasks"]["work"]["revision"]["fingerprint"]
        .as_str()
        .unwrap();
    assert!(
        r.call(&[
            "release",
            "approve",
            r.manifest.to_str().unwrap(),
            "--task",
            "work",
            "--gate",
            "owner",
            "--reviewer",
            "Human Fixture",
            "--fingerprint",
            fingerprint
        ])
        .0
    );
    assert!(r.call(&["resume", r.manifest.to_str().unwrap()]).0);
    let assets = r.root.join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::write(assets.join("widget.bin"), "example").unwrap();
    std::fs::write(
        assets.join("SHA256SUMS"),
        format!("{}  widget.bin\n", forge_domain::digest(b"example")),
    )
    .unwrap();
    std::fs::write(r.root.join("notes.md"), "Human release notes").unwrap();
    let args = [
        "release",
        "publish",
        r.manifest.to_str().unwrap(),
        "--repository",
        "fixture/project",
        "--tag",
        "v2.3.4",
        "--assets",
        "assets",
        "--notes",
        "notes.md",
    ];
    let mut dry = args.to_vec();
    dry.push("--dry-run");
    let (ok, v) = r.call(&dry);
    assert!(ok, "{v}");
    assert!(!r.root.join("gh-calls.jsonl").exists());
    let (ok, v) = r.call(&args);
    assert!(ok, "{v}");
    assert_eq!(v["published"], true);
    assert_eq!(r.call(&dry).1["expected_visibility"], "PRIVATE");
    let mut public = args.to_vec();
    public.extend(["--expected-visibility", "PUBLIC"]);
    // Both mismatch directions refuse before creating a release.
    assert!(!r.call(&public).0);
    std::fs::write(r.root.join("visibility"), "PUBLIC").unwrap();
    let before = std::fs::read_to_string(r.root.join("gh-calls.jsonl")).unwrap();
    assert!(!r.call(&args).0);
    let after = std::fs::read_to_string(r.root.join("gh-calls.jsonl")).unwrap();
    assert!(!after[before.len()..].contains("create"));
    assert!(r.call(&public).0);
    assert!(
        !std::fs::read_to_string(r.root.join("gh-calls.jsonl"))
            .unwrap()
            .contains("edit")
    );
    // Public selection still requires the exact approved revision.
    std::fs::write(r.repo.join("changed.txt"), "new revision").unwrap();
    assert!(!r.call(&public).0);
    std::fs::remove_file(r.repo.join("changed.txt")).unwrap();
    std::fs::write(assets.join("widget.bin"), "tampered").unwrap();
    assert!(!r.call(&args).0);
}

#[test]
fn spec_import_ids_survive_line_insertions_and_completed_tasks_are_not_evidence() {
    let r = Rig::new();
    let spec = r.repo.join("specs/001-pilot");
    std::fs::create_dir_all(&spec).unwrap();
    for name in ["spec.md", "plan.md"] {
        std::fs::write(spec.join(name), "# Context").unwrap();
    }
    let content = "- [ ] T001 Implement feature\n- [x] T002 old completion\n- [ ] Legacy task without permanent ID\n";
    std::fs::write(spec.join("tasks.md"), content).unwrap();
    let args = [
        "release",
        "import-spec",
        "--root",
        r.repo.to_str().unwrap(),
        "--spec",
        "001-pilot",
        "--release-id",
        "imported",
    ];
    let (ok, a) = r.call(&args);
    assert!(ok, "{a}");
    assert_eq!(a["candidate"]["tasks"].as_array().unwrap().len(), 2);
    std::fs::write(spec.join("tasks.md"), format!("# Header\n\n{content}")).unwrap();
    let (ok, b) = r.call(&args);
    assert!(ok, "{b}");
    assert_eq!(a["candidate"]["tasks"], b["candidate"]["tasks"]);
    assert_eq!(b["legacy_state_imported"], false);
    assert_eq!(b["candidate"]["automation"]["enabled"], false);
    let legacy = r.root.join("legacy.yaml");
    std::fs::write(&legacy,serde_json::to_vec(&json!({"release_id":"legacy","repos":{"app":{"path":r.repo,"speckit":{"specs":["001-pilot"]}}},"tasks":[]})).unwrap()).unwrap();
    let (ok, c) = r.call(&["release", "import-loop", legacy.to_str().unwrap()]);
    assert!(ok, "{c}");
    assert_eq!(c["candidate"]["tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn priority_respects_dependencies_before_numeric_order() {
    let r = Rig::new();
    r.change(|m| {
        let mut t = m["tasks"][0].clone();
        m["tasks"][0]["priority"] = json!(20);
        t["id"] = json!("early");
        t["priority"] = json!(1);
        m["tasks"].as_array_mut().unwrap().push(t.clone());
        t["id"] = json!("dependent");
        t["priority"] = json!(-5);
        t["dependencies"] = json!(["work"]);
        m["tasks"].as_array_mut().unwrap().push(t);
    });
    let (ok, v) = r.release("inspect");
    assert!(ok, "{v}");
    assert_eq!(v["order"], json!(["early", "work", "dependent"]));
}

#[test]
fn daemon_sigterm_cancels_active_process_and_persists_stop() {
    let r = Rig::new();
    r.automate(false);
    r.change(|m| {
        m["tasks"][0]["executor"]["argv"] =
            json!(common::command("echo $$ > child.pid; exec sleep 60"))
    });
    let mut child = ChildGuard(
        r.command(&[
            "daemon",
            "--releases-dir",
            r.manifest.parent().unwrap().to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap(),
    );
    wait_until(|| r.repo.join("child.pid").exists());
    let pid = std::fs::read_to_string(r.repo.join("child.pid")).unwrap();
    common::interrupt(child.id());
    let mut exited = false;
    for _ in 0..200 {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    if !exited {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    assert!(exited, "daemon did not stop after TERM");
    assert!(!forge_adapters::process::pid_alive(
        pid.trim().parse().unwrap()
    ));
    let store = forge_domain::Store::open(&r.state.join("state.sqlite"), true).unwrap();
    assert_eq!(store.load("pilot").unwrap().unwrap().status, "cancelled");
    assert_eq!(
        store.metadata("daemon_heartbeat").unwrap().unwrap()["running"],
        false
    );
}

#[test]
fn journal_recovers_commit_before_evaluation_without_rerunning_task() {
    let r = Rig::new();
    r.automate(true);
    assert!(r.release("run").0);
    let store = forge_domain::Store::open(&r.state.join("state.sqlite"), false).unwrap();
    let run = store.load("pilot").unwrap().unwrap();
    let parent = git(&r.repo, &["rev-parse", "HEAD"]);
    git(&r.repo, &["add", "--all"]);
    let tree = git(&r.repo, &["write-tree"]);
    store.set_metadata("draft-pr:pilot", &json!({"phase":"committing", "manifest_hash":run.manifest_hash, "parent":parent,"tree":tree})).unwrap();
    git(&r.repo, &["commit", "-qm", "feat: pilot"]);
    let head = git(&r.repo, &["rev-parse", "HEAD"]);
    let (ok, v) = r.release("draft-pr");
    assert!(ok, "{v}");
    assert_eq!(git(&r.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
    assert_eq!(store.load("pilot").unwrap().unwrap().status, "passed");
}

#[test]
fn failed_postcommit_gate_prevents_push_and_pr() {
    let r = Rig::new();
    r.automate(true);
    let eval = r.root.join("evaluate.py");
    std::fs::write(&eval, "import subprocess,sys\nassert subprocess.check_output(['git','-C',sys.argv[1],'rev-list','--count','HEAD']).strip()==b'1'\n").unwrap();
    r.change(|m| {
        m["gates"][0]["inputs"] =
            json!({eval.display().to_string():forge_domain::digest(&std::fs::read(&eval).unwrap())})
    });
    let (ok, v) = r.daemon();
    assert!(!ok, "{v}");
    assert_eq!(git(&r.repo, &["rev-list", "--count", "HEAD"]), "2");
    assert!(!r.root.join("gh-calls.jsonl").exists());
    assert_eq!(
        forge_domain::Store::open(&r.state.join("state.sqlite"), true)
            .unwrap()
            .load("pilot")
            .unwrap()
            .unwrap()
            .status,
        "blocked"
    );
    assert!(!r.release("draft-pr").0);
}

#[test]
fn staged_sensitive_path_is_refused_even_when_worktree_is_clean_relative_to_index() {
    let r = Rig::new();
    r.automate(true);
    r.change(|m| {
        m["tasks"][0]["executor"]["argv"] = json!(common::command(
            "printf fixture > .env.local; git add .env.local"
        ))
    });
    let (ok, v) = r.daemon();
    assert!(!ok, "{v}");
    assert!(v.to_string().contains("sensitive file path"));
    assert_eq!(git(&r.repo, &["rev-list", "--count", "HEAD"]), "1");
    assert!(!r.root.join("gh-calls.jsonl").exists());
}

#[test]
fn postcommit_human_approval_is_invalidated_and_can_be_reviewed_without_reexecution() {
    let r = Rig::new();
    r.automate(true);
    r.change(|m| {
        m["tasks"][0]["acceptance"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"human","id":"owner"}))
    });
    let (_, v) = r.release("run");
    let approve = |fingerprint: &str| {
        assert!(
            r.call(&[
                "release",
                "approve",
                r.manifest.to_str().unwrap(),
                "--task",
                "work",
                "--gate",
                "owner",
                "--reviewer",
                "Human Fixture",
                "--fingerprint",
                fingerprint
            ])
            .0
        );
        assert!(r.call(&["resume", r.manifest.to_str().unwrap()]).0);
    };
    let original = v["run"]["tasks"]["work"]["revision"]["fingerprint"]
        .as_str()
        .unwrap();
    approve(original);
    assert!(!r.release("draft-pr").0);
    assert!(!r.root.join("gh-calls.jsonl").exists());
    let store = forge_domain::Store::open(&r.state.join("state.sqlite"), true).unwrap();
    let run = store.load("pilot").unwrap().unwrap();
    assert_eq!(run.status, "awaiting_approval");
    let updated = &run.tasks["work"].revision.as_ref().unwrap().fingerprint;
    assert_ne!(updated, original);
    assert!(!store.approved(&run, "work", "owner", original).unwrap());
    approve(updated);
    let (ok, v) = r.release("draft-pr");
    assert!(ok, "{v}");
    assert_eq!(
        std::fs::read_to_string(r.repo.join("executions.txt")).unwrap(),
        "x"
    );
    assert_eq!(git(&r.repo, &["rev-list", "--count", "HEAD"]), "2");
}

#[test]
fn effective_pushurl_and_url_rewrites_must_match_declared_target() {
    for (key, value) in [
        (
            "remote.origin.pushurl",
            "https://github.com/other/project.git",
        ),
        (
            "url.https://github.com/other/.pushInsteadOf",
            "https://github.com/fixture/",
        ),
        (
            "url.https://github.com/other/.insteadOf",
            "https://github.com/fixture/",
        ),
    ] {
        let r = Rig::new();
        r.automate(true);
        git(&r.repo, &["config", key, value]);
        let (ok, v) = r.daemon();
        assert!(!ok, "{v}");
        assert!(v.to_string().contains("effective Git remote"));
        assert!(!r.repo.join("executions.txt").exists());
    }
}

#[test]
fn sigterm_during_publication_stops_before_push_or_create() {
    let r = Rig::new();
    r.automate(true);
    std::fs::write(r.root.join("block-list"), "").unwrap();
    let mut child = ChildGuard(
        r.command(&[
            "daemon",
            "--releases-dir",
            r.manifest.parent().unwrap().to_str().unwrap(),
            "--once",
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap(),
    );
    wait_until(|| r.root.join("list-waiting").exists());
    common::interrupt(child.id());
    let mut exited = false;
    for _ in 0..200 {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    if !exited {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    assert!(exited);
    assert!(!r.root.join("pr.json").exists());
    assert!(git(&r.root.join("remote.git"), &["for-each-ref", "refs/heads"]).is_empty());
    assert!(
        forge_domain::Store::open(&r.state.join("state.sqlite"), true)
            .unwrap()
            .metadata("daemon-block:pilot")
            .unwrap()
            .is_some()
    );
}

#[test]
fn spec_metadata_preserves_dependencies_priority_and_rejects_dangling_ids() {
    let r = Rig::new();
    let spec = r.repo.join("specs/001-pilot");
    std::fs::create_dir_all(&spec).unwrap();
    let tasks = "- [ ] T001 Prepare <!-- forge: {\"priority\":20} -->\n- [ ] T002 Implement <!-- forge: {\"dependencies\":[\"T001\"],\"priority\":1} -->\n";
    std::fs::write(spec.join("tasks.md"), tasks.replace('\n', "\r\n")).unwrap();
    let args = [
        "release",
        "import-spec",
        "--root",
        r.repo.to_str().unwrap(),
        "--spec",
        "001-pilot",
        "--release-id",
        "imported",
    ];
    let (ok, v) = r.call(&args);
    assert!(ok, "{v}");
    assert_eq!(v["candidate"]["tasks"][0]["priority"], 20);
    assert_eq!(
        v["candidate"]["tasks"][1]["dependencies"],
        json!(["app-001-pilot-T001"])
    );
    std::fs::write(
        spec.join("tasks.md"),
        tasks.replace("[\"T001\"]", "[\"T999\"]"),
    )
    .unwrap();
    assert!(!r.call(&args).0);
}

#[cfg(unix)]
#[test]
fn sigterm_during_git_clean_filter_stops_publication() {
    let r = Rig::new();
    r.automate(true);
    std::fs::write(
        r.repo.join(".gitattributes"),
        "executions.txt filter=waiting\n",
    )
    .unwrap();
    git(&r.repo, &["add", ".gitattributes"]);
    git(&r.repo, &["commit", "-qm", "Fixture filter contract"]);
    let filter = r.root.join("filter.py");
    std::fs::write(&filter,"import pathlib,sys,time\npathlib.Path(__file__).with_name('filter-waiting').touch()\ntime.sleep(60)\nprint(sys.stdin.read(),end='')\n").unwrap();
    git(
        &r.repo,
        &[
            "config",
            "filter.waiting.clean",
            &format!("python3 '{}'", filter.display()),
        ],
    );
    git(&r.repo, &["config", "filter.waiting.required", "true"]);
    let mut child = ChildGuard(
        r.command(&[
            "daemon",
            "--releases-dir",
            r.manifest.parent().unwrap().to_str().unwrap(),
            "--once",
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap(),
    );
    wait_until(|| r.root.join("filter-waiting").exists());
    common::interrupt(child.id());
    wait_until(|| child.try_wait().unwrap().is_some());
    assert!(!r.root.join("gh-calls.jsonl").exists());
    assert!(git(&r.root.join("remote.git"), &["for-each-ref", "refs/heads"]).is_empty());
}

#[test]
fn public_visibility_does_not_bypass_human_approval() {
    let r = Rig::new();
    r.change(|m| {
        m["tasks"][0]["executor"]["argv"] = json!(common::command("true"));
        m["tasks"][0]["acceptance"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"human","id":"owner"}));
    });
    assert_eq!(r.release("run").1["run"]["status"], "awaiting_approval");
    assert!(
        !r.call(&[
            "release",
            "publish",
            r.manifest.to_str().unwrap(),
            "--repository",
            "fixture/project",
            "--tag",
            "v1.0.0",
            "--assets",
            "assets",
            "--notes",
            "notes.md",
            "--expected-visibility",
            "PUBLIC"
        ])
        .0
    );
    assert!(!r.root.join("gh-calls.jsonl").exists());
}
