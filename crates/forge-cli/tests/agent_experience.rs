//! Public discovery and diagnostics must not create execution state or run inference.
use serde_json::Value;
use std::{fs, process::Command};
fn call(args: &[&str], cwd: &std::path::Path) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&out.stderr))),
    )
}
#[test]
fn discovery_and_invalid_requests_have_no_execution_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let (code, catalog) = call(
        &["--json", "contracts", "--command", "modules feature start"],
        tmp.path(),
    );
    assert_eq!(code, 0);
    assert_eq!(catalog["contracts"]["continuity"], 1);
    assert_eq!(
        catalog["command_catalog"]["commands"][0]["effect"],
        "execution_requires_review"
    );
    let (code, error) = call(
        &["--json", "contracts", "--command", "not a command"],
        tmp.path(),
    );
    assert_eq!(code, 2);
    assert_eq!(error["error_code"], "operation_blocked");
    assert_eq!(error["diagnostic"]["code"], "unknown_command");
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "contracts", "--nonexistent"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty()); // Existing parse-error contract is preserved.
}
#[test]
#[cfg(unix)]
fn doctor_distinguishes_missing_client_without_writing_or_inference() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let bin = base.join("bin");
    fs::create_dir(&bin).unwrap();
    let repo = base.join("project with spaces");
    fs::create_dir(&repo).unwrap();
    let git = Command::new("which").arg("git").output().unwrap();
    symlink(
        String::from_utf8(git.stdout).unwrap().trim(),
        bin.join("git"),
    )
    .unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    let marker = base.join("unexpected");
    fs::write(
        bin.join("forge"),
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(bin.join("forge"), fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["--json", "workflow", "doctor", "--root"])
        .arg(&repo)
        .env("PATH", &bin)
        .output()
        .unwrap();
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(2));
    let checks = value["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|c| c["id"] == "client" && c["status"] == "missing")
    );
    assert_eq!(value["native_inference_calls"], 0);
    assert_eq!(value["diagnostic"]["code"], "prerequisite_unavailable");
    assert!(!marker.exists());
    assert!(!repo.join(".forge-state").exists());
}
