use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::platform::interop::{InteropCommand, LaunchRequest};

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub target: Option<String>,
    pub fullscreen: bool,
    pub volume: Option<f64>,
    pub commands: Vec<InteropCommand>,
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

pub fn resolved_instance_identity() -> String {
    let mut identity = crate::config::resolved_app_name(&crate::config::AppConfig::load());
    if let Ok(instance_id) = std::env::var("PEALAYER_INSTANCE_ID") {
        let instance_id = instance_id.trim();
        if !instance_id.is_empty() {
            identity.push(':');
            identity.push_str(instance_id);
        }
    }
    identity
}

pub fn format_help_message() -> String {
    format!(
        r#"Pealayer {} - Modern Media & 4D Cinema Player

USAGE:
  pealayer [OPTIONS] [FILE_OR_URL]
  pealayer --remote <COMMAND>

ARGUMENTS:
  [FILE_OR_URL]             Path to a media file or network URL

PLAYER OPTIONS:
  --open <FILE_OR_URL>      Open media (equivalent to the positional argument)
  --play                    Start or resume playback
  --pause                   Pause playback
  --toggle-pause            Toggle play/pause
  --stop                    Stop and close the current media
  --next | --previous       Navigate the playlist
  --seek <SECONDS>          Seek relative to the current position
  --seek-to <SECONDS>       Seek to an absolute playback time
  --seek-percent <0-100>    Seek to a percentage of the media
  -v, --volume <0-130>      Set playback volume
  --mute | --unmute         Set mute state
  --toggle-mute             Toggle mute state
  --rate <0.05-16>          Set playback speed
  -f, --fullscreen          Enter fullscreen
  --windowed                Leave fullscreen
  --toggle-fullscreen       Toggle fullscreen
  --workspace <simple|nle>  Select the player workspace
  --activate                Activate and focus the window
  --minimize | --maximize   Change the window state
  --restore                 Restore and focus the window
  --quit                    Close the running application
  --command <COMMAND>       Queue a unified text or JSON command; repeatable
  --remote <COMMAND>        Send one unified command and exit

APPLICATION OPTIONS:
  --register-associations    Register Pealayer as the default media handler
  --unregister-associations  Unregister Pealayer file associations
  -h, --help                 Print help information
  -V, --version              Print version information
"#,
        env!("CARGO_PKG_VERSION")
    )
}

pub fn parse_cli_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliAction, String> {
    let mut args_iter = args.into_iter().skip(1); // skip program name
    let mut target = None;
    let mut fullscreen = false;
    let mut volume = None;
    let mut commands = Vec::new();

    let parse_number = |option: &str, value: String| {
        value
            .parse::<f64>()
            .map_err(|_| format!("Option '{option}' requires a numeric value"))
    };

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                return Ok(CliAction::PrintHelp(format_help_message()));
            }
            "-V" | "--version" => {
                return Ok(CliAction::PrintVersion(format!(
                    "pealayer {}",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            "--register-associations" => {
                return Ok(CliAction::RegisterAssociations);
            }
            "--unregister-associations" => {
                return Ok(CliAction::UnregisterAssociations);
            }
            "-f" | "--fullscreen" => {
                fullscreen = true;
                commands.push(InteropCommand::SetFullscreen { enabled: true });
            }
            "--windowed" => commands.push(InteropCommand::SetFullscreen { enabled: false }),
            "--toggle-fullscreen" => commands.push(InteropCommand::ToggleFullscreen),
            "-v" | "--volume" => {
                let val_str = args_iter
                    .next()
                    .ok_or("Option '--volume' requires a value between 0 and 130")?;
                let val = parse_number("--volume", val_str)?;
                let command = InteropCommand::SetVolume { value: val };
                command.validate()?;
                volume = Some(val);
                commands.push(command);
            }
            "--open" => {
                let value = args_iter
                    .next()
                    .ok_or("Option '--open' requires a file path or URL")?;
                if target.is_none() {
                    target = Some(value);
                } else {
                    commands.push(InteropCommand::Open { target: value });
                }
            }
            "--play" => commands.push(InteropCommand::Play),
            "--pause" => commands.push(InteropCommand::Pause),
            "--toggle-pause" => commands.push(InteropCommand::TogglePause),
            "--stop" => commands.push(InteropCommand::Stop),
            "--next" => commands.push(InteropCommand::Next),
            "--previous" => commands.push(InteropCommand::Previous),
            "--mute" => commands.push(InteropCommand::SetMute { muted: true }),
            "--unmute" => commands.push(InteropCommand::SetMute { muted: false }),
            "--toggle-mute" => commands.push(InteropCommand::ToggleMute),
            "--activate" => commands.push(InteropCommand::Activate),
            "--minimize" => commands.push(InteropCommand::Minimize),
            "--maximize" => commands.push(InteropCommand::Maximize),
            "--restore" => commands.push(InteropCommand::Restore),
            "--quit" => commands.push(InteropCommand::Quit),
            "--seek" | "--seek-to" | "--seek-percent" | "--rate" => {
                let val_str = args_iter
                    .next()
                    .ok_or_else(|| format!("Option '{arg}' requires a value"))?;
                let value = parse_number(&arg, val_str)?;
                let command = match arg.as_str() {
                    "--seek" => InteropCommand::Seek { seconds: value },
                    "--seek-to" => InteropCommand::SeekTo { seconds: value },
                    "--seek-percent" => InteropCommand::SeekAbs { percentage: value },
                    _ => InteropCommand::SetRate { rate: value },
                };
                command.validate()?;
                commands.push(command);
            }
            "--workspace" => {
                let value = args_iter
                    .next()
                    .ok_or("Option '--workspace' requires 'simple' or 'nle'")?;
                commands.push(crate::platform::interop::parse_text_command(&format!(
                    "workspace {value}"
                ))?);
            }
            "--command" => {
                let value = args_iter
                    .next()
                    .ok_or("Option '--command' requires a text or JSON command")?;
                commands.push(crate::platform::interop::parse_text_command(&value)?);
            }
            "--remote" => {
                let cmd = args_iter.next().ok_or(
                    "Option '--remote' requires a command argument (e.g. 'play', 'pause')",
                )?;
                return Ok(CliAction::SendRemote(cmd));
            }
            "--" => {
                let positional = args_iter
                    .next()
                    .ok_or("'--' must be followed by a media file or URL")?;
                if target.replace(positional).is_some() {
                    return Err("Only one media target may be opened per launch".to_string());
                }
                if args_iter.next().is_some() {
                    return Err("Only one media target may be opened per launch".to_string());
                }
                break;
            }
            other if other.starts_with('-') => {
                return Err(format!("Unrecognized option: {}", other));
            }
            pos => {
                if target.replace(pos.to_string()).is_some() {
                    return Err("Only one media target may be opened per launch".to_string());
                }
            }
        }
    }

    Ok(CliAction::RunGui(CliOptions {
        target,
        fullscreen,
        volume,
        commands,
    }))
}

pub fn send_remote_command(cmd_str: &str) -> Result<String, String> {
    let trimmed = cmd_str.trim();
    let payload = if trimmed.starts_with('{') {
        crate::platform::interop::parse_interop_request(trimmed)?;
        trimmed.to_string()
    } else {
        serde_json::to_string(&crate::platform::interop::parse_text_command(trimmed)?)
            .map_err(|error| error.to_string())?
    };
    send_unified_request(&payload, Duration::from_secs(2))
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
        application_identity: resolved_instance_identity(),
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
        commands: options.commands.clone(),
    }
}

pub fn try_forward_launch_request(request: &LaunchRequest) -> bool {
    let command = InteropCommand::Launch {
        request: request.clone(),
    };
    let payload = match serde_json::to_string(&command) {
        Ok(payload) => payload,
        Err(_) => return false,
    };
    send_unified_request(&payload, Duration::from_millis(500))
        .is_ok_and(|response| response.contains("\"status\":\"accepted\""))
}

fn send_unified_request(payload: &str, timeout: Duration) -> Result<String, String> {
    let identity = resolved_instance_identity();
    crate::platform::interop::send_native_request(payload, &identity, timeout).or_else(
        |native_error| {
            send_control_request(payload, timeout).map_err(|http_error| {
                format!("{native_error}; HTTP fallback also failed: {http_error}")
            })
        },
    )
}

fn send_control_request(payload: &str, timeout: Duration) -> Result<String, String> {
    let address = format!("127.0.0.1:{}", crate::config::control_port());
    let socket_address = address
        .parse()
        .map_err(|error| format!("invalid control address: {error}"))?;
    let mut stream = TcpStream::connect_timeout(&socket_address, Duration::from_millis(500))
        .map_err(|error| {
            format!("Could not connect to the player instance at {address}: {error}")
        })?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| error.to_string())?;

    let request = format!(
        "POST /api/ipc HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|error| error.to_string())?;

    let mut response = String::new();
    BufReader::new(stream)
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "invalid HTTP response from Pealayer control endpoint".to_string())?;
    if !headers.starts_with("HTTP/1.1 200") {
        return Err(format!(
            "Pealayer control endpoint rejected request: {headers}"
        ));
    }
    Ok(body.trim().to_string())
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
                commands: vec![],
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
                commands: vec![
                    InteropCommand::SetFullscreen { enabled: true },
                    InteropCommand::SetVolume { value: 80.0 },
                ],
            })
        );

        // 3. Remote IPC command
        let args = vec![
            "pealayer".to_string(),
            "--remote".to_string(),
            "play".to_string(),
        ];
        let action = parse_cli_args(args).unwrap();
        assert_eq!(action, CliAction::SendRemote("play".to_string()));

        // 4. Help and Version flags
        let args_h = vec!["pealayer".to_string(), "--help".to_string()];
        assert!(matches!(
            parse_cli_args(args_h).unwrap(),
            CliAction::PrintHelp(_)
        ));

        let args_v = vec!["pealayer".to_string(), "-V".to_string()];
        assert!(matches!(
            parse_cli_args(args_v).unwrap(),
            CliAction::PrintVersion(_)
        ));
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
                commands: vec![],
            })
        );

        // Long form flags
        let args = vec![
            "pealayer".to_string(),
            "--fullscreen".to_string(),
            "--volume".to_string(),
            "140".to_string(),
        ];
        assert!(parse_cli_args(args).is_err());

        // Volume clamping to 0
        let args = vec!["pealayer".to_string(), "-v".to_string(), "-20".to_string()];
        assert!(parse_cli_args(args).is_err());

        // Short help and long version
        let action_h = parse_cli_args(vec!["pealayer".to_string(), "-h".to_string()]).unwrap();
        assert!(matches!(action_h, CliAction::PrintHelp(_)));

        let action_ver =
            parse_cli_args(vec!["pealayer".to_string(), "--version".to_string()]).unwrap();
        assert!(matches!(action_ver, CliAction::PrintVersion(_)));

        // Error cases
        assert!(parse_cli_args(vec!["pealayer".to_string(), "-v".to_string()]).is_err());
        assert!(
            parse_cli_args(vec![
                "pealayer".to_string(),
                "-v".to_string(),
                "abc".to_string()
            ])
            .is_err()
        );
        assert!(parse_cli_args(vec!["pealayer".to_string(), "--remote".to_string()]).is_err());
        assert!(
            parse_cli_args(vec!["pealayer".to_string(), "--unknown-flag".to_string()]).is_err()
        );
    }

    #[test]
    fn test_cli_association_flags() {
        let args_reg = vec![
            "pealayer".to_string(),
            "--register-associations".to_string(),
        ];
        assert_eq!(
            parse_cli_args(args_reg).unwrap(),
            CliAction::RegisterAssociations
        );

        let args_unreg = vec![
            "pealayer".to_string(),
            "--unregister-associations".to_string(),
        ];
        assert_eq!(
            parse_cli_args(args_unreg).unwrap(),
            CliAction::UnregisterAssociations
        );
    }

    #[test]
    fn player_switches_use_the_unified_command_model() {
        let args = [
            "pealayer",
            "--open",
            "movie.mkv",
            "--play",
            "--seek-to",
            "12.5",
            "--mute",
            "--rate",
            "1.25",
            "--workspace",
            "simple",
            "--maximize",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        let CliAction::RunGui(options) = parse_cli_args(args).unwrap() else {
            panic!("expected GUI options");
        };
        assert_eq!(options.target.as_deref(), Some("movie.mkv"));
        assert_eq!(
            options.commands,
            vec![
                InteropCommand::Play,
                InteropCommand::SeekTo { seconds: 12.5 },
                InteropCommand::SetMute { muted: true },
                InteropCommand::SetRate { rate: 1.25 },
                InteropCommand::SetWorkspace { nle: false },
                InteropCommand::Maximize,
            ]
        );
    }
}
