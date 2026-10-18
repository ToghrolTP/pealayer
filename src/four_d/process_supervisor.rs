use std::io;
use std::process::{Child, Command, ExitStatus};

#[cfg(all(unix, not(target_os = "macos")))]
use std::os::unix::process::CommandExt;

pub struct ManagedChild {
    child: Child,
    #[cfg(windows)]
    job_handle: Option<windows::Win32::Foundation::HANDLE>,
}

impl ManagedChild {
    pub fn spawn(mut cmd: Command) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        unsafe {
            cmd.pre_exec(|| {
                // When the parent thread/process terminates, deliver SIGKILL to child
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }

        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows::Win32::Foundation::{CloseHandle, HANDLE};
            use windows::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            };

            let job = unsafe { CreateJobObjectW(None, None) }.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("CreateJobObjectW failed: {e}"))
            })?;

            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let set_res = unsafe {
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const std::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if let Err(e) = set_res {
                let _ = unsafe { CloseHandle(job) };
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("SetInformationJobObject failed: {e}"),
                ));
            }

            let child = cmd.spawn()?;
            let child_handle = HANDLE(child.as_raw_handle());
            if let Err(e) = unsafe { AssignProcessToJobObject(job, child_handle) } {
                let _ = unsafe { CloseHandle(job) };
                let mut c = child;
                let _ = c.kill();
                let _ = c.wait();
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("AssignProcessToJobObject failed: {e}"),
                ));
            }

            Ok(Self {
                child,
                job_handle: Some(job),
            })
        }

        #[cfg(not(windows))]
        {
            let child = cmd.spawn()?;
            Ok(Self { child })
        }
    }

    #[inline]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    #[inline]
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    #[inline]
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }

    #[inline]
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    #[inline]
    pub fn take_stdin(&mut self) -> Option<std::process::ChildStdin> {
        self.child.stdin.take()
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(windows)]
        if let Some(job) = self.job_handle.take() {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(job);
            }
        }
    }
}
