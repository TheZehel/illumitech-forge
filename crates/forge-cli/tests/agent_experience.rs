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
    assert_eq!(catalog["contracts"]["release_publication"], 1);
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

#[test]
#[cfg(unix)]
fn doctor_requires_a_git_revision_without_committing_or_running_inference() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let bin = base.join("bin");
    let repo = base.join("project");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(&repo).unwrap();
    let git = Command::new("which").arg("git").output().unwrap();
    let git = String::from_utf8(git.stdout).unwrap();
    let git = git.trim();
    symlink(git, bin.join("git")).unwrap();
    for name in ["forge", "illumitech-forge"] {
        symlink(env!("CARGO_BIN_EXE_forge"), bin.join(name)).unwrap();
    }
    let marker = base.join("inference-was-started");
    fs::write(
        bin.join("codex"),
        format!(
            "#!/bin/sh\ncase \"$*\" in\n'--version') echo 'codex-cli fixture';;\n'login status') echo 'Logged in using ChatGPT';;\n*) /usr/bin/touch '{}'; exit 1;;\nesac\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(bin.join("codex"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        Command::new(git)
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    fs::write(repo.join("preserved.txt"), b"keep\n").unwrap();
    let diagnose = || {
        let output = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(["--json", "workflow", "doctor", "--root"])
            .arg(&repo)
            .env("PATH", &bin)
            .output()
            .unwrap();
        (
            output.status.code(),
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        )
    };
    let (exit, value) = diagnose();
    assert_eq!(exit, Some(2));
    let revision = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "git_revision")
        .unwrap();
    assert_eq!(revision["status"], "missing");
    assert_eq!(revision["passed"], false);
    assert_eq!(revision["action"]["requires_review"], true);
    assert!(revision["action"]["argv"].is_null());
    assert!(
        revision["action"]["reason"]
            .as_str()
            .unwrap()
            .contains("initial Git commit")
    );
    assert!(
        !Command::new(git)
            .args(["rev-parse", "--verify", "HEAD"])
            .current_dir(&repo)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        Command::new(git)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--allow-empty",
                "-qm",
                "Initialize disposable fixture"
            ])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    let (exit, value) = diagnose();
    assert_eq!(exit, Some(0));
    assert_eq!(value["status"], "passed");
    assert!(
        value["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "git_revision" && c["passed"] == true)
    );
    assert_eq!(value["native_inference_calls"], 0);
    fs::remove_file(bin.join("git")).unwrap();
    let quoted_git = git.replace('\'', "'\\''");
    fs::write(
        bin.join("git"),
        format!(
            "#!/bin/sh\ncase \"$*\" in\n'rev-parse --verify --quiet HEAD^{{commit}}') exit 42;;\n*) exec '{quoted_git}' \"$@\";;\nesac\n"
        ),
    )
    .unwrap();
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o700)).unwrap();
    let (exit, value) = diagnose();
    assert_eq!(exit, Some(2));
    let revision = value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "git_revision")
        .unwrap();
    assert_eq!(revision["status"], "unverified");
    assert!(
        !revision["action"]["reason"]
            .as_str()
            .unwrap()
            .contains("initial Git commit")
    );
    assert!(!marker.exists());
    assert!(!repo.join(".forge-state").exists());
    assert!(!repo.join("specs").exists());
    assert_eq!(fs::read(repo.join("preserved.txt")).unwrap(), b"keep\n");
}
