//! Windows 11 containment. No breakaway flags; descendants inherit the Job.
use anyhow::{Result, ensure};
use std::{
    ffi::OsStr,
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    os::windows::{ffi::OsStrExt, process::ExitStatusExt},
    path::Path,
    process::ExitStatus,
    ptr::{null, null_mut},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    Security::SECURITY_ATTRIBUTES,
    System::{JobObjects::*, Pipes::CreatePipe, Threading::*},
};

pub(super) struct Group(OwnedHandle);
impl Group {
    pub(super) fn new() -> Result<Self> {
        // SAFETY: null security attributes/name create a private, non-inheritable job.
        unsafe {
            let handle = CreateJobObjectW(null(), null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let job = Self(OwnedHandle::from_raw_handle(handle));
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job.0.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(job)
        }
    }
    pub(super) fn terminate(&self, _force: bool) {
        // SAFETY: valid owned Job handle; Windows has no Unix SIGTERM equivalent.
        unsafe {
            TerminateJobObject(self.0.as_raw_handle(), 1);
        }
    }
}
impl Drop for Group {
    fn drop(&mut self) {
        self.terminate(true);
    }
}

pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return true;
    }
    // Fail closed on access denied/unknown errors: reconciliation must not restart a live owner.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return GetLastError() != ERROR_INVALID_PARAMETER;
        }
        let handle = OwnedHandle::from_raw_handle(handle);
        WaitForSingleObject(handle.as_raw_handle(), 0) != WAIT_OBJECT_0
    }
}

/// Own only the process handle. The surrounding Group owns tree termination,
/// including errors before observation and cancellation of the supervisor future.
pub(super) struct Child {
    handle: OwnedHandle,
    pid: u32,
    pub(super) stdout: Option<PipeReader>,
    pub(super) stderr: Option<PipeReader>,
}
// Anonymous pipes report ERROR_BROKEN_PIPE when the final writer closes.
// Treat that native condition as EOF, while preserving every other I/O error.
pub(super) struct PipeReader(tokio::fs::File);
impl tokio::io::AsyncRead for PipeReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match tokio::io::AsyncRead::poll_read(std::pin::Pin::new(&mut self.get_mut().0), cx, buf) {
            std::task::Poll::Ready(Err(error))
                if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) =>
            {
                std::task::Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}

impl Child {
    pub(super) fn id(&self) -> Option<u32> {
        Some(self.pid)
    }
    pub(super) async fn wait(&mut self) -> io::Result<ExitStatus> {
        loop {
            // A zero-duration wait is cancellation safe and never blocks a runtime worker.
            // SAFETY: the process handle remains owned throughout the wait/query.
            unsafe {
                match WaitForSingleObject(self.handle.as_raw_handle(), 0) {
                    WAIT_OBJECT_0 => {
                        let mut code = 0;
                        if GetExitCodeProcess(self.handle.as_raw_handle(), &mut code) == 0 {
                            return Err(io::Error::last_os_error());
                        }
                        return Ok(ExitStatus::from_raw(code));
                    }
                    WAIT_TIMEOUT => {}
                    _ => return Err(io::Error::last_os_error()),
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

fn wide(value: &OsStr) -> Result<Vec<u16>> {
    let mut value: Vec<_> = value.encode_wide().collect();
    ensure!(!value.contains(&0), "NUL in process argument or path");
    value.push(0);
    Ok(value)
}

/// Microsoft CRT argv rules; no shell is involved. Always quote each argument,
/// doubling backslashes only before quotes and the closing quote.
fn command_line(executable: &Path, args: &[String]) -> Result<Vec<u16>> {
    let exe = wide(executable.as_os_str())?;
    ensure!(!exe.contains(&34), "quote in executable path");
    let mut line = vec![34];
    line.extend_from_slice(&exe[..exe.len() - 1]);
    line.push(34);
    for arg in args {
        let arg = wide(OsStr::new(arg))?;
        line.extend([32, 34]);
        let mut slashes = 0;
        for &unit in &arg[..arg.len() - 1] {
            if unit == 92 {
                slashes += 1;
                continue;
            }
            line.extend(std::iter::repeat_n(
                92,
                slashes * if unit == 34 { 2 } else { 1 },
            ));
            if unit == 34 {
                line.push(92);
            }
            line.push(unit);
            slashes = 0;
        }
        line.extend(std::iter::repeat_n(92, slashes * 2));
        line.push(34);
    }
    line.push(0);
    ensure!(
        line.len() <= 32767,
        "Windows command line exceeds 32767 UTF-16 units"
    );
    Ok(line)
}

fn pipe(inherit_read: bool) -> io::Result<(OwnedHandle, OwnedHandle)> {
    // SAFETY: CreatePipe initializes both handles on success. Wrap immediately
    // so every subsequent failure closes them. Only the child's end is inheritable.
    unsafe {
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let (mut read, mut write) = (null_mut(), null_mut());
        if CreatePipe(&mut read, &mut write, &security, 0) == 0 {
            return Err(io::Error::last_os_error());
        }
        let read = OwnedHandle::from_raw_handle(read);
        let write = OwnedHandle::from_raw_handle(write);
        let parent = if inherit_read { &write } else { &read };
        if SetHandleInformation(parent.as_raw_handle(), HANDLE_FLAG_INHERIT, 0) == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((read, write))
    }
}

// Pointer-aligned backing storage; Delete runs before freeing the allocation.
struct Attributes(Vec<usize>);
impl Attributes {
    fn new() -> io::Result<Self> {
        unsafe {
            let mut bytes = 0;
            InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut bytes);
            if bytes == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
            if InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 2, 0, &mut bytes) == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(Self(storage))
        }
    }
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr().cast()
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: list was successfully initialized and backing storage is alive.
        unsafe { DeleteProcThreadAttributeList(self.pointer()) }
    }
}

pub(super) fn spawn(executable: &Path, args: &[String], cwd: &Path) -> Result<(Child, Group)> {
    spawn_created(executable, args, cwd, |_, _, _| {})
}

fn spawn_created(
    executable: &Path,
    args: &[String],
    cwd: &Path,
    created: impl FnOnce(HANDLE, HANDLE, u32),
) -> Result<(Child, Group)> {
    let application = wide(executable.as_os_str())?;
    let directory = wide(cwd.as_os_str())?;
    let mut line = command_line(executable, args)?;
    let group = Group::new()?;
    let (stdin, stdin_writer) = pipe(true)?;
    drop(stdin_writer); // EOF, equivalent to null stdin.
    let (stdout, stdout_writer) = pipe(false)?;
    let (stderr, stderr_writer) = pipe(false)?;
    let jobs = [group.0.as_raw_handle()];
    let handles = [
        stdin.as_raw_handle(),
        stdout_writer.as_raw_handle(),
        stderr_writer.as_raw_handle(),
    ];
    // Values referenced by the attributes outlive the list and CreateProcessW.
    let mut attributes = Attributes::new()?;
    unsafe {
        for (key, values) in [
            (PROC_THREAD_ATTRIBUTE_JOB_LIST, jobs.as_slice()),
            (PROC_THREAD_ATTRIBUTE_HANDLE_LIST, handles.as_slice()),
        ] {
            if UpdateProcThreadAttribute(
                attributes.pointer(),
                0,
                key as usize,
                values.as_ptr().cast(),
                std::mem::size_of_val(values),
                null_mut(),
                null(),
            ) == 0
            {
                return Err(io::Error::last_os_error().into());
            }
        }
        let mut startup: STARTUPINFOEXW = std::mem::zeroed();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = handles[0];
        startup.StartupInfo.hStdOutput = handles[1];
        startup.StartupInfo.hStdError = handles[2];
        startup.lpAttributeList = attributes.pointer();
        let mut info: PROCESS_INFORMATION = std::mem::zeroed();
        // Atomic membership: no process can exist outside our kill-on-close Job.
        // Only the three explicit pipe handles may be inherited; never the Job.
        // Application is explicit, command line mutable UTF-16, cwd Unicode.
        if CreateProcessW(
            application.as_ptr(),
            line.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT,
            null(),
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut info,
        ) == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        let handle = OwnedHandle::from_raw_handle(info.hProcess);
        drop(OwnedHandle::from_raw_handle(info.hThread));
        created(
            handle.as_raw_handle(),
            group.0.as_raw_handle(),
            info.dwProcessId,
        );
        Ok((
            Child {
                handle,
                pid: info.dwProcessId,
                stdout: Some(PipeReader(tokio::fs::File::from_std(std::fs::File::from(
                    stdout,
                )))),
                stderr: Some(PipeReader(tokio::fs::File::from_std(std::fs::File::from(
                    stderr,
                )))),
            },
            group,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_boundary_crash_helper() {
        let Some(root) = std::env::var_os("FORGE_TEST_CREATION_CRASH") else {
            return;
        };
        let root = std::path::PathBuf::from(root);
        let exe = crate::clients::resolve_command("python.exe", &root).unwrap();
        let _ = spawn_created(
            &exe,
            &["-c".into(), "import time; time.sleep(60)".into()],
            &root,
            |process, job, pid| {
                // Verify membership at the first observable creation boundary,
                // before spawn returns or the supervisor records the PID.
                let mut member = 0;
                unsafe {
                    assert_ne!(IsProcessInJob(process, job, &mut member), 0);
                }
                assert_ne!(member, 0);
                std::fs::write(root.join("pid"), pid.to_string()).unwrap();
                // Hard exit deliberately skips all Rust destructors: the OS must
                // close the non-inherited Job and terminate the child itself.
                unsafe {
                    assert_ne!(TerminateProcess(GetCurrentProcess(), 73), 0);
                }
                unreachable!();
            },
        )
        .unwrap();
        panic!("creation callback did not terminate helper");
    }

    #[test]
    fn atomic_membership_survives_supervisor_crash_before_spawn_returns() {
        let root = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "process::windows::tests::creation_boundary_crash_helper",
                "--nocapture",
            ])
            .env("FORGE_TEST_CREATION_CRASH", root.path())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let pid = std::fs::read_to_string(root.path().join("pid"))
            .unwrap()
            .parse()
            .unwrap();
        for _ in 0..250 {
            if !pid_alive(pid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("child survived supervisor crash at creation boundary");
    }

    #[tokio::test]
    async fn unrelated_inheritable_handle_is_not_passed_to_child() {
        // A deliberately inheritable event must not escape the HANDLE_LIST.
        let event = unsafe {
            let handle = CreateEventW(null(), 1, 0, null());
            assert!(!handle.is_null());
            let event = OwnedHandle::from_raw_handle(handle);
            assert_ne!(
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT),
                0
            );
            event
        };
        let root = tempfile::tempdir().unwrap();
        let code = format!(
            "import ctypes; k=ctypes.WinDLL('kernel32',use_last_error=True); k.SetEvent.argtypes=[ctypes.c_void_p]; k.SetEvent({}); print('attempted')",
            event.as_raw_handle() as usize
        );
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let result = super::super::run(
            &["python.exe".into(), "-c".into(), code],
            root.path(),
            10,
            rx,
        )
        .await
        .unwrap();
        assert!(result.passed());
        assert_eq!(result.stdout, "attempted");
        assert_eq!(
            unsafe { WaitForSingleObject(event.as_raw_handle(), 0) },
            WAIT_TIMEOUT
        );
    }

    #[test]
    fn failed_creation_never_calls_observer() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("invalid.exe");
        std::fs::write(&exe, b"not an executable").unwrap();
        assert!(
            spawn_created(&exe, &[], root.path(), |_, _, _| {
                panic!("failed CreateProcessW reported a child")
            })
            .is_err()
        );
        assert!(command_line(&exe, &["embedded\0nul".into()]).is_err());
        assert!(command_line(&exe, &["x".repeat(32767)]).is_err());
    }
}
