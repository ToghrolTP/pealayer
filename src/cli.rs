use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::platform::interop::{InteropCommand, LaunchRequest};

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub target: Option<String>,
    pub fullscreen: bool,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CliAction {
    RunGui(CliOptions),
    SendRemote(String),
    RegisterAssociations,
    UnregisterAssociations,
    PrintHelp(String),
    PrintVersion(String),
}

pub fn format_help_message() -> String {
    format!(
        "Pealayer {} - Modern Media & 4D Cinema Player\n\n\
        USAGE:\n  \
          pealayer [OPTIONS] [FILE_OR_URL]\n  \
          pealayer --remote <COMMAND>\n\n\
        ARGUMENTS:\n  \
          [FILE_OR_URL]             Path to media file or network URL to play\n\n\
        OPTIONS:\n  \
          -f, --fullscreen          Start player in fullscreen mode\n  \
          -v, --volume <0-130>      Set initial playback volume level\n  \
          --remote <COMMAND>        Send IPC command to running instance and exit\n  \
          --register-associations    Register Pealayer as the default handler for media files\n  \
          --unregister-associations  Unregister Pealayer file associations\n  \
          -h, --help                Print help information\n  \
          -V, --version             Print version information\n",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn parse_cli_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliAction, String> {
    let mut args_iter = args.into_iter().skip(1); // skip program name
    let mut target = None;
    let mut fullscreen = false;
    let mut volume = None;

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                return Ok(CliAction::PrintHelp(format_help_message()));
            }
            "-V" | "--version" => {
                return Ok(CliAction::PrintVersion(format!("pealayer {}", env!("CARGO_PKG_VERSION"))));
            }
            "--register-associations" => {
                return Ok(CliAction::RegisterAssociations);
            }
            "--unregister-associations" => {
                return Ok(CliAction::UnregisterAssociations);
            }
            "-f" | "--fullscreen" => {
                fullscreen = true;
            }
            "-v" | "--volume" => {
                let val_str = args_iter.next().ok_or("Option '--volume' requires a value between 0 and 130")?;
                let val = val_str.parse::<f64>().map_err(|_| "Invalid volume value: must be a number")?;
                volume = Some(val.clamp(0.0, 130.0));
            }
            "--remote" => {
                let cmd = args_iter.next().ok_or("Option '--remote' requires a command argument (e.g. 'play', 'pause')")?;
                return Ok(CliAction::SendRemote(cmd));
            }
            other if other.starts_with('-') => {
                return Err(format!("Unrecognized option: {}", other));
            }
            pos => {
                if target.is_none() {
                    target = Some(pos.to_string());
                }
            }
        }
    }

    Ok(CliAction::RunGui(CliOptions {
        target,
        fullscreen,
        volume,
    }))
}

pub fn send_remote_command(cmd_str: &str) -> Result<String, String> {
    let address = format!(
        "127.0.0.1:{}",
        crate::config::runtime_port("PEALAYER_IPC_PORT", 8082)
    );
    let mut stream = TcpStream::connect_timeout(
        &address.parse().unwrap(),
        Duration::from_millis(500),
    ).map_err(|e| format!("Could not connect to the player instance at {address}: {e}"))?;

    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;

    let trimmed = cmd_str.trim();
    let payload = if trimmed.starts_with('{') {
        trimmed.to_string() + "\n"
    } else {
        let lower = trimmed.to_lowercase();
        match lower.as_str() {
            "play" => "{\"command\":\"play\"}\n".to_string(),
            "pause" => "{\"command\":\"pause\"}\n".to_string(),
            "toggle" | "toggle_pause" => "{\"command\":\"toggle_pause\"}\n".to_string(),
            "status" | "get_status" => "{\"command\":\"get_status\"}\n".to_string(),
            s if s.starts_with("seek ") => {
                let sec: f64 = trimmed[5..].trim().parse().unwrap_or(0.0);
                serde_json::json!({
                    "command": "seek",
                    "seconds": sec
                }).to_string() + "\n"
            }
            s if s.starts_with("volume ") => {
                let v: f64 = trimmed[7..].trim().parse().unwrap_or(100.0);
                serde_json::json!({
                    "command": "set_volume",
                    "value": v
                }).to_string() + "\n"
            }
            s if s.starts_with("open ") => {
                let target = trimmed[5..].trim();
                serde_json::json!({
                    "command": "open",
                    "target": target
                }).to_string() + "\n"
            }
            _ => serde_json::json!({
                "command": trimmed
            }).to_string() + "\n",
        }
    };

    stream.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).map_err(|e| e.to_string())?;
    Ok(response.trim().to_string())
}

pub fn launch_request(options: &CliOptions) -> LaunchRequest {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    LaunchRequest {
        operation_id: format!(
            "launch-{}-{timestamp}-{}",
            std::process::id(),
            REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ),
        application_identity: crate::config::resolved_app_name(&crate::config::AppConfig::load()),
        #[cfg(target_os = "windows")]
        sender_session_id: crate::platform::windows::current_session_id().ok(),
        #[cfg(not(target_os = "windows"))]
        sender_session_id: None,
        sender_working_directory: std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().to_string()),
        target: options.target.clone(),
        fullscreen: options.fullscreen,
        volume: options.volume,
        activate: true,
    }
}

pub fn try_forward_launch_request(request: &LaunchRequest) -> bool {
    let command = InteropCommand::Launch {
        request: request.clone(),
    };
    let payload = match serde_json::to_string(&command) {
        Ok(payload) => payload + "\n",
        Err(_) => return false,
    };
    let address = format!(
        "127.0.0.1:{}",
        crate::config::runtime_port("PEALAYER_IPC_PORT", 8082)
    );
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &address.parse().unwrap(),
        Duration::from_millis(200),
    ) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        if stream.write_all(payload.as_bytes()).is_ok() && stream.flush().is_ok() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok()
                && line.contains("\"status\":\"accepted\"")
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_arguments() {
        // 1. Basic file target
        let args = vec!["pealayer".to_string(), "sample.mp4".to_string()];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(
            action,
            CliAction::RunGui(CliOptions {
                target: Some("sample.mp4".to_string()),
                fullscreen: false,
                volume: None,
            })
        );

        // 2. Fullscreen and volume flags
        let args = vec![
            "pealayer".to_string(),
            "-f".to_string(),
            "-v".to_string(),
            "80".to_string(),
            "https://test.com/stream.m3u8".to_string(),
        ];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(
            action,
            CliAction::RunGui(CliOptions {
                target: Some("https://test.com/stream.m3u8".to_string()),
                fullscreen: true,
                volume: Some(80.0),
            })
        );

        // 3. Remote IPC command
        let args = vec!["pealayer".to_string(), "--remote".to_string(), "play".to_string()];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(action, CliAction::SendRemote("play".to_string()));

        // 4. Help and Version flags
        let args_h = vec!["pealayer".to_string(), "--help".to_string()];
        assert!(matches!(parse_cli_args(args_h).unwrap(), CliAction::PrintHelp(_)));

        let args_v = vec!["pealayer".to_string(), "-V".to_string()];
        assert!(matches!(parse_cli_args(args_v).unwrap(), CliAction::PrintVersion(_)));
    }

    #[test]
    fn test_parse_cli_edge_cases() {
        // Empty args (just binary)
        let action = parse_cli_args(vec!["pealayer".to_string()]).unwrap();
        assert_eq!(
            action,
            CliAction::RunGui(CliOptions {
                target: None,
                fullscreen: false,
                volume: None,
            })
        );

        // Long form flags
        let args = vec![
            "pealayer".to_string(),
            "--fullscreen".to_string(),
            "--volume".to_string(),
            "140".to_string(),
        ];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(
            action,
            CliAction::RunGui(CliOptions {
                target: None,
                fullscreen: true,
                volume: Some(130.0), // clamped to 130
            })
        );

        // Volume clamping to 0
        let args = vec![
            "pealayer".to_string(),
            "-v".to_string(),
            "-20".to_string(),
        ];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(
            action,
            CliAction::RunGui(CliOptions {
                target: None,
                fullscreen: false,
                volume: Some(0.0),
            })
        );

        // Short help and long version
        let action_h = parse_cli_args(vec!["pealayer".to_string(), "-h".to_string()]).unwrap();
        assert!(matches!(action_h, CliAction::PrintHelp(_)));

        let action_ver = parse_cli_args(vec!["pealayer".to_string(), "--version".to_string()]).unwrap();
        assert!(matches!(action_ver, CliAction::PrintVersion(_)));

        // Error cases
        assert!(parse_cli_args(vec!["pealayer".to_string(), "-v".to_string()]).is_err());
        assert!(parse_cli_args(vec!["pealayer".to_string(), "-v".to_string(), "abc".to_string()]).is_err());
        assert!(parse_cli_args(vec!["pealayer".to_string(), "--remote".to_string()]).is_err());
        assert!(parse_cli_args(vec!["pealayer".to_string(), "--unknown-flag".to_string()]).is_err());
    }

    #[test]
    fn test_cli_association_flags() {
        let args_reg = vec!["pealayer".to_string(), "--register-associations".to_string()];
        assert_eq!(parse_cli_args(args_reg).unwrap(), CliAction::RegisterAssociations);

        let args_unreg = vec!["pealayer".to_string(), "--unregister-associations".to_string()];
        assert_eq!(parse_cli_args(args_unreg).unwrap(), CliAction::UnregisterAssociations);
    }
}
