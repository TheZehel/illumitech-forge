//! Native onboarding through the real engine. No provider or platform acceptance claims.
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn call(args: &[&str], cwd: &Path, success: bool) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("--json")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert_eq!(
        out.status.success(),
        success,
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
#[cfg(unix)]
fn doctor_preserves_invoked_alias_identity_and_marks_partial_scans() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let target = base.join("illumitech-forge");
    fs::copy(env!("CARGO_BIN_EXE_forge"), &target).unwrap();
    let alias = base.join("forge");
    symlink(&target, &alias).unwrap();
    let old = base.join("old");
    let empty = base.join("empty");
    fs::create_dir(&old).unwrap();
    fs::create_dir(&empty).unwrap();
    let marker = base.join("executed");
    fs::write(
        old.join("forge"),
        format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(old.join("forge"), fs::Permissions::from_mode(0o700)).unwrap();
    for truncated in [false, true] {
        let mut dirs = if truncated {
            vec![empty.clone(); 128]
        } else {
            Vec::new()
        };
        dirs.push(old.clone());
        let out = Command::new(&alias)
            .args(["--json", "installation", "doctor"])
            .env("PATH", std::env::join_paths(dirs).unwrap())
            .output()
            .unwrap();
        assert!(out.status.success());
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        let diag = &v["path_diagnostics"];
        assert_eq!(diag["truncated"], truncated);
        assert_eq!(
            diag["commands"][0]["status"],
            if truncated { "incomplete" } else { "attention" }
        );
        if !truncated {
            assert_eq!(
                diag["commands"][0]["selected"]["path_matches_running"],
                false
            );
        }
        assert!(!marker.exists());
    }
}

fn create(base: &Path) -> Value {
    call(
        &[
            "demo",
            "create",
            base.join("example with spaces").to_str().unwrap(),
        ],
        base,
        true,
    )
}

fn next(value: &Value, name: &str, base: &Path, success: bool) -> Value {
    let args: Vec<&str> = value["commands"][name]
        .as_array()
        .unwrap()
        .iter()
        .skip(1)
        .map(|v| v.as_str().unwrap())
        .collect();
    call(&args, base, success)
}

#[test]
fn native_demo_previews_are_read_only_and_real_execution_preserves_files() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let demo = create(&base);
    let project = Path::new(demo["repo"].as_str().unwrap());
    let state = Path::new(demo["state_dir"].as_str().unwrap());
    assert_eq!(demo["executed"], false);
    next(&demo, "inspect", &base, true);
    next(&demo, "dry_run", &base, true);
    assert!(!state.exists());
    assert!(!project.join("hello.txt").exists());
    assert!(!project.join(".ai-memory.toml").exists());
    let result = next(&demo, "run", &base, true);
    assert_eq!(result["run"]["status"], "passed");
    assert_eq!(result["token_usage"]["calls"], 0);
    assert_eq!(fs::read(project.join("preserved.txt")).unwrap(), b"keep\n");
    assert!(state.join("state.sqlite").exists());
    next(&demo, "report", &base, true);
    fs::write(project.join("changed.txt"), b"new revision").unwrap();
    let stale = next(&demo, "report", &base, false);
    assert_eq!(stale["run"]["status"], "passed"); // Historical snapshot, not current acceptance.
    assert_eq!(stale["effective_status"], "blocked");
    assert_eq!(stale["evidence_current"], false);
}

#[test]
fn native_demo_refuses_existing_directories_and_preserves_changed_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let output = base.join("example with spaces");
    let demo = create(&base);
    fs::write(output.join("keep-extra.txt"), b"preserve").unwrap();
    call(&["demo", "create", output.to_str().unwrap()], &base, false);
    assert_eq!(
        fs::read(output.join("keep-extra.txt")).unwrap(),
        b"preserve"
    );
    let project = Path::new(demo["repo"].as_str().unwrap());
    call(&["demo", "write"], project, true);
    fs::write(project.join("hello.txt"), b"changed").unwrap();
    call(&["demo", "write"], project, false);
    call(&["demo", "verify", project.to_str().unwrap()], &base, false);
    assert_eq!(fs::read(project.join("hello.txt")).unwrap(), b"changed");
}

#[test]
fn demo_cannot_pass_when_declared_evaluator_hash_is_invalid() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let demo = create(&base);
    let path = Path::new(demo["manifest"].as_str().unwrap());
    let mut manifest: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let inputs = manifest["gates"][0]["inputs"].as_object_mut().unwrap();
    for digest in inputs.values_mut() {
        *digest = Value::String("0".repeat(64));
    }
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let result = next(&demo, "run", &base, false);
    assert_ne!(result["run"]["status"], "passed");
}

#[test]
fn init_memory_is_opt_in_and_existing_routing_is_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    fs::create_dir(base.join(".git")).unwrap();
    let result = call(&["init"], &base, true);
    assert_eq!(result["memory_config"], "not_requested");
    assert!(!base.join(".ai-memory.toml").exists());
    let content = b"workspace = \"existing\"\ncustom = true\n";
    fs::write(base.join(".ai-memory.toml"), content).unwrap();
    assert_eq!(call(&["init"], &base, true)["memory_config"], "preserved");
    call(&["init", "--memory-workspace", "another"], &base, false);
    assert_eq!(fs::read(base.join(".ai-memory.toml")).unwrap(), content);
    let other = base.join("other");
    fs::create_dir_all(other.join(".git")).unwrap();
    call(&["init", "--memory-workspace", "my-team"], &other, true);
    assert_eq!(
        call(&["init", "--memory-workspace", "my-team"], &other, true)["memory_config"],
        "preserved"
    );
    assert!(
        fs::read_to_string(other.join(".ai-memory.toml"))
            .unwrap()
            .contains("my-team")
    );
}

#[test]
fn init_preflights_existing_scaffolding_before_creating_memory() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    fs::create_dir(base.join(".git")).unwrap();
    fs::create_dir(base.join(".forge")).unwrap();
    fs::write(base.join(".forge/README.md"), "my existing file").unwrap();
    call(&["init", "--memory-workspace", "my-team"], &base, false);
    assert!(!base.join(".ai-memory.toml").exists());
    assert_eq!(
        fs::read_to_string(base.join(".forge/README.md")).unwrap(),
        "my existing file"
    );
}

#[test]
fn demo_rejects_inherited_git_routing_before_creating_files() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let destination = base.join("demo");
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "demo", "create"])
        .arg(&destination)
        .env("GIT_DIR", base.join("unrelated-git"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!destination.exists());
    assert!(String::from_utf8_lossy(&out.stdout).contains("GIT_DIR"));
}

#[cfg(unix)]
#[test]
fn demo_preserves_symlink_destinations_and_does_not_run_ambient_git_hooks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let existing = base.join("existing");
    fs::create_dir(&existing).unwrap();
    let link = base.join("link");
    symlink(&existing, &link).unwrap();
    call(&["demo", "create", link.to_str().unwrap()], &base, false);
    assert!(link.is_symlink());
    assert_eq!(fs::read_dir(&existing).unwrap().count(), 0);
    let hooks = base.join("hooks");
    fs::create_dir(&hooks).unwrap();
    let marker = base.join("executed");
    let hook = hooks.join("pre-commit");
    fs::write(
        &hook,
        format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "demo", "create"])
        .arg(base.join("demo"))
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", hooks)
        .env("GIT_TEMPLATE_DIR", base.join("hooks"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!marker.exists());
    assert!(!base.join("demo/app/.git/hooks/pre-commit").exists());
}

#[cfg(unix)]
#[test]
fn doctor_inspects_both_names_and_never_executes_unknown_candidates() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let first = base.join("first");
    let second = base.join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    let marker = base.join("executed");
    fs::write(
        first.join("forge"),
        format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(first.join("forge"), fs::Permissions::from_mode(0o700)).unwrap();
    symlink(env!("CARGO_BIN_EXE_forge"), second.join("forge")).unwrap();
    symlink(env!("CARGO_BIN_EXE_forge"), second.join("illumitech-forge")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "installation", "doctor"])
        .env(
            "PATH",
            std::env::join_paths([first.clone(), second]).unwrap(),
        )
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let commands = value["path_diagnostics"]["commands"].as_array().unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(
        commands[0]["selected"]["path"],
        first.join("forge").to_str().unwrap()
    );
    assert_eq!(commands[0]["selected"]["path_matches_running"], false);
    assert!(commands[0]["selected"]["version"].is_null());
    assert!(commands[0]["warning"].is_string());
    assert_eq!(commands[1]["selected"]["path_matches_running"], true);
    assert!(!marker.exists());
}
