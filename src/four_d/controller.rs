use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use crate::four_d::protocol::Command;

pub const DEFAULT_ENDPOINT: &str = "pccontroller://127.0.0.1:8787";

pub struct ControllerClient {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: u64,
}

impl ControllerClient {
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let address = normalize_endpoint(endpoint)?;
        let socket = address
            .to_socket_addrs()
            .map_err(|error| format!("resolve PCController endpoint {address}: {error}"))?
            .next()
            .ok_or_else(|| format!("PCController endpoint {address} resolved to no address"))?;
        let writer = TcpStream::connect_timeout(&socket, Duration::from_secs(2))
            .map_err(|error| format!("connect to PCController at {address}: {error}"))?;
        writer
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|error| format!("configure PCController read timeout: {error}"))?;
        writer
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|error| format!("configure PCController write timeout: {error}"))?;
        writer
            .set_nodelay(true)
            .map_err(|error| format!("configure PCController TCP stream: {error}"))?;
        let reader = BufReader::new(
            writer
                .try_clone()
                .map_err(|error| format!("clone PCController TCP stream: {error}"))?,
        );
        let mut client = Self {
            writer,
            reader,
            next_id: 1,
        };
        client.call("controller.ping", json!({}))?;
        Ok(client)
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        serde_json::to_writer(&mut self.writer, &request)
            .map_err(|error| format!("encode PCController JSON-RPC request: {error}"))?;
        self.writer
            .write_all(b"\n")
            .and_then(|_| self.writer.flush())
            .map_err(|error| format!("write PCController JSON-RPC request: {error}"))?;

        loop {
            let mut line = String::new();
            let read = self
                .reader
                .read_line(&mut line)
                .map_err(|error| format!("read PCController JSON-RPC response: {error}"))?;
            if read == 0 {
                return Err("PCController closed the JSON-RPC connection".to_string());
            }
            let response: Value = serde_json::from_str(line.trim())
                .map_err(|error| format!("decode PCController JSON-RPC response: {error}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = response.get("error").filter(|value| !value.is_null()) {
                return Err(format!("PCController JSON-RPC error: {error}"));
            }
            return Ok(response.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub fn send_command(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::Ping => {
                self.call("controller.ping", json!({}))?;
            }
            Command::RelaySet { id, state } => {
                self.call(
                    "controller.command.execute",
                    json!({"command": format!("relay {id} {}", if state { "on" } else { "off" })}),
                )?;
            }
            Command::PwmSet { channel, value } => {
                let value = (u32::from(value) * 4095 / 255) as u16;
                self.call(
                    "controller.pwm.set",
                    json!({"channel": channel, "value": value}),
                )?;
            }
            Command::AllOff => {
                self.call(
                    "controller.command.execute",
                    json!({"command": "relay off"}),
                )?;
                self.call("controller.pwm.off", json!({}))?;
            }
        }
        Ok(())
    }
}

pub fn normalize_endpoint(endpoint: &str) -> Result<&str, String> {
    let address = endpoint
        .strip_prefix("pccontroller://")
        .or_else(|| endpoint.strip_prefix("tcp://"))
        .unwrap_or(endpoint)
        .trim();
    if address.is_empty() || !address.contains(':') {
        return Err(format!("invalid PCController endpoint: {endpoint}"));
    }
    Ok(address)
}

pub fn is_controller_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("pccontroller://") || endpoint.starts_with("tcp://")
}

pub fn direct_serial_name(endpoint: &str) -> &str {
    endpoint.strip_prefix("direct:").unwrap_or(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn normalizes_controller_endpoints() {
        assert_eq!(
            normalize_endpoint(DEFAULT_ENDPOINT).unwrap(),
            "127.0.0.1:8787"
        );
        assert_eq!(normalize_endpoint("tcp://host:9000").unwrap(), "host:9000");
        assert!(normalize_endpoint("missing-port").is_err());
    }

    #[test]
    fn exchanges_correlated_ndjson_rpc() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            for _ in 0..2 {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(line.trim()).unwrap();
                let response = json!({"jsonrpc":"2.0","id":request["id"],"result":{"ok":true}});
                writeln!(stream, "{response}").unwrap();
            }
        });

        let mut client = ControllerClient::connect(&format!("pccontroller://{address}")).unwrap();
        let result = client.call("controller.snapshot", json!({})).unwrap();
        assert_eq!(result["ok"], true);
        server.join().unwrap();
    }
}
