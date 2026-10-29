#![cfg(windows)]
use illumitech_forge_adapters::{clients, process};
use std::{path::Path, time::Duration};
use tokio::sync::watch;
fn python(code: &str) -> Vec<String> {
    vec!["python.exe".into(), "-c".into(), code.into()]
}
async fn wait_pid(path: &Path) -> u32 {
    for _ in 0..250 {
        if let Ok(s) = std::fs::read_to_string(path)
            && let Ok(pid) = s.parse()
        {
            return pid;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("descendant did not start");
}
async fn assert_dead(pid: u32) {
    for _ in 0..250 {
        if !process::pid_alive(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("descendant {pid} survived");
}
#[tokio::test]
async fn job_ends_descendants_on_timeout_cancel_abort_and_normal_exit() {
    for mode in ["timeout", "cancel", "abort", "exit"] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("space café 日本");
        std::fs::create_dir(&root).unwrap();
        let code = format!(
            "import subprocess,sys,time,pathlib; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); pathlib.Path('pid').write_text(str(p.pid)); {}",
            if mode == "exit" {
                "time.sleep(1)"
            } else {
                "time.sleep(60)"
            }
        );
        let (tx, rx) = watch::channel(false);
        let cwd = root.clone();
        let task = tokio::spawn(async move {
            process::run(
                &python(&code),
                &cwd,
                if mode == "timeout" { 2 } else { 20 },
                rx,
            )
            .await
        });
        let pid = wait_pid(&root.join("pid")).await;
        if mode == "cancel" {
            tx.send(true).unwrap();
        }
        if mode == "abort" {
            task.abort();
            let _ = task.await;
        } else {
            let result = task.await.unwrap().unwrap();
            assert_eq!(
                result.reason,
                match mode {
                    "cancel" => "cancelled",
                    "exit" => "exited",
                    _ => "timeout",
                }
            );
        }
        assert_dead(pid).await;
    }
}
#[tokio::test]
async fn native_argv_crlf_and_output_bound() {
    let dir = tempfile::tempdir().unwrap();
    let values = [
        "",
        "a b",
        "日本 café",
        "a\"b",
        "C:\\space path\\",
        "& | %PATH% $HOME ;",
        "line\r\nnext",
    ];
    let mut argv = python("import sys,json; print(json.dumps(sys.argv[1:]))");
    argv.extend(values.iter().map(|s| s.to_string()));
    let (_tx, rx) = watch::channel(false);
    let r = process::run(&argv, dir.path(), 10, rx).await.unwrap();
    assert!(r.passed(), "{r:?}");
    assert_eq!(
        serde_json::from_str::<Vec<String>>(&r.stdout).unwrap(),
        values
    );
    let (_tx, rx) = watch::channel(false);
    let r = process::run(
        &python("import sys; sys.stdout.write('x'*3000000)"),
        dir.path(),
        10,
        rx,
    )
    .await
    .unwrap();
    assert!(!r.passed());
    assert!(r.stdout.len() <= process::OUTPUT_LIMIT);
}
#[test]
fn native_discovery_and_pid_checks() {
    assert!(process::pid_alive(std::process::id()));
    assert!(!process::pid_alive(u32::MAX));
    assert!(
        clients::find_executable("git")
            .unwrap()
            .extension()
            .unwrap()
            .eq_ignore_ascii_case("exe")
    );
    assert!(clients::resolve_command("C:\\fake.cmd", Path::new(".")).is_err());
}

#[tokio::test]
async fn service_child_drains_without_capture_and_owns_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let (_tx, rx) = watch::channel(false);
    let result = tokio::time::timeout(Duration::from_secs(10), process::service_child(
        &python("import sys; sys.stdout.write('x'*3000000); sys.stderr.write('y'*3000000); sys.exit(7)"),
        dir.path(), rx,
    )).await.unwrap().unwrap();
    assert_eq!(result, Some(7));
    for abort in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let (tx, rx) = watch::channel(false);
        let task = tokio::spawn(async move {
            process::service_child(&python("import subprocess,sys,time,pathlib; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); pathlib.Path('pid').write_text(str(p.pid)); time.sleep(60)"), &root, rx).await
        });
        let pid = wait_pid(&dir.path().join("pid")).await;
        if abort {
            task.abort();
            let _ = task.await;
        } else {
            tx.send(true).unwrap();
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(5), task)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap(),
                None
            );
        }
        assert_dead(pid).await;
    }
}

#[tokio::test]
async fn observation_failure_closes_job_and_stdin_is_eof() {
    let dir = tempfile::tempdir().unwrap();
    let (_tx, rx) = watch::channel(false);
    let mut pid = 0;
    let result = process::run_observed(
        &python("import time; time.sleep(60)"),
        dir.path(),
        10,
        rx,
        |child| {
            pid = child;
            anyhow::bail!("injected recording failure")
        },
    )
    .await;
    assert!(result.is_err());
    assert_ne!(pid, 0);
    assert_dead(pid).await;
    let (_tx, rx) = watch::channel(false);
    let result = process::run(
        &python("import sys; assert sys.stdin.read() == ''; print('eof'); sys.stderr.write('error stream')"),
        dir.path(), 10, rx,
    ).await.unwrap();
    assert!(result.passed(), "{result:?}");
    assert_eq!(result.stdout, "eof");
    assert_eq!(result.stderr, "error stream");
    let (_tx, rx) = watch::channel(false);
    let result = process::run(
        &python("import sys; sys.stderr.write('x'*3000000)"),
        dir.path(),
        10,
        rx,
    )
    .await
    .unwrap();
    assert!(!result.passed());
    assert!(result.stderr.len() <= process::OUTPUT_LIMIT);
}
