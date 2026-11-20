use serde_json::Value;
use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TERM_ALPHA: &str = "100";

struct Paths {
    config_home: PathBuf,
    config: PathBuf,
    state: PathBuf,
    runtime: PathBuf,
    colors: PathBuf,
}

impl Paths {
    fn from_env() -> Self {
        let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| home.join(".config"));
        let state_home = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| home.join(".local/state"));
        let runtime = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })));
        let config = config_home.join("quickshell/dotfiles");
        Self {
            colors: config.join("scripts/colors"),
            config_home,
            config,
            state: state_home.join("quickshell"),
            runtime,
        }
    }
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let paths = Paths::from_env();
    let generated = paths.state.join("user/generated");
    if let Err(error) = fs::create_dir_all(&generated) {
        eprintln!(
            "dots-applycolor: cannot create {}: {error}",
            generated.display()
        );
        return 1;
    }
    if !paths.config.is_dir() {
        return 1;
    }
    let colors_file = generated.join("material_colors.scss");
    let colors = match fs::read_to_string(&colors_file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "dots-applycolor: cannot read {}: {error}",
                colors_file.display()
            );
            String::new()
        }
    };
    let (names, values) = parse_colors(&colors);

    let config_file = paths.config_home.join("dotfiles/config.json");
    let enabled = match fs::read_to_string(&config_file) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(document) => match document
                .get("appearance")
                .and_then(|value| value.get("wallpaperTheming"))
                .and_then(|value| value.get("enableTerminal"))
            {
                Some(Value::Bool(true)) => true,
                Some(Value::String(value)) => value == "true",
                _ => false,
            },
            Err(error) => {
                eprintln!("dots-applycolor: {}: {error}", config_file.display());
                false
            }
        },
        Err(_) => {
            println!(
                "Config file not found at {}. Applying terminal theming by default.",
                config_file.display()
            );
            true
        }
    };
    if enabled {
        apply_anyterm(&paths, &generated, &names, &values);
        apply_kitty(&paths, &generated, &names, &values);
    }
    0
}

fn parse_colors(text: &str) -> (Vec<String>, Vec<String>) {
    let mut names = Vec::new();
    let mut values = Vec::new();
    for line in text.lines() {
        let name = line.split(':').next().unwrap_or("").to_string();
        let colon = line.split_once(':').map(|(_, rest)| rest).unwrap_or(line);
        let spaced = match colon.split(' ').nth(1) {
            Some(second) => second,
            None => colon,
        };
        let value = spaced.split(';').next().unwrap_or("").to_string();
        if !name.is_empty() {
            names.push(name);
        }
        if !value.is_empty() {
            values.push(value);
        }
    }
    (names, values)
}

fn replace_all(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(from) {
            out.extend_from_slice(to);
            index += from.len();
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    out
}

fn write_themed(
    template: &Path,
    destination: &Path,
    names: &[String],
    values: &[String],
    alpha: bool,
) -> Result<(), String> {
    fs::copy(template, destination)
        .map_err(|error| format!("cannot copy {}: {error}", template.display()))?;
    let bytes = fs::read(destination)
        .map_err(|error| format!("cannot read {}: {error}", destination.display()))?;
    let mut result = bytes;
    for (index, name) in names.iter().enumerate() {
        let Some(value) = values.get(index) else {
            break;
        };
        let pattern = format!("{name} #");
        let replacement = value.strip_prefix('#').unwrap_or(value);
        result = replace_all(&result, pattern.as_bytes(), replacement.as_bytes());
    }
    if alpha {
        result = replace_all(&result, b"$alpha", TERM_ALPHA.as_bytes());
    }
    fs::write(destination, result)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))
}

fn apply_kitty(paths: &Paths, generated: &Path, names: &[String], values: &[String]) {
    let template = paths.colors.join("terminal/kitty-theme.conf");
    if !template.is_file() {
        println!("Template file not found for Kitty theme. Skipping that.");
        return;
    }
    let terminal = generated.join("terminal");
    let _ = fs::create_dir_all(&terminal);
    let destination = terminal.join("kitty-theme.conf");
    if let Err(error) = write_themed(&template, &destination, names, values, false) {
        eprintln!("dots-applycolor: {error}");
        return;
    }

    let running = Command::new("pgrep")
        .args(["-f", "kitty"])
        .stdout(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !running {
        return;
    }

    let mut pushed = false;
    if let Ok(entries) = fs::read_dir(&paths.runtime) {
        let mut sockets = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("kitty.sock"))
                    .unwrap_or(false)
                    && path
                        .metadata()
                        .map(|meta| meta.file_type().is_socket())
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        sockets.sort();
        for socket in sockets {
            let status = Command::new("kitten")
                .arg("@")
                .arg("--to")
                .arg(format!("unix:{}", socket.display()))
                .args(["set-colors", "--all", "--configured"])
                .arg(&destination)
                .status();
            if status.map(|status| status.success()).unwrap_or(false) {
                pushed = true;
            }
        }
    }
    if !pushed {
        match Command::new("pidof").arg("kitty").output() {
            Ok(output) => {
                let pids = String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .filter_map(|value| value.parse::<i32>().ok())
                    .collect::<Vec<_>>();
                for pid in pids {
                    if unsafe { libc::kill(pid, libc::SIGUSR1) } != 0 {
                        eprintln!(
                            "dots-applycolor: cannot signal kitty pid {pid}: {}",
                            std::io::Error::last_os_error()
                        );
                    }
                }
            }
            Err(error) => eprintln!("dots-applycolor: cannot run pidof: {error}"),
        }
    }
}

fn apply_anyterm(paths: &Paths, generated: &Path, names: &[String], values: &[String]) {
    let template = paths.colors.join("terminal/sequences.txt");
    if !template.is_file() {
        println!("Template file not found for Terminal. Skipping that.");
        return;
    }
    let terminal = generated.join("terminal");
    let _ = fs::create_dir_all(&terminal);
    let destination = terminal.join("sequences.txt");
    if let Err(error) = write_themed(&template, &destination, names, values, true) {
        eprintln!("dots-applycolor: {error}");
        return;
    }
    let contents = match fs::read(&destination) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!(
                "dots-applycolor: cannot read {}: {error}",
                destination.display()
            );
            return;
        }
    };
    let pts = env::var_os("DOTS_PTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/dev/pts"));
    if let Ok(entries) = fs::read_dir(pts) {
        let mut targets = entries
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
                    .unwrap_or(false)
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        targets.sort();
        for target in targets {
            if let Err(error) = fs::write(&target, &contents) {
                eprintln!(
                    "dots-applycolor: cannot write {}: {error}",
                    target.display()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_the_scss_like_the_shell() {
        let (names, values) =
            parse_colors("$background: #111318;\n$primary_paletteKeyColor: #5778AA;\n$broken\n\n");
        assert_eq!(
            names,
            ["$background", "$primary_paletteKeyColor", "$broken"]
        );
        assert_eq!(values, ["#111318", "#5778AA", "$broken"]);
    }

    #[test]
    fn replaces_every_occurrence_without_regex_escapes() {
        assert_eq!(
            replace_all(b"$a # and $a #", b"$a #", b"123456"),
            b"123456 and 123456"
        );
    }
}
