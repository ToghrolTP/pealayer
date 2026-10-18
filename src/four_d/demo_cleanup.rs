use std::fs;

#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if res == 0 {
        true
    } else {
        let err = std::io::Error::last_os_error();
        err.raw_os_error() == Some(libc::EPERM)
    }
}

#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
        Ok(h) => h,
        Err(_) => return false,
    };

    let mut exit_code = 0;
    let alive = unsafe { GetExitCodeProcess(handle, &mut exit_code) }.is_ok()
        && exit_code == STILL_ACTIVE.0 as u32;
    let _ = unsafe { CloseHandle(handle) };
    alive
}

#[cfg(not(any(unix, windows)))]
pub fn is_process_alive(_pid: u32) -> bool {
    true
}

pub fn cleanup_stale_demo_resources() {
    let temp_dir = std::env::temp_dir();
    let entries = match fs::read_dir(&temp_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if let Some(pid) = extract_pid_from_name(file_name) {
            if !is_process_alive(pid) {
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else if path.is_file() {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
}

fn extract_pid_from_name(name: &str) -> Option<u32> {
    if let Some(rest) = name.strip_prefix("pealayer-demo-coordinator-") {
        let pid_str = rest.split('-').next()?;
        pid_str.parse::<u32>().ok()
    } else if let Some(rest) = name.strip_prefix("virtual-mcu-eeprom-") {
        let without_bin = rest.strip_suffix(".bin").unwrap_or(rest);
        let pid_str = without_bin.split('-').next()?;
        pid_str.parse::<u32>().ok()
    } else {
        None
    }
}
