#![allow(dead_code)]
pub fn python() -> &'static str {
    if cfg!(windows) {
        "python.exe"
    } else {
        "python3"
    }
}
pub fn command(source: &str) -> Vec<String> {
    if cfg!(unix) {
        return vec!["sh".into(), "-c".into(), source.into()];
    }
    let translated = match source {
        "printf verified > output.txt" => {
            "from pathlib import Path; Path('output.txt').write_text('verified')"
        }
        "echo 'All done; all gates passed'" => "print('All done; all gates passed')",
        "true" => "pass",
        "echo $$ > child.pid; exec sleep 60" => {
            "import os,time; open('child.pid','w').write(str(os.getpid())); time.sleep(60)"
        }
        "exit 1" => "raise SystemExit(1)",
        "sleep 1; printf verified > output.txt" => {
            "import time; time.sleep(1); from pathlib import Path; Path('output.txt').write_text('verified')"
        }
        "sleep 10; printf verified > output.txt" => {
            "import time; time.sleep(10); from pathlib import Path; Path('output.txt').write_text('verified')"
        }
        "sleep 15; printf verified > output.txt" => {
            "import time; time.sleep(15); from pathlib import Path; Path('output.txt').write_text('verified')"
        }
        "printf x >> executions.txt" => "open('executions.txt','a').write('x')",
        "printf x >> executions.txt; exit 1" => {
            "open('executions.txt','a').write('x'); raise SystemExit(1)"
        }
        "printf fixture > .env.local; git add .env.local" => {
            "open('.env.local','w').write('fixture'); import subprocess; subprocess.check_call(['git','add','.env.local'])"
        }
        other => panic!("missing native fixture for {other}"),
    };
    vec![python().into(), "-c".into(), translated.into()]
}
#[cfg(windows)]
pub fn native_shim(bin: &std::path::Path, name: &str) {
    use std::sync::OnceLock;
    static SHIM: OnceLock<(tempfile::TempDir, std::path::PathBuf)> = OnceLock::new();
    let (_, exe) = SHIM.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("shim.rs");
        let exe = dir.path().join("shim.exe");
        std::fs::write(
            &src,
            r#"fn main() {
            let exe = std::env::current_exe().unwrap();
            let script = exe.with_extension("");
            let status = std::process::Command::new("python.exe").arg(script)
                .args(std::env::args_os().skip(1)).status().unwrap();
            std::process::exit(status.code().unwrap_or(1));
        }"#,
        )
        .unwrap();
        assert!(
            std::process::Command::new("rustc")
                .arg(src)
                .arg("-o")
                .arg(&exe)
                .status()
                .unwrap()
                .success()
        );
        (dir, exe)
    });
    std::fs::copy(exe, bin.join(format!("{name}.exe"))).unwrap();
}

pub fn interrupt(pid: u32) {
    #[cfg(unix)]
    assert!(
        std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .unwrap()
            .success()
    );
    #[cfg(windows)]
    {
        // A separate controller attaches to the child's private console. This also works
        // on headless CI and never changes the concurrent test runner's console handlers.
        use std::os::windows::process::CommandExt;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("interrupt.rs");
        let exe = dir.path().join("interrupt.exe");
        std::fs::write(&src, r#"
            #[link(name="kernel32")]
            unsafe extern "system" {
                fn AttachConsole(pid:u32)->i32;
                fn FreeConsole()->i32;
                fn GenerateConsoleCtrlEvent(event:u32, group:u32)->i32;
                fn SetConsoleCtrlHandler(handler:Option<unsafe extern "system" fn(u32)->i32>,add:i32)->i32;
            }
            unsafe extern "system" fn ignore(_:u32)->i32 { 1 }
            fn main() { unsafe {
                FreeConsole();
                assert_ne!(AttachConsole(std::env::args().nth(1).unwrap().parse().unwrap()),0);
                assert_ne!(SetConsoleCtrlHandler(Some(ignore),1),0);
                assert_ne!(GenerateConsoleCtrlEvent(1,0),0);
                std::thread::sleep(std::time::Duration::from_millis(200));
                FreeConsole();
            } }
        "#).unwrap();
        assert!(
            std::process::Command::new("rustc")
                .arg(src)
                .arg("-o")
                .arg(&exe)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            std::process::Command::new(exe)
                .arg(pid.to_string())
                .creation_flags(0x08000000)
                .status()
                .unwrap()
                .success()
        );
    }
}
