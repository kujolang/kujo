//! Opt-in process lifetime ownership for externally supervised Windows runtimes.
//! This does not grant capabilities or sandbox filesystem/network access.

#[cfg(not(windows))]
pub fn own_descendants() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "--kill-children-on-exit is supported only on Windows",
    ))
}

#[cfg(windows)]
pub fn own_descendants() -> std::io::Result<()> {
    use std::{mem::size_of, ptr};
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
            Threading::GetCurrentProcess,
        },
    };

    // SAFETY: null name creates a fresh unnamed job; null security attributes
    // make its handle non-inheritable. No script-controlled pointers are used.
    let job = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
    if job.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: job is a valid owned handle; limits has the exact documented layout
    // and remains alive through the synchronous call. Breakaway is not enabled.
    let configured = unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    // SAFETY: the pseudo-handle identifies this process. Assignment occurs before
    // script execution, so descendants cannot race ahead of job membership.
    if configured == 0 || unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) } == 0 {
        let error = std::io::Error::last_os_error();
        // SAFETY: assignment did not succeed; closing our sole handle cannot
        // terminate this process or any script child (none has been admitted).
        unsafe { CloseHandle(job) };
        return Err(error);
    }
    // Keep this non-inheritable handle until the OS closes it on process exit.
    // Closing it here would terminate this process too. An unnamed job cannot
    // be reopened by name, and children inherit membership, not handle ownership.
    Ok(())
}

/// Per-command ownership, independent of the optional runtime-wide job.
#[cfg(windows)]
pub(crate) mod command_job {
    use std::{
        io,
        mem::size_of,
        os::windows::{
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
            process::CommandExt,
        },
        process::{Child, Command},
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
                THREADENTRY32,
            },
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
            Threading::{OpenThread, ResumeThread, CREATE_SUSPENDED, THREAD_SUSPEND_RESUME},
        },
    };

    pub(crate) struct CommandJob(OwnedHandle);

    impl CommandJob {
        pub(crate) fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
            // SAFETY: unnamed, non-inheritable job, with no caller-supplied pointers.
            let raw = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: the newly created handle is owned exactly once.
            let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: valid owned job and correctly sized live configuration.
            if unsafe {
                SetInformationJobObject(
                    job.0.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const _,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            // Stable Rust does not expose spawn attributes or the primary thread
            // handle. Suspend before admission so no child code can spawn outside
            // the job; discover the sole initial thread through ToolHelp afterward.
            command.creation_flags(CREATE_SUSPENDED);
            let mut child = command.spawn()?;
            // SAFETY: both handles are live. Nested jobs are supported on Windows
            // 8+, including the optional runtime-wide lifetime job. No breakaway.
            let admitted =
                unsafe { AssignProcessToJobObject(job.0.as_raw_handle(), child.as_raw_handle()) };
            let resumed = if admitted == 0 {
                Err(io::Error::last_os_error())
            } else {
                resume_initial_thread(child.id())
            };
            if let Err(error) = resumed {
                // Fail closed: never run without ownership, and reap a suspended
                // child even if admission failed before it entered the job.
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::other(format!("cannot own process descendants: {error}")));
            }
            Ok((child, job))
        }

        pub(crate) fn terminate(&self) -> io::Result<()> {
            // SAFETY: our owned handle remains valid throughout this call.
            if unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    fn resume_initial_thread(pid: u32) -> io::Result<()> {
        // SAFETY: system thread snapshot; no module/heap inspection of the
        // suspended process. The returned handle is checked before ownership.
        let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if raw == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fresh valid snapshot transferred exactly once.
        let snapshot = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut entry =
            THREADENTRY32 { dwSize: size_of::<THREADENTRY32>() as u32, ..Default::default() };
        // SAFETY: entry is correctly sized and writable; snapshot stays live.
        let mut found = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
        while found != 0 {
            if entry.th32OwnerProcessID == pid {
                // SAFETY: thread belongs to our suspended, unreaped child.
                let raw = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                if raw.is_null() {
                    return Err(io::Error::last_os_error());
                }
                // SAFETY: fresh valid thread handle transferred exactly once.
                let thread = unsafe { OwnedHandle::from_raw_handle(raw) };
                // SAFETY: only the initial thread of our CREATE_SUSPENDED child
                // is resumed; it has already been admitted to the owned job.
                if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
                    return Err(io::Error::last_os_error());
                }
                return Ok(());
            }
            entry.dwSize = size_of::<THREADENTRY32>() as u32;
            // SAFETY: same valid snapshot and writable entry as above.
            found = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
        }
        Err(io::Error::other("suspended child initial thread was not found"))
    }
}
