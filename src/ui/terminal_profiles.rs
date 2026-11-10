//! Owned Rust views of PortableShell's profile discovery domain. Only call
//! discovery from an explicit refresh worker: C owns formats and native paths.
//! This bridge never expands values, resolves executables, launches or writes.
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

const MAX_PROFILES: usize = 768;
const MAX_COPIED_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Profile {
    pub stable_id: String,
    pub name: String,
    pub provenance: String,
    pub source: Option<String>,
    pub command_line: Option<String>,
    pub program: Option<String>,
    pub args: Vec<String>,
    pub path_candidates: Vec<String>,
    pub cwd: Option<String>,
    pub env: BTreeMap<String, Option<String>>,
    pub icon: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
    pub is_default: bool,
    pub unresolved: bool,
    pub elevated: bool,
    pub inherit_cwd: bool,
    pub clear_env: bool,
    pub format: u32,
    pub platform: u32,
}

impl Profile {
    pub fn ready(&self) -> bool {
        self.blocked_reason().is_none()
    }

    pub fn blocked_reason(&self) -> Option<&'static str> {
        if self.disabled {
            Some("This profile is disabled in its source settings")
        } else if self.elevated {
            Some("This profile requests elevation; embedded terminals do not elevate automatically")
        } else if self.unresolved {
            Some("This profile needs explicit source, variable or executable resolution before use")
        } else if self
            .program
            .as_ref()
            .is_none_or(|program| program.trim().is_empty())
        {
            Some("This profile has no configured executable")
        } else {
            None
        }
    }
}

#[derive(Debug, Default)]
pub struct Discovery {
    pub profiles: Vec<Profile>,
    pub warnings: Vec<String>,
}

#[repr(C)]
struct RawEnv {
    name: *const c_char,
    value: *const c_char,
    unset: c_uint,
}

#[repr(C)]
struct RawProfile {
    stable_id: *const c_char,
    name: *const c_char,
    provenance: *const c_char,
    source: *const c_char,
    command_line: *const c_char,
    executable: *const c_char,
    cwd: *const c_char,
    icon: *const c_char,
    arguments: [*const c_char; 64],
    path_candidates: [*const c_char; 8],
    environment: [RawEnv; 64],
    argument_count: c_uint,
    path_count: c_uint,
    environment_count: c_uint,
    flags: c_uint,
    format: c_uint,
    platform: c_uint,
}

#[repr(C)]
struct ScanResult {
    files_read: c_uint,
    profiles_emitted: c_uint,
    errors: c_uint,
    cancelled: c_uint,
}

type Emit = unsafe extern "C" fn(*mut c_void, *const RawProfile) -> c_int;
type Diagnostic = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char);

unsafe extern "C" {
    fn ps_terminal_profiles_discover(
        emit: Option<Emit>,
        context: *mut c_void,
        diagnostic: Option<Diagnostic>,
        diagnostic_context: *mut c_void,
    ) -> ScanResult;
    fn ps_terminal_profiles_import_file(
        path: *const c_char,
        format: c_uint,
        platform: c_uint,
        emit: Option<Emit>,
        context: *mut c_void,
        diagnostic: Option<Diagnostic>,
        diagnostic_context: *mut c_void,
    ) -> ScanResult;
}

#[derive(Default)]
struct Collector {
    result: Discovery,
    bytes: usize,
    error: Option<String>,
}

impl Collector {
    unsafe fn string(&mut self, raw: *const c_char) -> Result<Option<String>, String> {
        if raw.is_null() {
            return Ok(None);
        }
        // The pinned C implementation owns valid NUL-terminated storage for the
        // callback's duration. Check size/encoding before allocating our copy.
        let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes();
        if bytes.len() > 1024 * 1024 || self.bytes.saturating_add(bytes.len()) > MAX_COPIED_BYTES {
            return Err("Imported terminal profile data exceeds the copy limit".into());
        }
        let value =
            std::str::from_utf8(bytes).map_err(|_| "Profile importer returned invalid UTF-8")?;
        self.bytes += bytes.len();
        Ok(Some(value.to_owned()))
    }

    unsafe fn required(&mut self, raw: *const c_char) -> Result<String, String> {
        unsafe { self.string(raw) }?
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Profile importer omitted required identity data".into())
    }

    unsafe fn profile(&mut self, raw: &RawProfile) -> Result<Profile, String> {
        if raw.argument_count > 64 || raw.path_count > 8 || raw.environment_count > 64 {
            return Err("Profile importer returned invalid array bounds".into());
        }
        let mut args = Vec::with_capacity(raw.argument_count as usize);
        for value in raw.arguments.iter().take(raw.argument_count as usize) {
            args.push(unsafe { self.string(*value) }?.ok_or("Missing imported argument")?);
        }
        let mut path_candidates = Vec::with_capacity(raw.path_count as usize);
        for value in raw.path_candidates.iter().take(raw.path_count as usize) {
            path_candidates.push(unsafe { self.required(*value) }?);
        }
        let mut env = BTreeMap::new();
        for value in raw.environment.iter().take(raw.environment_count as usize) {
            let name = unsafe { self.required(value.name) }?;
            let value = if value.unset != 0 {
                None
            } else {
                Some(
                    unsafe { self.string(value.value) }?
                        .ok_or("Missing imported environment value")?,
                )
            };
            if env.insert(name, value).is_some() {
                return Err("Profile importer returned duplicate environment entries".into());
            }
        }
        Ok(Profile {
            stable_id: unsafe { self.required(raw.stable_id) }?,
            name: unsafe { self.required(raw.name) }?,
            provenance: unsafe { self.required(raw.provenance) }?,
            source: unsafe { self.string(raw.source) }?,
            command_line: unsafe { self.string(raw.command_line) }?,
            program: unsafe { self.string(raw.executable) }?,
            cwd: unsafe { self.string(raw.cwd) }?,
            icon: unsafe { self.string(raw.icon) }?,
            args,
            path_candidates,
            env,
            hidden: raw.flags & 1 != 0,
            disabled: raw.flags & 2 != 0,
            is_default: raw.flags & 4 != 0,
            unresolved: raw.flags & 8 != 0,
            inherit_cwd: raw.flags & 16 != 0,
            clear_env: raw.flags & 32 != 0,
            elevated: raw.flags & 64 != 0,
            format: raw.format,
            platform: raw.platform,
        })
    }

    fn finish(self, result: ScanResult) -> Result<Discovery, String> {
        if let Some(error) = self.error {
            return Err(error);
        }
        if result.cancelled != 0 {
            return Err("Terminal profile discovery was cancelled".into());
        }
        if result.profiles_emitted as usize != self.result.profiles.len() {
            return Err("Profile importer returned an inconsistent record count".into());
        }
        if result.errors != 0 && self.result.warnings.is_empty() {
            return Err("Terminal profile discovery failed without diagnostics".into());
        }
        Ok(self.result)
    }
}

unsafe extern "C" fn receive(context: *mut c_void, raw: *const RawProfile) -> c_int {
    if context.is_null() || raw.is_null() {
        return 0;
    }
    let collector = unsafe { &mut *context.cast::<Collector>() };
    if collector.error.is_some() {
        return 0;
    }
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        if collector.result.profiles.len() >= MAX_PROFILES {
            return Err("Too many terminal profiles were discovered".into());
        }
        let profile = unsafe { collector.profile(&*raw) }?;
        if collector
            .result
            .profiles
            .iter()
            .any(|entry| entry.stable_id == profile.stable_id)
        {
            return Err("Profile importer returned duplicate stable identities".into());
        }
        collector.result.profiles.push(profile);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => 1,
        Ok(Err(error)) => {
            collector.error = Some(error);
            0
        }
        Err(_) => {
            collector.error = Some("Terminal profile callback failed".into());
            0
        }
    }
}

unsafe extern "C" fn diagnostic(context: *mut c_void, path: *const c_char, message: *const c_char) {
    if context.is_null() {
        return;
    }
    let collector = unsafe { &mut *context.cast::<Collector>() };
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        if collector.result.warnings.len() >= 32 {
            return Err("Too many profile import diagnostics".into());
        }
        let path = unsafe { collector.string(path) }?.unwrap_or_default();
        let message = unsafe { collector.required(message) }?;
        collector.result.warnings.push(format!("{path}: {message}"));
        Ok(())
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => collector.error = Some(error),
        Err(_) => collector.error = Some("Terminal profile diagnostic callback failed".into()),
    }
}

/// Explicit refresh worker API. No session is opened or settings overwritten.
/// Use discover_with_diagnostics when the UI can show partial-source warnings.
pub fn discover() -> Result<Vec<Profile>, String> {
    let result = discover_with_diagnostics()?;
    if !result.warnings.is_empty() {
        return Err(result.warnings.join("\n"));
    }
    Ok(result.profiles)
}

pub fn discover_with_diagnostics() -> Result<Discovery, String> {
    let mut collector = Collector::default();
    let context = (&mut collector as *mut Collector).cast::<c_void>();
    let result =
        unsafe { ps_terminal_profiles_discover(Some(receive), context, Some(diagnostic), context) };
    collector.finish(result)
}

/// Import only a user-selected path, retaining the exact source path in every
/// record. This is not a search path or permission to launch imported commands.
pub fn import_file(path: &str, format: u32, platform: u32) -> Result<Discovery, String> {
    if path.len() > 4096 {
        return Err("Profile settings path is too long".into());
    }
    let path = CString::new(path).map_err(|_| "Profile settings path contains NUL")?;
    let mut collector = Collector::default();
    let context = (&mut collector as *mut Collector).cast::<c_void>();
    let result = unsafe {
        ps_terminal_profiles_import_file(
            path.as_ptr(),
            format,
            platform,
            Some(receive),
            context,
            Some(diagnostic),
            context,
        )
    };
    collector.finish(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_data_outlives_callbacks_and_preserves_literal_values_and_launch_guards() {
        let path =
            std::env::temp_dir().join(format!("pealayer-profile-{}.jsonc", uuid::Uuid::new_v4()));
        let data = r#"{"terminal.integrated.profiles.windows":{
          "Ready":{"path":"cmd.exe","args":["/k","literal"],"env":{"EMPTY":"","UNSET":null}},
          "Tokens":{"path":"${env:SHELL}"},"Disabled":null,
          "Elevated":{"path":"cmd.exe","elevate":true},
          "Hidden":{"path":"cmd.exe","hidden":true}}}"#;
        std::fs::write(&path, data).unwrap();
        let result = import_file(path.to_str().unwrap(), 2, 1);
        std::fs::remove_file(&path).unwrap();
        let result = result.unwrap();
        assert!(result.warnings.is_empty());
        assert_eq!(result.profiles.len(), 5);
        assert!(result.profiles[0].ready());
        assert_eq!(result.profiles[0].args, vec!["/k", "literal"]);
        assert_eq!(result.profiles[0].env["EMPTY"], Some(String::new()));
        assert_eq!(result.profiles[0].env["UNSET"], None);
        assert_eq!(result.profiles[1].program.as_deref(), Some("${env:SHELL}"));
        assert!(!result.profiles[1].ready());
        assert!(!result.profiles[2].ready());
        assert!(!result.profiles[3].ready());
        assert!(result.profiles[4].hidden);
        assert!(result.profiles[4].ready());
        assert!(
            result
                .profiles
                .iter()
                .all(|profile| profile.provenance == path.to_string_lossy())
        );
    }

    #[test]
    fn malformed_source_reports_diagnostics_without_partial_delivery() {
        let path =
            std::env::temp_dir().join(format!("pealayer-profile-{}.jsonc", uuid::Uuid::new_v4()));
        std::fs::write(
            &path,
            r#"{"profiles":[{"name":"valid"},{"name":"invalid","hidden":"false"}]}"#,
        )
        .unwrap();
        let result = import_file(path.to_str().unwrap(), 1, 1);
        std::fs::remove_file(&path).unwrap();
        let result = result.unwrap();
        assert!(result.profiles.is_empty());
        assert_eq!(result.warnings.len(), 1);
    }
}
