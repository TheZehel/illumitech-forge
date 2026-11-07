#![cfg(unix)]
//! Public CLI regressions with local fixture processes, never installed scanners/native models.
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Command, Output},
    time::{Duration, Instant},
};

struct Fixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    ops: PathBuf,
    script: PathBuf,
    request: PathBuf,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let repo = base.join("project with spaces");
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
                    .arg("-C")
                    .arg(&repo)
                    .args(args)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        std::fs::write(repo.join("source.txt"), "preserved source\n").unwrap();
        std::fs::write(repo.join(".gitignore"), "build-output/\n").unwrap();
        let f = Self {
            _temp: temp,
            ops: base.join("ops"),
            script: base.join("check.sh"),
            request: base.join("request.json"),
            repo,
        };
        std::fs::write(&f.script, body).unwrap();
        f.write(f.contract());
        f
    }
    fn contract(&self) -> Value {
        json!({"schema_version":1,"id":"065-local-qa","title":"Bounded fixture checks","level":"light","checks":(["QA-QUALITY","QA-SECRETS","QA-DEPENDENCIES"].iter().enumerate().map(|(i,id)|json!({"id":format!("check-{i}"),"coverage_id":id,"argv":["sh",self.script,"{repo}"],"timeout_seconds":5,"inputs":{self.script.to_str().unwrap():forge_domain::digest(&std::fs::read(&self.script).unwrap())}})).collect::<Vec<_>>())})
    }
    fn write(&self, value: Value) {
        std::fs::write(&self.request, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    fn command(&self, verb: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forge"));
        cmd.args(["--json", "modules", "qa", verb]);
        if verb == "prepare" {
            cmd.arg(&self.request);
        } else if verb != "discover" {
            cmd.arg("065-local-qa");
        }
        cmd.arg("--root").arg(&self.repo);
        if verb != "discover" {
            cmd.arg("--ops-root").arg(&self.ops);
        }
        cmd
    }
    fn invoke(&self, verb: &str, args: &[&str]) -> (i32, Value) {
        decode(self.command(verb).args(args).output().unwrap())
    }
    fn prepare(&self) -> String {
        let (code, v) = self.invoke("prepare", &["--apply"]);
        assert_eq!(code, 0, "{v}");
        assert_eq!(v["status"], "prepared");
        v["scope_digest"].as_str().unwrap().into()
    }
    fn start(&self, digest: &str) -> (i32, Value) {
        self.invoke("start", &["--approve-scope", digest])
    }
    fn dir(&self) -> PathBuf {
        self.ops.join("qa/065-local-qa")
    }
}
fn decode(out: Output) -> (i32, Value) {
    let value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: stdout={}, stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), value)
}
#[test]
fn preview_discovery_and_exact_once_observed_coverage() {
    let f = Fixture::new("printf 'observed fixture\\n'\n");
    std::fs::write(f.repo.join("Cargo.toml"), "[package]\nname='fixture'\n").unwrap();
    let (code, v) = f.invoke("discover", &[]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["executed"], false);
    assert_eq!(v["suggestions"][0]["source"], "Cargo.toml");
    assert_eq!(v["suggestions"][0]["availability"], "unknown");
    assert!(
        v["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["status"] == "not_run")
    );
    let before = forge_domain::revision(&f.repo).unwrap();
    let (code, preview) = f.invoke("prepare", &[]);
    assert_eq!(code, 0, "{preview}");
    assert_eq!(preview["status"], "preview");
    assert_eq!(preview["checks"][0]["status"], "not_run");
    assert!(!f.ops.exists());
    let digest = f.prepare();
    assert_eq!(preview["scope_digest"], digest);
    let immutable = std::fs::read(f.dir().join("candidate.json")).unwrap();
    let (code, bad) = f.start("wrong");
    assert_eq!(code, 2, "{bad}");
    assert!(!f.dir().join("intent.json").exists());
    let (code, result) = f.start(&digest);
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["status"], "passed");
    for check in result["checks"].as_array().unwrap() {
        assert_eq!(check["status"], "passed");
        let path = check["evidence"]["path"].as_str().unwrap();
        assert_eq!(
            check["evidence"]["sha256"],
            forge_domain::digest(&std::fs::read(path).unwrap())
        );
        assert_eq!(check["process"]["code"], 0);
    }
    assert!(
        result["coverage"].as_array().unwrap()[..3]
            .iter()
            .all(|c| c["status"] == "passed")
    );
    assert!(
        result["coverage"].as_array().unwrap()[3..]
            .iter()
            .all(|c| c["status"] == "not_requested")
    );
    let retained_before = retained_snapshot(&f.ops);
    let (code, status) = f.invoke("status", &[]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(status["status"], "passed");
    assert_eq!(status["continuity"]["progress"]["completed"], 3);
    assert_eq!(status["continuity"]["authority"], "none");
    assert_eq!(
        retained_before,
        retained_snapshot(&f.ops),
        "status must not modify operational files"
    );
    assert_eq!(f.start(&digest).0, 2);
    assert_eq!(
        immutable,
        std::fs::read(f.dir().join("candidate.json")).unwrap()
    );
    assert_eq!(before, forge_domain::revision(&f.repo).unwrap());
    // Changing retained evidence is detected, not silently accepted or rewritten.
    std::fs::write(f.dir().join("check-00.json"), "{}").unwrap();
    assert_eq!(f.invoke("status", &[]).0, 2);
}
#[test]
fn coverage_gaps_absent_executables_and_all_checks_required() {
    for (kind, expected, coverage_status) in [
        ("gap", "degraded", "not_run"),
        ("missing", "degraded", "unavailable"),
        ("multiple", "failed", "failed"),
    ] {
        let f = Fixture::new("exit 0\n");
        let mut r = f.contract();
        if kind == "gap" {
            r["checks"].as_array_mut().unwrap().truncate(1);
        }
        if kind == "missing" {
            r["checks"][0]["argv"] = json!(["forge-fixture-no-such-qa-executable"]);
        }
        if kind == "multiple" {
            let mut second = r["checks"][0].clone();
            second["id"] = json!("second-quality");
            second["argv"] = json!(["sh", "-c", "exit 7"]);
            r["checks"].as_array_mut().unwrap().push(second);
        }
        f.write(r);
        let digest = f.prepare();
        let (code, v) = f.start(&digest);
        assert_eq!(code, 2, "{v}");
        assert_eq!(v["status"], expected, "{v}");
        let row = if kind == "gap" { 1 } else { 0 };
        assert_eq!(v["coverage"][row]["status"], coverage_status, "{v}");
        let (code, status) = f.invoke("status", &[]);
        assert_eq!(code, 2);
        assert_eq!(
            status["diagnostic"]["code"],
            if expected == "failed" {
                "validation_failed"
            } else {
                "coverage_incomplete"
            }
        );
    }
}
#[test]
fn source_and_external_hash_are_revalidated_before_spawn_and_by_status() {
    for what in ["source", "input", "contract", "candidate"] {
        let f = Fixture::new("exit 0\n");
        let digest = f.prepare();
        match what {
            "source" => std::fs::write(f.repo.join("source.txt"), "operator edit").unwrap(),
            "input" => std::fs::write(&f.script, "exit 99\n").unwrap(),
            "contract" => std::fs::write(f.dir().join("request.json"), "{}").unwrap(),
            _ => {
                let mut c: Value =
                    serde_json::from_slice(&std::fs::read(f.dir().join("candidate.json")).unwrap())
                        .unwrap();
                c["evaluator_sha256"] = json!("f".repeat(64));
                std::fs::write(
                    f.dir().join("candidate.json"),
                    serde_json::to_vec(&c).unwrap(),
                )
                .unwrap();
            }
        }
        assert_eq!(f.start(&digest).0, 2, "{what}");
        assert!(!f.dir().join("intent.json").exists());
        let (code, status) = f.invoke("status", &[]);
        assert_eq!(code, 2, "{what}: {status}");
        assert_eq!(
            status.get("effective_status").unwrap_or(&status["status"]),
            "blocked"
        );
    }
}
#[test]
fn strict_requests_internal_paths_traversal_and_symlinks_are_rejected() {
    let f = Fixture::new("exit 0\n");
    for change in [
        "schema",
        "unknown",
        "unknown-check",
        "empty",
        "no-inputs",
        "coverage",
        "depth",
        "timeout",
        "duplicate",
        "too-many",
        "internal",
        "traversal",
    ] {
        let mut r = f.contract();
        match change {
            "schema" => r["schema_version"] = json!(2),
            "unknown" => r["publish"] = json!(true),
            "unknown-check" => r["checks"][0]["shell"] = json!(true),
            "empty" => r["checks"] = json!([]),
            "no-inputs" => r["checks"][0]["inputs"] = json!({}),
            "coverage" => r["checks"][0]["coverage_id"] = json!("QA-IMAGINARY"),
            "depth" => r["checks"][0]["coverage_id"] = json!("QA-AUTH"),
            "timeout" => r["checks"][0]["timeout_seconds"] = json!(601),
            "duplicate" => r["checks"][1]["id"] = r["checks"][0]["id"].clone(),
            "too-many" => {
                r["checks"] = json!(
                    (0..33)
                        .map(|i| {
                            let mut c = r["checks"][0].clone();
                            c["id"] = json!(format!("c{i}"));
                            c
                        })
                        .collect::<Vec<_>>()
                )
            }
            "internal" => {
                r["checks"][0]["inputs"] = json!({f.repo.join("source.txt").to_str().unwrap():forge_domain::digest(b"preserved source\n")})
            }
            _ => {
                r["checks"][0]["inputs"] = json!({format!("{}/../check.sh", f.repo.display()):forge_domain::digest(b"exit 0\n")})
            }
        }
        f.write(r);
        let (code, v) = f.invoke("prepare", &["--apply"]);
        assert_eq!(code, 2, "{change}: {v}");
        assert!(!f.ops.exists());
    }
    f.write(f.contract());
    let internal = f.repo.join("ops");
    let out = f
        .command("prepare")
        .arg("--apply")
        .arg("--ops-root")
        .arg(&internal)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!internal.exists());
    let link = f.repo.parent().unwrap().join("linked-input");
    std::os::unix::fs::symlink(&f.script, &link).unwrap();
    let mut r = f.contract();
    r["checks"][0]["inputs"] = json!({link.to_str().unwrap():forge_domain::digest(b"exit 0\n")});
    f.write(r);
    assert_eq!(f.invoke("prepare", &["--apply"]).0, 2);
    f.write(f.contract());
    std::os::unix::fs::symlink(f.repo.parent().unwrap().join("outside"), &f.ops).unwrap();
    assert_eq!(f.invoke("prepare", &["--apply"]).0, 2);
}
#[test]
fn bounded_timeout_output_limit_and_redacted_evidence() {
    for (script, expected) in [
        ("sleep 30\n", "timeout"),
        ("yes x\n", "output_limit"),
        ("printf 'password=fixture-only-value\\n'\n", "passed"),
    ] {
        let f = Fixture::new(script);
        let mut r = f.contract();
        r["checks"].as_array_mut().unwrap().truncate(1);
        r["checks"][0]["timeout_seconds"] = json!(1);
        f.write(r);
        let digest = f.prepare();
        let began = Instant::now();
        let (code, v) = f.start(&digest);
        assert_eq!(code, 2, "{v}"); // Remaining coverage is explicitly missing, even for clean exit.
        assert!(began.elapsed() < Duration::from_secs(10));
        assert_eq!(v["checks"][0]["status"], expected, "{v}");
        assert_ne!(v["status"], "passed");
        if expected == "passed" {
            let bytes = std::fs::read_to_string(f.dir().join("check-00.json")).unwrap();
            assert!(!bytes.contains("fixture-only-value"));
            assert!(bytes.contains("REDACTED"));
        }
    }
}
#[test]
fn sarif_findings_errors_invalid_and_clean_output_are_observed() {
    for kind in ["clean", "findings", "error", "invalid"] {
        let mut sarif =
            json!({"version":"2.1.0","runs":[{"tool":{"driver":{"name":"fixture"}},"results":[]}]});
        if kind == "findings" {
            sarif["runs"][0]["results"] = json!([{"ruleId":"fixture-rule","message":{"text":"fixture finding"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"source.txt"},"region":{"startLine":1}}}]}]);
        }
        if kind == "error" {
            sarif["runs"][0]["invocations"] = json!([{"executionSuccessful":false,"toolExecutionNotifications":[{"level":"error","message":{"text":"incomplete"}}]}]);
        }
        let output = if kind == "invalid" {
            "not SARIF".into()
        } else {
            serde_json::to_string(&sarif).unwrap()
        };
        let f = Fixture::new(&format!("cat <<'SARIF'\n{output}\nSARIF\n"));
        let mut r = f.contract();
        r["checks"][0]["format"] = json!("sarif");
        f.write(r);
        let digest = f.prepare();
        let (code, v) = f.start(&digest);
        assert_eq!(code, if kind == "clean" { 0 } else { 2 }, "{kind}: {v}");
        assert_eq!(
            v["status"],
            if kind == "clean" { "passed" } else { "failed" },
            "{v}"
        );
        assert_eq!(v["checks"][0]["process"]["code"], 0);
        if kind == "findings" {
            assert_eq!(
                v["checks"][0]["sarif"]["findings"][0]["rule"],
                "fixture-rule"
            );
        }
        if kind == "error" {
            assert!(
                !v["checks"][0]["sarif"]["scanner_errors"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(f.invoke("status", &[]).0, code);
    }
}
#[test]
fn source_mutation_blocks_without_restoring_and_ignored_builds_are_allowed() {
    let f = Fixture::new("printf 'changed by reviewed command' > source.txt\n");
    let digest = f.prepare();
    let (code, v) = f.start(&digest);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["status"], "blocked");
    assert_eq!(v["checks"][0]["observed_status"], "passed");
    assert_ne!(
        v["checks"][0]["source_before"],
        v["checks"][0]["source_after"]
    );
    assert_eq!(v["checks"][1]["status"], "not_run");
    assert_eq!(
        std::fs::read_to_string(f.repo.join("source.txt")).unwrap(),
        "changed by reviewed command"
    );
    assert_eq!(f.invoke("status", &[]).1["status"], "stale");
    let f = Fixture::new("mkdir -p build-output\nprintf 'artifact' > build-output/result\n");
    let digest = f.prepare();
    assert_eq!(f.start(&digest).0, 0);
    assert!(f.repo.join("build-output/result").exists());
}
#[test]
fn interrupted_intent_never_replays_and_lock_is_shared_by_worktree() {
    let f = Fixture::new("printf started > \"$1/../started\"\nsleep 30\n");
    let digest = f.prepare();
    let child = f
        .command("start")
        .args(["--approve-scope", &digest])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let marker = f.repo.parent().unwrap().join("started");
    let began = Instant::now();
    while !marker.exists() {
        assert!(began.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
    }
    let (code, status) = f.invoke("status", &[]);
    assert_eq!(code, 2, "{status}");
    assert_eq!(status["checks"][0]["status"], "blocked");
    assert_eq!(f.start(&digest).0, 2);
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let (code, result) = decode(child.wait_with_output().unwrap());
    assert_eq!(code, 2, "{result}");
    assert_eq!(result["checks"][0]["status"], "cancelled");
    assert_eq!(result["checks"][1]["status"], "not_run");
    assert_eq!(f.start(&digest).0, 2);
    // An uncompleted durable intent, even without any process, is never replayable.
    let f = Fixture::new("exit 0\n");
    let digest = f.prepare();
    std::fs::write(
        f.dir().join("intent.json"),
        serde_json::to_vec(&json!({"scope_digest":digest})).unwrap(),
    )
    .unwrap();
    assert_eq!(f.start(&digest).0, 2);
    assert_eq!(f.invoke("status", &[]).1["status"], "blocked");
    assert!(!f.dir().join("result.json").exists());
}

#[test]
fn argv_substitution_is_whole_argument_and_never_implicit_shell_code() {
    let f = Fixture::new(
        "test \"$1\" = \"$PWD\" || exit 10\ntest \"$2\" = '{repo}/literal' || exit 11\ntest \"$3\" = '$(touch injected)' || exit 12\ntest \"$#\" -eq 4 && test \"$4\" = '' || exit 13\n",
    );
    let mut r = f.contract();
    for check in r["checks"].as_array_mut().unwrap() {
        check["argv"].as_array_mut().unwrap().extend([
            json!("{repo}/literal"),
            json!("$(touch injected)"),
            json!(""),
        ]);
    }
    f.write(r);
    let before = forge_domain::revision(&f.repo).unwrap();
    let digest = f.prepare();
    let (code, v) = f.start(&digest);
    assert_eq!(code, 0, "{v}");
    assert!(!f.repo.join("injected").exists());
    assert_eq!(before, forge_domain::revision(&f.repo).unwrap());
}

#[test]
fn modifying_pinned_evaluator_during_a_check_blocks_further_spawns() {
    let f = Fixture::new("printf changed > \"$1/../policy\"\n");
    let policy = f.repo.parent().unwrap().join("policy");
    std::fs::write(&policy, "reviewed policy").unwrap();
    let mut r = f.contract();
    r["checks"][0]["inputs"][policy.to_str().unwrap()] =
        json!(forge_domain::digest(b"reviewed policy"));
    f.write(r);
    let digest = f.prepare();
    let (code, v) = f.start(&digest);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["status"], "blocked");
    assert_eq!(v["checks"][0]["observed_status"], "passed");
    assert_eq!(v["checks"][1]["status"], "not_run");
    assert_eq!(std::fs::read_to_string(policy).unwrap(), "changed");
    assert_eq!(f.invoke("status", &[]).0, 2);
}

fn retained_snapshot(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, String> {
    let mut snapshot = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            snapshot.extend(retained_snapshot(&path));
        } else {
            snapshot.insert(
                path.clone(),
                forge_domain::digest(&std::fs::read(path).unwrap()),
            );
        }
    }
    snapshot
}
#[test]
fn symlinked_state_is_rejected_before_any_check() {
    let f = Fixture::new("exit 0\n");
    let digest = f.prepare();
    std::os::unix::fs::symlink(&f.repo, f.dir().join("state")).unwrap();
    assert_eq!(f.start(&digest).0, 2);
    assert_eq!(f.invoke("status", &[]).0, 2);
    assert!(!f.dir().join("intent.json").exists());
    assert!(!f.repo.join("state.sqlite").exists());
}
