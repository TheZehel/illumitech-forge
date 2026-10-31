//! Product integration regressions. Disposable command gates are not native acceptance.
use forge_domain::digest;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn call(state: &Path, args: &[&str], ok: bool) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_illumitech-forge"))
        .args(["--json", "--state-dir"])
        .arg(state)
        .args(args)
        .output()
        .unwrap();
    assert_eq!(
        out.status.success(),
        ok,
        "args={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["tool_version"], env!("CARGO_PKG_VERSION"));
    value
}
#[test]
fn v1_end_to_end_with_external_gates() {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let state = base.join("state");
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    fs::write(root.join("preserved.txt"), "keep").unwrap();
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "disposable fixture"
            ])
            .status()
            .unwrap()
            .success()
    );
    let ops = base.join("operations");
    call(
        &state,
        &[
            "feature",
            "new",
            "--root",
            root.to_str().unwrap(),
            "--id",
            "001-demand",
            "--title",
            "Synthetic demand",
            "--ops-root",
            ops.to_str().unwrap(),
            "--apply",
        ],
        true,
    );
    assert!(
        !state.exists(),
        "guided creation must not open execution state"
    );
    let gate = base.join("independent_gate.py");
    fs::write(&gate, "from pathlib import Path\nimport sys\nroot=Path(sys.argv[1])\nassert (root/'result.txt').read_text() == '42'\nassert (root/'preserved.txt').read_text() == 'keep'\n").unwrap();
    let manifest = base.join("release.json");
    let body = json!({"schema_version":1,"release_id":"v1-integration","repos":{"app":{"path":root,"specs":["001-demand"]}},"context":{"profile":"bounded","budget_tokens":8192},"budget":{"actions":"off"},"tasks":[{"id":"work","repo":"app","description":"Write exactly 42 while preserving existing content.","executor":{"kind":"command","argv":["python3","-c","from pathlib import Path; Path('result.txt').write_text('42')"]},"acceptance":[{"kind":"gate","id":"independent"}]}],"gates":[{"id":"independent","argv":["python3",gate,"{repo}"],"inputs":{gate.to_str().unwrap():digest(&fs::read(&gate).unwrap())}}]});
    fs::write(&manifest, serde_json::to_vec(&body).unwrap()).unwrap();
    let m = manifest.to_str().unwrap();
    for args in [
        vec!["release", "inspect", m],
        vec!["release", "run", m, "--dry-run"],
        vec![
            "context",
            "plan",
            m,
            "--task",
            "work",
            "--budget-tokens",
            "8192",
        ],
        vec!["review", "plan", m],
    ] {
        call(&state, &args, true);
        assert!(
            !state.exists(),
            "previews must not create operational state"
        );
    }
    let execution = call(&state, &["release", "run", m], true);
    assert_eq!(execution["run"]["status"], "passed");
    let report = call(&state, &["report", m], true);
    assert_eq!(report["run"]["status"], "passed");
    let snapshot = base.join("snapshot");
    call(
        &state,
        &["state", "snapshot", "--output", snapshot.to_str().unwrap()],
        true,
    );
    let restored = base.join("restored");
    call(
        &state,
        &[
            "state",
            "restore",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--target-state-dir",
            restored.to_str().unwrap(),
            "--apply",
        ],
        true,
    );
    assert_eq!(call(&restored, &["report", m], true)["run"], report["run"]);
    // The immutable evaluator pin is still authoritative after a successful restore.
    fs::write(&gate, "raise SystemExit('changed evaluator')\n").unwrap();
    let blocked = call(&restored, &["release", "run", m], false);
    assert_eq!(blocked["status"], "blocked");
    assert_eq!(blocked["error_code"], "operation_blocked");
    assert_eq!(
        fs::read_to_string(root.join("preserved.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn v1_distribution_inventory_and_support_matrix() {
    let t = tempfile::tempdir().unwrap();
    let state = t.path().join("absent");
    let contract = call(&state, &["contracts"], true);
    assert!(!state.exists());
    assert_eq!(contract["contracts"]["manifest"], 1);
    assert_eq!(contract["contracts"]["state_sqlite"], 1);
    assert!(
        contract["platforms"]["windows"]
            .as_str()
            .unwrap()
            .contains("experimental")
    );
    assert!(
        contract["platforms"]["linux"]
            .as_str()
            .unwrap()
            .contains("blocked")
    );
    assert!(
        contract["candidate"]
            .as_str()
            .unwrap()
            .contains("unpublished")
    );
    assert!(
        contract["publication"]
            .as_str()
            .unwrap()
            .contains("separate explicit authorization")
    );
}
