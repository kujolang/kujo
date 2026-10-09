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
