//! Windows 11 containment. No breakaway flags; descendants inherit the Job.
use anyhow::{Result, bail};
use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::null,
};
use windows_sys::Win32::{
    Foundation::*,
    System::{Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
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
    pub(super) fn attach_and_resume(&self, child: &tokio::process::Child) -> Result<()> {
        // SAFETY: child is alive and CREATE_SUSPENDED; all acquired handles are owned.
        unsafe {
            if AssignProcessToJobObject(self.0.as_raw_handle(), child.raw_handle().unwrap()) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error().into());
            }
            let snapshot = OwnedHandle::from_raw_handle(snapshot);
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = size_of::<THREADENTRY32>() as u32;
            let mut found = Thread32First(snapshot.as_raw_handle(), &mut entry);
            while found != 0 {
                if entry.th32OwnerProcessID == child.id().unwrap() {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if thread.is_null() {
                        return Err(std::io::Error::last_os_error().into());
                    }
                    let thread = OwnedHandle::from_raw_handle(thread);
                    if ResumeThread(thread.as_raw_handle()) == u32::MAX {
                        return Err(std::io::Error::last_os_error().into());
                    }
                    return Ok(());
                }
                found = Thread32Next(snapshot.as_raw_handle(), &mut entry);
            }
            bail!("suspended child thread not found")
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
