use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn call(root: &Path, input: &str, target: Option<&str>, code: i32) -> Value {
    let path = root.join("changes.json");
    fs::write(&path, input).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_forge"));
    command
        .current_dir(root)
        .args(["--json", "release", "version", "--changes"])
        .arg(&path);
    if let Some(target) = target {
        command.args(["--target", target]);
    }
    let out = command.output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), input);
    assert!(!root.join(".forge-state").exists());
    serde_json::from_slice(&out.stdout).unwrap()
}
fn changes(kind: &str) -> String {
    json!({"schema_version":1,"base_version":"1.3.0","changes":[{"id":"C1","kind":kind,"summary":"Reviewed change"}]}).to_string()
}
#[test]
fn real_cli_plans_versions_and_enforces_the_target_without_mutation() {
    let t = tempfile::tempdir().unwrap();
    for (kind, bump, next) in [
        ("fix", "patch", "1.3.1"),
        ("feature", "minor", "1.4.0"),
        ("breaking", "major", "2.0.0"),
    ] {
        let input = changes(kind);
        let result = call(t.path(), &input, None, 0);
        assert_eq!(result["required_bump"], bump);
        assert_eq!(result["proposed_version"], next);
        assert_eq!(
            result["change_set_sha256"],
            forge_domain::digest(input.as_bytes())
        );
        assert_eq!(result["mutation"], false);
        assert_eq!(result["publication_authorized"], false);
        assert_eq!(call(t.path(), &input, Some(next), 0)["status"], "passed");
        assert_eq!(
            call(t.path(), &input, Some("1.3.0"), 2)["status"],
            "blocked"
        );
    }
    assert_eq!(
        call(t.path(), &changes("feature"), Some("1.3.1"), 2)["proposed_version"],
        "1.4.0"
    );
    assert_eq!(fs::read_dir(t.path()).unwrap().count(), 1);
}
#[test]
fn real_cli_refuses_ambiguous_invalid_or_oversize_declarations() {
    let t = tempfile::tempdir().unwrap();
    for input in [
        "{}".to_string(),
        changes("guess"),
        changes("fix").replace("\"schema_version\":1", "\"schema_version\":2"),
        "x".repeat(1024 * 1024 + 1),
    ] {
        assert_eq!(call(t.path(), &input, None, 2)["status"], "blocked");
    }
    assert_eq!(
        call(t.path(), &changes("docs"), None, 0)["release_required"],
        false
    );
    assert_eq!(
        call(t.path(), &changes("docs"), Some("1.3.1"), 2)["status"],
        "blocked"
    );
    assert_eq!(
        call(t.path(), &changes("fix"), Some("v1.3.1"), 2)["status"],
        "blocked"
    );
}
