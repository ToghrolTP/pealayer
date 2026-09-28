use std::collections::HashSet;
use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};

use libloading::Library;
use serde_json::{Value, json};

type InvokeFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type FreeFn = unsafe extern "C" fn(*mut c_char);

const LIBRARY_ENV: &str = "PEALAYER_PCCONTROLLER_LIBRARY";
const DATA_ROOT_ENV: &str = "PEALAYER_PCCONTROLLER_DATA_ROOT";
const HTTP_ADDRESS_ENV: &str = "PEALAYER_PCCONTROLLER_HTTP_ADDRESS";

#[derive(Debug, Clone)]
pub struct EmbeddedHostOptions {
    pub data_root: PathBuf,
    pub app_id: String,
    pub app_name: String,
    pub disable_auto_connect: bool,
    pub disable_native: bool,
    pub enable_integrations: bool,
    pub http_address: Option<String>,
}

impl EmbeddedHostOptions {
    pub fn for_pealayer() -> Self {
        let data_root = std::env::var_os(DATA_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                crate::config::AppConfig::get_config_path()
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("pccontroller")
            });
        let app_name = std::env::var("APP_NAME")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Pealayer".to_string());
        let http_address = std::env::var(HTTP_ADDRESS_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Self {
            data_root,
            app_id: "pealayer".to_string(),
            app_name,
            disable_auto_connect: false,
            disable_native: false,
            enable_integrations: true,
            http_address,
        }
    }
}

#[derive(Clone, Copy)]
struct NativeApi {
    invoke: InvokeFn,
    free: FreeFn,
}

pub struct EmbeddedHost {
    api: NativeApi,
    handle: Option<u64>,
    library_path: PathBuf,
    endpoints: Value,
}

impl EmbeddedHost {
    pub fn discover_and_start() -> Result<Self, String> {
        let candidates = library_candidates();
        if candidates.is_empty() {
            return Err(format!(
                "no packaged PCController library found; set {LIBRARY_ENV} or place {} beside Pealayer",
                platform_library_name()
            ));
        }

        let options = EmbeddedHostOptions::for_pealayer();
        let mut errors = Vec::new();
        for path in candidates {
            match Self::load_and_start(&path, &options) {
                Ok(host) => return Ok(host),
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }
        Err(format!(
            "could not start packaged PCController host: {}",
            errors.join("; ")
        ))
    }

    pub fn load_and_start(path: &Path, options: &EmbeddedHostOptions) -> Result<Self, String> {
        // PCController's Go c-shared runtime is process-lifetime state. Leak the
        // loader handle deliberately so Rust never unloads it while Go runtime
        // threads may still exist; host_stop/host_destroy still run normally.
        let library = unsafe { Library::new(path) }
            .map_err(|error| format!("load PCController library: {error}"))?;
        let invoke = unsafe {
            *library
                .get::<InvokeFn>(b"PCControllerInvoke\0")
                .map_err(|error| format!("resolve PCControllerInvoke: {error}"))?
        };
        let free = unsafe {
            *library
                .get::<FreeFn>(b"PCControllerFree\0")
                .map_err(|error| format!("resolve PCControllerFree: {error}"))?
        };
        let _library = Box::leak(Box::new(library));
        let api = NativeApi { invoke, free };

        let config_path = options.data_root.join("config.json");
        let create = invoke_json(
            api,
            &json!({
                "operation": "host_create",
                "host_options": {
                    "config_path": config_path.to_string_lossy(),
                    "data_root": options.data_root.to_string_lossy(),
                    "app_id": options.app_id,
                    "app_name": options.app_name,
                    "disable_auto_connect": options.disable_auto_connect,
                    "disable_native": options.disable_native,
                    "enable_integrations": options.enable_integrations,
                    "http_address": options.http_address,
                }
            }),
        )?;
        let handle = create
            .get("handle")
            .and_then(Value::as_u64)
            .ok_or_else(|| "PCController host_create returned no handle".to_string())?;
        let mut host = Self {
            api,
            handle: Some(handle),
            library_path: path.to_path_buf(),
            endpoints: Value::Null,
        };

        let start_result = host.invoke_host("host_start", json!({}));
        if let Err(error) = start_result {
            let _ = host.shutdown();
            return Err(error);
        }
        if let Err(error) = host.call("controller.ping", json!({})) {
            let _ = host.shutdown();
            return Err(format!("canonical in-process ping failed: {error}"));
        }
        match host.invoke_host("host_endpoints", json!({})) {
            Ok(result) => host.endpoints = result,
            Err(error) => {
                let _ = host.shutdown();
                return Err(error);
            }
        }
        Ok(host)
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.invoke_host(
            "host_call",
            json!({
                "method": method,
                "params": params,
            }),
        )
    }

    pub fn endpoints(&self) -> &Value {
        &self.endpoints
    }

    pub fn description(&self) -> String {
        format!("embedded:{}", self.library_path.display())
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        let stop = invoke_json(
            self.api,
            &json!({
                "operation": "host_stop",
                "handle": handle,
                "timeout_ms": 10_000,
            }),
        );
        let destroy = invoke_json(
            self.api,
            &json!({
                "operation": "host_destroy",
                "handle": handle,
                "timeout_ms": 10_000,
            }),
        );
        match (stop, destroy) {
            (Ok(_), Ok(_)) => Ok(()),
            (Err(stop_error), Ok(_)) => Err(format!("host_stop: {stop_error}")),
            (Ok(_), Err(destroy_error)) => Err(format!("host_destroy: {destroy_error}")),
            (Err(stop_error), Err(destroy_error)) => Err(format!(
                "host_stop: {stop_error}; host_destroy: {destroy_error}"
            )),
        }
    }

    fn invoke_host(&mut self, operation: &str, fields: Value) -> Result<Value, String> {
        let handle = self
            .handle
            .ok_or_else(|| "PCController embedded host is already destroyed".to_string())?;
        let mut request = fields.as_object().cloned().unwrap_or_default();
        request.insert("operation".to_string(), Value::String(operation.to_string()));
        request.insert("handle".to_string(), Value::from(handle));
        invoke_json(self.api, &Value::Object(request))
            .map(|response| response.get("result").cloned().unwrap_or(Value::Null))
    }
}

impl Drop for EmbeddedHost {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn invoke_json(api: NativeApi, request: &Value) -> Result<Value, String> {
    let request = CString::new(request.to_string())
        .map_err(|_| "PCController request contains an interior NUL".to_string())?;
    let response_ptr = unsafe { (api.invoke)(request.as_ptr().cast_mut()) };
    if response_ptr.is_null() {
        return Err("PCControllerInvoke returned NULL".to_string());
    }
    let response = unsafe { CStr::from_ptr(response_ptr) }
        .to_bytes()
        .to_vec();
    unsafe { (api.free)(response_ptr) };
    let value: Value = serde_json::from_slice(&response)
        .map_err(|error| format!("decode PCController response: {error}"))?;
    if !value.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("PCController operation failed")
            .to_string());
    }
    Ok(value)
}

fn platform_library_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "pccontroller.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "pccontroller.dylib"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "pccontroller.so"
    }
}

pub fn library_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os(LIBRARY_ENV) {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.push(directory.join(platform_library_name()));
        candidates.push(directory.join("pccontroller").join(platform_library_name()));
    }
    if let Ok(directory) = std::env::current_dir() {
        candidates.push(directory.join(platform_library_name()));
        candidates.push(directory.join("bin").join(platform_library_name()));
    }

    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|path| path.is_file())
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_library_name_is_not_versioned() {
        let name = platform_library_name();
        assert!(name.starts_with("pccontroller."));
        assert!(!name.contains("v1"));
        assert!(!name.contains("v2"));
    }
}
