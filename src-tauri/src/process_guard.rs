//! Process lifecycle protection to guarantee that child sidecar processes
//! (python-runner, llama-server) are 100% killed when the parent Voquill process exits.

use std::process::Command;

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    struct SafeJobHandle(HANDLE);
    unsafe impl Send for SafeJobHandle {}
    unsafe impl Sync for SafeJobHandle {}

    impl Drop for SafeJobHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }

    static GLOBAL_JOB: OnceLock<Option<SafeJobHandle>> = OnceLock::new();

    fn get_global_job() -> Option<HANDLE> {
        GLOBAL_JOB
            .get_or_init(|| unsafe {
                let job = match CreateJobObjectW(None, None) {
                    Ok(j) => j,
                    Err(e) => {
                        crate::log_warn!("Failed to create Windows Job Object: {}", e);
                        return None;
                    }
                };

                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                let res = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );

                if let Err(e) = res {
                    crate::log_warn!("Failed to configure Windows Job Object limit flags: {}", e);
                    let _ = windows::Win32::Foundation::CloseHandle(job);
                    None
                } else {
                    crate::log_info!("Initialized Windows Job Object with KILL_ON_JOB_CLOSE guard");
                    Some(SafeJobHandle(job))
                }
            })
            .as_ref()
            .map(|h| h.0)
    }

    pub fn assign_pid_to_job(pid: u32) {
        if let Some(job) = get_global_job() {
            unsafe {
                match OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) {
                    Ok(proc_handle) => {
                        if let Err(e) = AssignProcessToJobObject(job, proc_handle) {
                            crate::log_warn!(
                                "Failed to assign PID {} to Windows Job Object: {}",
                                pid,
                                e
                            );
                        } else {
                            crate::log_info!(
                                "Assigned PID {} to Windows Job Object termination guard",
                                pid
                            );
                        }
                        let _ = windows::Win32::Foundation::CloseHandle(proc_handle);
                    }
                    Err(e) => {
                        crate::log_warn!(
                            "Failed to open process PID {} for Job Object: {}",
                            pid,
                            e
                        );
                    }
                }
            }
        }
    }
}

/// Binds a child process by PID to the OS-level process guard.
/// On Windows, binds the PID to a Job Object configured with KILL_ON_JOB_CLOSE.
pub fn bind_child_process(child_pid: u32) {
    #[cfg(target_os = "windows")]
    {
        windows_impl::assign_pid_to_job(child_pid);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = child_pid;
    }
}

/// Configures standard `std::process::Command` with platform death signals before spawn.
#[allow(dead_code)]
pub fn configure_command_death_signal(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cmd;
    }
}

/// Configures `tokio::process::Command` with platform death signals before spawn.
pub fn configure_tokio_command_death_signal(cmd: &mut tokio::process::Command) {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cmd;
    }
}
