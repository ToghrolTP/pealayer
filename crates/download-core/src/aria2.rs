//! aria2 JSON-RPC adapter. Secrets never enter queue snapshots or command lines.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io::Read, time::Duration};

#[derive(Clone)]
pub struct Client {
    endpoint: url::Url,
    secret: Option<String>,
    http: reqwest::blocking::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub gid: String,
    pub status: String,
    #[serde(rename = "totalLength")]
    pub total_length: String,
    #[serde(rename = "completedLength")]
    pub completed_length: String,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
    pub connections: String,
    #[serde(rename = "errorCode", default)]
    pub error_code: String,
}

impl Client {
    pub fn new(endpoint: &str, secret: Option<String>) -> Result<Self, String> {
        let endpoint = url::Url::parse(endpoint).map_err(|_| "Invalid aria2 endpoint")?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || !endpoint
                .host_str()
                .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1"))
        {
            return Err("aria2 requires a local HTTP(S) RPC endpoint without embedded credentials or query parameters".into());
        }
        Ok(Self {
            endpoint,
            secret,
            http: reqwest::blocking::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| "Cannot create aria2 client")?,
        })
    }

    fn call(&self, method: &str, mut params: Vec<Value>) -> Result<Value, String> {
        if let Some(secret) = &self.secret {
            params.insert(0, json!(format!("token:{secret}")));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let response = self
            .http
            .post(self.endpoint.clone())
            .json(&json!({
                "jsonrpc": "2.0", "id": id, "method": method, "params": params
            }))
            .send()
            .map_err(|_| "aria2 is unreachable")?
            .error_for_status()
            .map_err(|_| "aria2 rejected HTTP request")?;
        let mut bytes = Vec::new();
        response
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read aria2 response")?;
        if bytes.len() > 65536 {
            return Err("aria2 response exceeds the RPC limit".into());
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "aria2 returned invalid JSON")?;
        if value.get("id").and_then(Value::as_str) != Some(&id)
            || value.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        {
            return Err("aria2 response correlation is invalid".into());
        }
        if let Some(error) = value.get("error") {
            return Err(format!(
                "aria2 RPC failed (code {})",
                error.get("code").unwrap_or(&Value::Null)
            ));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| "aria2 omitted RPC result".into())
    }

    pub fn version(&self) -> Result<Value, String> {
        self.call("aria2.getVersion", vec![])
    }

    pub fn add(
        &self,
        request: &crate::AddRequest,
        directory: &str,
        filename: &str,
        speed_limit: u64,
    ) -> Result<String, String> {
        crate::validate_request(request)?;
        let connections = request.connections;
        if !(1..=16).contains(&connections) {
            return Err("Connections must be 1..16".into());
        }
        let proxy = if request.use_proxy {
            request
                .proxy_url
                .clone()
                .or_else(|| std::env::var("HTTPS_PROXY").ok())
                .or_else(|| std::env::var("HTTP_PROXY").ok())
                .unwrap_or_default()
        } else {
            String::new()
        };
        self.call("aria2.addUri", vec![json!([request.url]), json!({
            "dir": directory, "split": connections.to_string(), "max-connection-per-server": connections.to_string(),
            "max-download-limit": speed_limit.to_string(), "allow-overwrite": "false", "auto-file-renaming": "false",
            "user-agent": crate::USER_AGENT, "all-proxy": proxy, "out": filename, "continue": "true"
        })])?.as_str().map(str::to_owned).ok_or_else(|| "aria2 returned invalid transfer ID".into())
    }

    pub fn status(&self, gid: &str) -> Result<Status, String> {
        serde_json::from_value(self.call(
            "aria2.tellStatus",
            vec![
                json!(gid),
                json!([
                    "gid",
                    "status",
                    "totalLength",
                    "completedLength",
                    "downloadSpeed",
                    "connections",
                    "errorCode"
                ]),
            ],
        )?)
        .map_err(|_| "aria2 status does not match the transfer contract".into())
    }

    pub fn action(&self, gid: &str, action: &str) -> Result<(), String> {
        let method = match action {
            "pause" => "aria2.pause",
            "resume" => "aria2.unpause",
            "cancel" => "aria2.remove",
            _ => return Err("Unknown aria2 action".into()),
        };
        self.call(method, vec![json!(gid)])?;
        Ok(())
    }

    pub fn set_speed_limit(&self, gid: &str, bytes_per_second: u64) -> Result<(), String> {
        self.call(
            "aria2.changeOption",
            vec![
                json!(gid),
                json!({"max-download-limit": bytes_per_second.to_string()}),
            ],
        )?;
        Ok(())
    }
}
