use serde_json::Value;
use std::env;
use std::ffi::CString;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Args {
    region: String,
    sound: bool,
    fullscreen: bool,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args = parse_args(&env::args().skip(1).collect::<Vec<_>>());
    let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
    let recording_dir = recording_dir(&home);

    if process_running("wf-recorder") {
        let _ = Command::new("pkill").args(["-x", "wf-recorder"]).status();
        notify("Recording stopped", &recording_dir.to_string_lossy());
        return 0;
    }

    let mut region = args.region;
    if !args.fullscreen && region.is_empty() {
        match Command::new("slurp").stderr(Stdio::inherit()).output() {
            Ok(output) if output.status.success() => {
                region = String::from_utf8_lossy(&output.stdout)
                    .trim_end_matches(['\n', '\r'])
                    .to_string();
            }
            _ => {
                notify("Recording cancelled", "Selection was cancelled");
                return 1;
            }
        }
    }

    if let Err(error) = fs::create_dir_all(&recording_dir) {
        eprintln!(
            "dots-record: cannot create {}: {error}",
            recording_dir.display()
        );
        return 1;
    }
    let output_name = format!("recording_{}.mp4", timestamp());
    let mut command = Command::new("wf-recorder");
    command.current_dir(&recording_dir);
    command.args(["-x", "yuv420p", "-f"]);
    command.arg(&output_name);
    if args.fullscreen {
        command.args(["-o", &active_monitor()]);
    }
    if !region.is_empty() {
        command.args(["--geometry", &region]);
    }
    if args.sound {
        command.arg(format!("--audio={}", audio_output()));
    }
    notify("Recording started", &output_name);
    let error = command.exec();
    eprintln!("dots-record: cannot run wf-recorder: {error}");
    1
}

fn parse_args(args: &[String]) -> Args {
    let mut parsed = Args {
        region: String::new(),
        sound: false,
        fullscreen: false,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--region" => {
                parsed.region = args.get(index + 1).cloned().unwrap_or_default();
                index += 2;
            }
            "--sound" => {
                parsed.sound = true;
                index += 1;
            }
            "--fullscreen" => {
                parsed.fullscreen = true;
                index += 1;
            }
            _ => index += 1,
        }
    }
    parsed
}

fn recording_dir(home: &Path) -> PathBuf {
    let config = home.join(".config/dotfiles/config.json");
    let fallback = home.join("Videos");
    let text = match fs::read_to_string(&config) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("dots-record: cannot read {}: {error}", config.display());
            return fallback;
        }
    };
    let Ok(document) = serde_json::from_str::<Value>(&text) else {
        eprintln!("dots-record: {} is not valid JSON", config.display());
        return fallback;
    };
    let Some(value) = document
        .get("screenRecord")
        .and_then(|record| record.get("savePath"))
        .and_then(Value::as_str)
    else {
        return fallback;
    };
    if value.is_empty() {
        fallback
    } else {
        PathBuf::from(value)
    }
}

fn process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .stdout(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn notify(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args([summary, body, "-a", "Recorder"])
        .status();
}

fn active_monitor() -> String {
    let output = match Command::new("hyprctl")
        .args(["monitors", "-j"])
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(_) => return String::new(),
        Err(error) => {
            eprintln!("dots-record: cannot run hyprctl: {error}");
            return String::new();
        }
    };
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        eprintln!("dots-record: hyprctl monitors -j did not return JSON");
        return String::new();
    };
    value
        .as_array()
        .and_then(|monitors| {
            monitors
                .iter()
                .find(|monitor| monitor.get("focused") == Some(&Value::Bool(true)))
        })
        .and_then(|monitor| monitor.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn audio_output() -> String {
    match Command::new("pactl")
        .args(["get-default-sink"])
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) if output.status.success() => {
            let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sink.is_empty() {
                return format!("{sink}.monitor");
            }
        }
        Ok(_) => {}
        Err(error) => eprintln!("dots-record: cannot run pactl: {error}"),
    }
    let output = match Command::new("pactl")
        .args(["list", "sources", "short"])
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("dots-record: cannot run pactl: {error}");
            return String::new();
        }
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|line| line.contains("monitor"))
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("")
        .to_string()
}

fn timestamp() -> String {
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let mut local: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut local).is_null() {
            return "unknown".to_string();
        }
        let format = CString::new("%Y-%m-%d_%H.%M.%S").unwrap();
        let mut buffer = [0i8; 64];
        let length = libc::strftime(buffer.as_mut_ptr(), buffer.len(), format.as_ptr(), &local);
        if length == 0 {
            return "unknown".to_string();
        }
        std::ffi::CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_the_shell_flags() {
        let parsed = parse_args(&args(&[
            "--region",
            "10,20 300x200",
            "--sound",
            "--fullscreen",
        ]));
        assert_eq!(parsed.region, "10,20 300x200");
        assert!(parsed.sound && parsed.fullscreen);
        let parsed = parse_args(&args(&["--unknown", "--region", "1,2 3x4"]));
        assert_eq!(parsed.region, "1,2 3x4");
        let parsed = parse_args(&args(&["--region"]));
        assert_eq!(parsed.region, "");
    }

    #[test]
    fn timestamp_has_the_bash_shape() {
        let stamp = timestamp();
        assert_eq!(stamp.len(), 19, "{stamp}");
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], "_");
        assert_eq!(&stamp[13..14], ".");
        assert_eq!(&stamp[16..17], ".");
    }
}
