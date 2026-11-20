use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct Cli {
    file: Option<String>,
    set: Vec<(String, String)>,
    reset: Vec<String>,
    help: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("dots-hyprconfig: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let cli = parse_args(&args);
    if cli.help {
        print_help();
        return Ok(());
    }
    let file = expand_home(
        cli.file
            .as_deref()
            .unwrap_or("~/.config/hypr/hyprland.conf"),
    );
    let mut sets: Vec<(String, String)> = Vec::new();
    let mut resets = cli.reset;
    for (key, value) in cli.set {
        if value == "[[EMPTY]]" {
            resets.push(key);
        } else if let Some(entry) = sets.iter_mut().find(|(name, _)| *name == key) {
            entry.1 = value;
        } else {
            sets.push((key, value));
        }
    }
    if sets.is_empty() && resets.is_empty() {
        println!("Error: Must specify at least one key to set or reset.");
        return Ok(());
    }
    edit(&file, &sets, &resets)
}

fn print_help() {
    println!(
        "usage: dots-hyprconfig [-h] [--file FILE] [--set KEY VALUE] [--reset KEY]\n\nEdit a Hyprland config file. Subkeys use colon (:) for nesting.\n\noptions:\n  -h, --help            show this help message and exit\n  --file FILE           Path to the Hyprland config file (default: ~/.config/hypr/hyprland.conf).\n  --set KEY VALUE       Set a configuration key to a value.\n  --reset KEY           Remove a configuration key."
    );
}

fn parse_args(args: &[String]) -> Cli {
    let mut cli = Cli::default();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        let (flag, inline) = match argument.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
            _ => (argument.as_str(), None),
        };
        let take = |index: &mut usize| -> Option<String> {
            if let Some(value) = inline.clone() {
                return Some(value);
            }
            *index += 1;
            args.get(*index).cloned()
        };
        match flag {
            "-h" | "--help" => cli.help = true,
            "--file" => match take(&mut index) {
                Some(value) => cli.file = Some(value),
                None => argument_error("argument --file: expected one argument"),
            },
            "--set" => {
                if inline.is_some() {
                    argument_error("argument --set: expected 2 arguments");
                }
                let key = match take(&mut index) {
                    Some(value) => value,
                    None => argument_error("argument --set: expected 2 arguments"),
                };
                let value = match take(&mut index) {
                    Some(value) => value,
                    None => argument_error("argument --set: expected 2 arguments"),
                };
                if key.starts_with("--") {
                    argument_error("argument --set: expected 2 arguments");
                }
                cli.set.push((key, value));
            }
            "--reset" => match take(&mut index) {
                Some(value) => cli.reset.push(value),
                None => argument_error("argument --reset: expected one argument"),
            },
            other => argument_error(&format!("unrecognized arguments: {other}")),
        }
        index += 1;
    }
    cli
}

fn argument_error(message: &str) -> ! {
    eprintln!("usage: dots-hyprconfig [-h] [--file FILE] [--set KEY VALUE] [--reset KEY]");
    eprintln!("dots-hyprconfig: error: {message}");
    std::process::exit(2)
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" {
        return PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into())).join(rest);
    }
    PathBuf::from(value)
}

fn python_float(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Python accepts underscores between digits, but not next to a sign,
    // decimal point or exponent marker.
    let bytes = trimmed.as_bytes();
    let mut cleaned = String::with_capacity(trimmed.len());
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'_' {
            let before = index > 0 && bytes[index - 1].is_ascii_digit();
            let after = index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit();
            if !before || !after {
                return false;
            }
        } else {
            cleaned.push(*byte as char);
        }
    }
    cleaned.parse::<f64>().is_ok()
}

fn format_value(value: &str) -> String {
    if value == "true" || value == "false" || python_float(value) {
        value.to_string()
    } else {
        format!("\"{value}\"")
    }
}

fn config_line(key: &str, value: &str) -> String {
    let parts = key.split(':').collect::<Vec<_>>();
    format!("hl.config({{{}}})\n", nested(&parts, value))
}

fn nested(parts: &[&str], value: &str) -> String {
    if parts.len() == 1 {
        format!("{}={}", parts[0], format_value(value))
    } else {
        format!("{}={{{}}}", parts[0], nested(&parts[1..], value))
    }
}

fn matches(line: &str, key: &str) -> bool {
    let parts = key.split(':').collect::<Vec<_>>();
    let Some(mut rest) = line
        .trim_start_matches(char::is_whitespace)
        .strip_prefix("hl.config({")
    else {
        return false;
    };
    for (index, part) in parts.iter().enumerate() {
        rest = rest.trim_start_matches(char::is_whitespace);
        let Some(after) = rest.strip_prefix(part) else {
            return false;
        };
        rest = after.trim_start_matches(char::is_whitespace);
        let Some(after) = rest.strip_prefix('=') else {
            return false;
        };
        rest = after;
        if index + 1 != parts.len() {
            // The Python pattern joins the key parts with a literal `{` and
            // does not allow whitespace between `=` and that brace.
            let Some(after) = rest.strip_prefix('{') else {
                return false;
            };
            rest = after;
        }
    }
    true
}

fn edit(path: &Path, sets: &[(String, String)], resets: &[String]) -> Result<(), String> {
    let raw = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let text = String::from_utf8_lossy(&raw)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut lines = text
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if text.is_empty() {
        lines.clear();
    }

    let mut output = Vec::with_capacity(lines.len() + sets.len());
    let mut found = vec![false; sets.len()];
    for line in lines {
        if resets.iter().any(|key| matches(&line, key)) {
            continue;
        }
        let mut replaced = false;
        for (index, (key, value)) in sets.iter().enumerate() {
            if matches(&line, key) {
                output.push(config_line(key, value));
                found[index] = true;
                replaced = true;
                break;
            }
        }
        if !replaced {
            output.push(line);
        }
    }

    for (index, (key, value)) in sets.iter().enumerate() {
        if !found[index] {
            if let Some(last) = output.last_mut() {
                if !last.ends_with('\n') {
                    last.push('\n');
                }
            }
            output.push(config_line(key, value));
        }
    }

    let parent = absolute(path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let mode = fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o7777)
        .unwrap_or(0o644);
    let temp = parent.join(format!(".hyprconfig.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temp);
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?;
        for line in &output {
            file.write_all(line.as_bytes())?;
        }
        file.flush()?;
        fs::set_permissions(&temp, fs::Permissions::from_mode(mode))?;
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        println!("Error saving file: {error}");
        return Ok(());
    }

    for key in resets {
        println!("Removed '{key}' from '{}'", path.display());
    }
    for (key, value) in sets {
        println!(
            "Updated '{}' with {}",
            path.display(),
            config_line(key, value).trim()
        );
    }
    Ok(())
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_python() {
        assert_eq!(format_value("true"), "true");
        assert_eq!(format_value("false"), "false");
        assert_eq!(format_value("42"), "42");
        assert_eq!(format_value("-3.5"), "-3.5");
        assert_eq!(format_value(".5"), ".5");
        assert_eq!(format_value("1e3"), "1e3");
        assert_eq!(format_value("NaN"), "NaN");
        assert_eq!(format_value("inf"), "inf");
        assert_eq!(format_value("1_000"), "1_000");
        assert_eq!(format_value("True"), "\"True\"");
        assert_eq!(format_value("hello world"), "\"hello world\"");
        assert_eq!(format_value("a,b"), "\"a,b\"");
    }

    #[test]
    fn builds_nested_lines() {
        assert_eq!(config_line("a:b:c", "x"), "hl.config({a={b={c=\"x\"}}})\n");
        assert_eq!(config_line("a", "1"), "hl.config({a=1})\n");
    }

    #[test]
    fn matches_the_python_regex_shape() {
        assert!(matches("hl.config({a=1})", "a"));
        assert!(matches("  hl.config({ a = 1 })", "a"));
        assert!(matches("hl.config({a={b=1}})", "a:b"));
        assert!(!matches("  hl.config({a = { b = 1 }})", "a:b"));
        assert!(matches("  hl.config({a ={ b = 1 }})", "a:b"));
        assert!(!matches("hl.config({ab=1})", "a"));
        assert!(!matches("x hl.config({a=1})", "a"));
        assert!(!matches("hl.config({a={b=1}})", "a:c"));
    }
}
