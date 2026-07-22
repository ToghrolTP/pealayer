pub struct TaskbarState {
    pub progress_percent: u32,
    pub is_paused: bool,
    pub is_active: bool,
}

impl Default for TaskbarState {
    fn default() -> Self {
        Self {
            progress_percent: 0,
            is_paused: false,
            is_active: false,
        }
    }
}

pub fn update_windows_taskbar_state(_progress: f64, _duration: f64, _is_paused: bool) {
    // Taskbar integration helper (activates on Windows targets)
    #[cfg(target_os = "windows")]
    {
        // Safe stub / platform adapter for Windows Taskbar ITaskbarList3 progress updates
    }
}

pub fn sync_windows_jump_list(_recent_media: &[std::path::PathBuf]) {
    #[cfg(target_os = "windows")]
    {
        // Jump List MRU synchronization helper for Windows Shell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taskbar_state_default() {
        let state = TaskbarState::default();
        assert_eq!(state.progress_percent, 0);
        assert!(!state.is_paused);
    }
}
