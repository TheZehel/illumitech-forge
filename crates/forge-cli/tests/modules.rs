use serde_json::{Value, json};
use std::process::Command;

#[test]
fn qa_plan_is_revision_bound_and_does_not_create_state_or_run_tasks() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let repo = root.join("repo");
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
    let gate = root.join("gate.sh");
    std::fs::write(&gate, "exit 99\n").unwrap();
    let manifest = root.join("release.json");
    std::fs::write(&manifest, serde_json::to_vec(&json!({
        "schema_version":1,"release_id":"qa-plan-test",
        "repos":{"app":{"path":repo}},
        "tasks":[{"id":"task","repo":"app","description":"Must not execute","executor":{"kind":"command","argv":["nonexistent-qa-plan-command"]},"acceptance":[{"kind":"gate","id":"proof"}]}],
        "gates":[{"id":"proof","argv":["sh",gate],"inputs":{gate.to_str().unwrap():forge_domain::digest(b"exit 99\n")}}]
    })).unwrap()).unwrap();
    let state = root.join("state-must-not-exist");
    for level in ["light", "standard", "intensive"] {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .arg("--json")
            .arg("--state-dir")
            .arg(&state)
            .args(["modules", "qa-plan"])
            .arg(&manifest)
            .args(["--level", level])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["checks_executed"], 0);
        assert_eq!(value["acceptance"], "unchanged");
        assert_eq!(value["native_call"], false);
        assert_eq!(value["level"], level);
        assert!(value["review_plan"]["source"]["app"].is_object());
        assert!(!state.exists());
    }
    let invalid = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["modules", "qa-plan"])
        .arg(&manifest)
        .args(["--level", "unlimited"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(!state.exists());
    // A modified evaluator must not produce a usable plan for the old contract.
    std::fs::write(&gate, "exit 0\n").unwrap();
    let stale = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--json")
        .arg("--state-dir")
        .arg(&state)
        .args(["modules", "qa-plan"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(!stale.status.success());
    let value: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(value["status"], "blocked");
    assert!(!state.exists());

    let proposal = root.join("feature.json");
    let mut feature = json!({"schema_version":1,"id":"001-feature","title":"Feature example","summary":"Deliver an observable behavior", "tasks":[{"id":"implement","title":"Implement behavior","dependencies":[],"acceptance":[{"id":"AC1","description":"Behavior matches expectations","command":["nonexistent-feature-check"]}]}]});
    std::fs::write(&proposal, serde_json::to_vec(&feature).unwrap()).unwrap();
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_forge"))
            .arg("--json")
            .args(["modules", "feature-plan"])
            .arg(&proposal)
            .arg("--root")
            .arg(&repo)
            .output()
            .unwrap()
    };
    let output = invoke();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["module"], "feature");
    assert_eq!(value["execution"], "not_started");
    assert_eq!(value["native_call"], false);
    assert_eq!(value["candidate"]["order"], json!(["implement"]));
    assert_eq!(value["candidate"]["acceptance"], "not_evaluated");
    feature["tasks"][0]["acceptance"] = json!([]);
    std::fs::write(&proposal, serde_json::to_vec(&feature).unwrap()).unwrap();
    assert!(!invoke().status.success());
    assert!(!state.exists());
}

#[test]
fn catalogue_keeps_future_modules_explicit() {
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "modules", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let modules = value["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 4);
    assert_eq!(modules[0]["id"], "feature");
    assert_eq!(modules[1]["id"], "qa");
    assert_eq!(modules[0]["availability"], "execution");
    assert!(modules[2..].iter().all(|m| m["availability"] == "roadmap"));
}
