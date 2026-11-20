use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

struct Args {
    interval: String,
    duration: String,
    source: String,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let Some(args) = parse_args(&env::args().skip(1).collect::<Vec<_>>()) else {
        return 1;
    };
    if find_in_path("songrec").is_none() {
        return 1;
    }

    let device = match args.source.as_str() {
        "monitor" => {
            let sink = command_output("pactl", &["get-default-sink"])
                .map(|output| String::from_utf8_lossy(&output).trim().to_string())
                .unwrap_or_default();
            format!("{sink}.monitor")
        }
        "input" => {
            let info = command_output("pactl", &["info"]).unwrap_or_default();
            String::from_utf8_lossy(&info)
                .lines()
                .find(|line| line.contains("Default Source:"))
                .and_then(|line| line.split_whitespace().nth(2))
                .unwrap_or("")
                .to_string()
        }
        _ => {
            println!("Invalid source type");
            return 1;
        }
    };

    if device.is_empty() {
        return 1;
    }
    let sources = command_output("pactl", &["list", "short", "sources"]).unwrap_or_default();
    if !String::from_utf8_lossy(&sources).contains(&device) {
        return 1;
    }

    let mut child = match Command::new("songrec")
        .args([
            "listen",
            "--audio-device",
            &device,
            "--request-interval",
            &args.interval,
            "--json",
            "--disable-mpris",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("dots-recognize: cannot run songrec: {error}");
            return 1;
        }
    };
    let pid = child.id() as i32;
    let finished = Arc::new(AtomicBool::new(false));
    if let Some(duration) = parse_duration(&args.duration) {
        let finished = Arc::clone(&finished);
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            if !finished.load(Ordering::SeqCst) {
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
        });
    } else {
        eprintln!(
            "dots-recognize: invalid duration '{}', no timeout",
            args.duration
        );
    }

    let stdout = child.stdout.take();
    let mut matched = false;
    if let Some(stdout) = stdout {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };
            if line.contains("\"matches\": [") {
                println!("{line}");
                matched = true;
                break;
            }
        }
    }
    finished.store(true, Ordering::SeqCst);
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    let _ = child.wait();
    let _ = matched;
    0
}

fn command_output(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    match Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) => Some(output.stdout),
        Err(error) => {
            eprintln!("dots-recognize: cannot run {program}: {error}");
            None
        }
    }
}

fn parse_args(args: &[String]) -> Option<Args> {
    let mut parsed = Args {
        interval: "2".to_string(),
        duration: "30".to_string(),
        source: "monitor".to_string(),
    };
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" {
            break;
        }
        if !argument.starts_with('-') || argument.len() == 1 {
            break;
        }
        let characters = argument[1..].chars().collect::<Vec<_>>();
        let flag = characters[0];
        if !matches!(flag, 'i' | 't' | 's') {
            eprintln!("dots-recognize: invalid option -- '{flag}'");
            return None;
        }
        let value = if characters.len() > 1 {
            characters[1..].iter().collect::<String>()
        } else {
            index += 1;
            match args.get(index) {
                Some(value) => value.clone(),
                None => {
                    eprintln!("dots-recognize: option requires an argument -- '{flag}'");
                    return None;
                }
            }
        };
        match flag {
            'i' => parsed.interval = value,
            't' => parsed.duration = value,
            's' => parsed.source = value,
            _ => unreachable!(),
        }
        index += 1;
    }
    Some(parsed)
}

fn parse_duration(text: &str) -> Option<Duration> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (number, multiplier) = match trimmed.chars().last()? {
        's' => (&trimmed[..trimmed.len() - 1], 1.0),
        'm' => (&trimmed[..trimmed.len() - 1], 60.0),
        'h' => (&trimmed[..trimmed.len() - 1], 3600.0),
        'd' => (&trimmed[..trimmed.len() - 1], 86400.0),
        _ => (trimmed, 1.0),
    };
    let seconds = number.trim().parse::<f64>().ok()? * multiplier;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Some(Duration::from_secs_f64(seconds))
}

fn find_in_path(program: &str) -> Option<PathBuf> {
    for directory in env::var_os("PATH")?.to_string_lossy().split(':') {
        let candidate = Path::new(directory).join(program);
        if let Ok(metadata) = candidate.metadata() {
            if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_getopts_forms() {
        let parsed = parse_args(&args(&["-i", "1", "-t0.5", "-sinput", "ignored"])).unwrap();
        assert_eq!(parsed.interval, "1");
        assert_eq!(parsed.duration, "0.5");
        assert_eq!(parsed.source, "input");
        let parsed = parse_args(&args(&["--", "-i", "1"])).unwrap();
        assert_eq!(parsed.interval, "2");
        assert!(parse_args(&args(&["-x"])).is_none());
        assert!(parse_args(&args(&["-i"])).is_none());
    }

    #[test]
    fn parses_sleep_durations() {
        assert_eq!(parse_duration("2"), Some(Duration::from_secs(2)));
        assert_eq!(parse_duration("0.5"), Some(Duration::from_millis(500)));
        assert_eq!(parse_duration("1m"), Some(Duration::from_secs(60)));
        assert_eq!(parse_duration("1.5h"), Some(Duration::from_secs(5400)));
        assert_eq!(parse_duration("bad"), None);
    }
}
