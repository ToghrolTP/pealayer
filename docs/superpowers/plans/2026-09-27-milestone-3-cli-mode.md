# Milestone 3: CLI Mode & Launch Arguments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement robust command-line argument parsing supporting file/URL targets, fullscreen, volume presets, remote IPC control, and single-instance forwarding.

**Architecture:** A standalone, zero-dependency `src/cli.rs` module that parses command-line arguments into `CliAction` variants (`RunGui`, `SendRemote`, `PrintHelp`, `PrintVersion`). Before initializing the graphical subsystem, if a target file or URL is passed, the CLI attempts a brief handshake with an active Pealayer instance on `127.0.0.1:8082`. If an instance is running, it forwards the media target via IPC and exits immediately; otherwise, it boots the GUI and auto-loads the requested media.

**Tech Stack:** Rust 2024 edition, `std::env::args`, `std::net::TcpStream`, `serde_json`, `eframe::egui`.

**Spec:** [Issue #2](https://github.com/ToghrolTP/pealayer/issues/2) (Integration: CLI for providing options and commands; as well as loading media files and URLs).

## Global Constraints
- Zero additional heavy dependencies; implement parsing with pure Rust stdlib in `src/cli.rs`.
- Support standard flags: `--fullscreen` / `-f`, `--volume` / `-v`, `--remote`, `--help` / `-h`, `--version` / `-V`.
- If an existing instance is running and target media is provided, forwarding must complete within 200ms or fall back to GUI launch.
- Must compile cleanly with zero warnings on all supported targets.

---

### Task 1: CLI Argument Parser Module

**Files:**
- Create: `src/cli.rs`
- Modify: `src/lib.rs`
- Test: `src/cli.rs`

**Interfaces:**
- Produces:
  ```rust
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
      PrintHelp(String),
      PrintVersion(String),
  }

  pub fn parse_cli_args<I: IntoIterator<Item = String>>(args: I) -> Result<CliAction, String>;
  ```

- [ ] **Step 1: Write failing unit test for argument parsing**

In `src/cli.rs`:
```rust
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_parse_cli_arguments`
Expected: FAIL with missing module or function

- [ ] **Step 3: Implement minimal CLI parser in src/cli.rs**

In `src/cli.rs`:
```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_parse_cli_arguments`
Expected: PASS

---

### Task 2: IPC Remote Client & Single-Instance Forwarding

**Files:**
- Modify: `src/cli.rs`
- Test: `tests/cli_forwarding_test.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn send_remote_command(cmd_str: &str) -> Result<String, String>;
  pub fn try_forward_to_existing_instance(target: &str) -> bool;
  ```

- [ ] **Step 1: Write integration test for CLI remote forwarding**

Create `tests/cli_forwarding_test.rs`:
```rust
use pealayer::cli::{send_remote_command, try_forward_to_existing_instance};
use pealayer::platform::interop::{spawn_interop_listener, InteropCommand};
use std::sync::mpsc::channel;
use std::time::Duration;

#[test]
fn test_cli_remote_and_single_instance_forwarding() {
    let (tx, rx) = channel::<InteropCommand>();
    let ctx = eframe::egui::Context::default();
    spawn_interop_listener(tx, ctx);

    std::thread::sleep(Duration::from_millis(100));

    // 1. Test forwarding an open target
    let forwarded = try_forward_to_existing_instance("test_video.mkv");
    assert!(forwarded);

    let cmd = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive forwarded open command");
    if let InteropCommand::Open { target } = cmd {
        assert_eq!(target, "test_video.mkv");
    } else {
        panic!("Expected Open command");
    }

    // 2. Test sending a remote command
    let resp = send_remote_command("pause").expect("Failed to send remote command");
    assert!(resp.contains("\"status\":\"ok\""));

    let cmd2 = rx.recv_timeout(Duration::from_secs(1)).expect("Did not receive remote pause command");
    assert!(matches!(cmd2, InteropCommand::Pause));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_forwarding_test`
Expected: FAIL with "cannot find function `send_remote_command`"

- [ ] **Step 3: Implement client helpers in src/cli.rs**

In `src/cli.rs`:
```rust
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn send_remote_command(cmd_str: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect_timeout(
        &"127.0.0.1:8082".parse().unwrap(),
        Duration::from_millis(500),
    ).map_err(|e| format!("Could not connect to Pealayer on 127.0.0.1:8082: {}", e))?;

    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e| e.to_string())?;

    let payload = if cmd_str.trim_start().starts_with('{') {
        cmd_str.trim().to_string() + "\n"
    } else {
        match cmd_str.trim().to_lowercase().as_str() {
            "play" => "{\"command\":\"play\"}\n".to_string(),
            "pause" => "{\"command\":\"pause\"}\n".to_string(),
            "toggle" | "toggle_pause" => "{\"command\":\"toggle_pause\"}\n".to_string(),
            "status" | "get_status" => "{\"command\":\"get_status\"}\n".to_string(),
            s if s.starts_with("seek ") => {
                let sec: f64 = s[5..].trim().parse().unwrap_or(0.0);
                format!("{{\"command\":\"seek\",\"seconds\":{}}}\n", sec)
            }
            s if s.starts_with("volume ") => {
                let v: f64 = s[7..].trim().parse().unwrap_or(100.0);
                format!("{{\"command\":\"set_volume\",\"value\":{}}}\n", v)
            }
            s if s.starts_with("open ") => {
                let target = s[5..].trim();
                format!("{{\"command\":\"open\",\"target\":\"{}\"}}\n", target)
            }
            other => format!("{{\"command\":\"{}\"}}\n", other),
        }
    };

    stream.write_all(payload.as_bytes()).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).map_err(|e| e.to_string())?;
    Ok(response.trim().to_string())
}

pub fn try_forward_to_existing_instance(target: &str) -> bool {
    let payload = format!("{{\"command\":\"open\",\"target\":\"{}\"}}\n", target);
    if let Ok(mut stream) = TcpStream::connect_timeout(
        &"127.0.0.1:8082".parse().unwrap(),
        Duration::from_millis(200),
    ) {
        if stream.write_all(payload.as_bytes()).is_ok() && stream.flush().is_ok() {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() && line.contains("\"status\":\"ok\"") {
                return true;
            }
        }
    }
    false
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test cli_forwarding_test`
Expected: PASS

---

### Task 3: Main Entry Point & GUI Integration

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Hook CLI parsing at beginning of main in src/main.rs**

In `src/main.rs`:
```rust
pub mod cli;

fn main() -> eframe::Result {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let cli_options = match crate::cli::parse_cli_args(args) {
        Ok(crate::cli::CliAction::PrintHelp(msg)) => {
            println!("{}", msg);
            return Ok(());
        }
        Ok(crate::cli::CliAction::PrintVersion(ver)) => {
            println!("{}", ver);
            return Ok(());
        }
        Ok(crate::cli::CliAction::SendRemote(cmd)) => {
            match crate::cli::send_remote_command(&cmd) {
                Ok(resp) => {
                    println!("{}", resp);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Ok(crate::cli::CliAction::RunGui(opts)) => {
            if let Some(ref target) = opts.target {
                if crate::cli::try_forward_to_existing_instance(target) {
                    println!("Forwarded '{}' to active Pealayer instance.", target);
                    return Ok(());
                }
            }
            opts
        }
        Err(err) => {
            eprintln!("Error: {}\nRun 'pealayer --help' for usage.", err);
            std::process::exit(1);
        }
    };
```

- [ ] **Step 2: Apply fullscreen and initial startup media loading**

In `src/main.rs`:
```rust
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([800.0, 600.0])
        .with_transparent(true);
    if cli_options.fullscreen {
        viewport = viewport.with_fullscreen(true);
    }
```
And inside `Box::new(|cc| { ... })`:
```rust
    let initial_volume = cli_options.volume.unwrap_or(loaded_config.volume);
    let _ = mpv_static.set_property("volume", initial_volume);
```
And after constructing `PealayerApp`:
```rust
    let mut app = PealayerApp { ... };
    if let Some(target) = cli_options.target {
        if target.starts_with("http://") || target.starts_with("https://") {
            app.load_url(&target);
        } else {
            app.load_video_file(std::path::PathBuf::from(target));
        }
    }
    Ok(Box::new(app))
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All unit tests and integration tests pass with 0 errors.
