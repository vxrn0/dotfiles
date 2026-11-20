use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

struct Paths {
    config: PathBuf,
    sequences: PathBuf,
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
        Self {
            config: config_home.join("dotfiles/config.json"),
            sequences: state_home.join("quickshell/user/generated/terminal/sequences.txt"),
        }
    }
}

fn main() {
    let paths = Paths::from_env();
    if !enabled(&paths.config) {
        return;
    }
    push(&paths.sequences, &pts_dir());
}

fn enabled(config: &Path) -> bool {
    let text = match fs::read_to_string(config) {
        Ok(text) => text,
        Err(_) => {
            println!(
                "Config file not found at {}. Applying terminal theming by default.",
                config.display()
            );
            return true;
        }
    };
    match interpret(&text) {
        Ok(enabled) => enabled,
        Err(error) => {
            eprintln!("dots-applycolor: {}: {error}", config.display());
            false
        }
    }
}

fn interpret(text: &str) -> Result<bool, serde_json::Error> {
    let document = serde_json::from_str::<Value>(text)?;
    Ok(match document
        .get("appearance")
        .and_then(|value| value.get("wallpaperTheming"))
        .and_then(|value| value.get("enableTerminal"))
    {
        Some(Value::Bool(enabled)) => *enabled,
        Some(Value::String(value)) => value == "true",
        _ => false,
    })
}

fn pts_dir() -> PathBuf {
    env::var_os("DOTS_PTS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/dev/pts"))
}

fn push(sequences: &Path, pts: &Path) {
    let contents = match fs::read(sequences) {
        Ok(contents) => contents,
        Err(_) => {
            println!("Terminal color sequences not found. Skipping that.");
            return;
        }
    };
    let Ok(entries) = fs::read_dir(pts) else {
        return;
    };
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
        let _ = fs::write(&target, &contents);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_terminal_toggle_like_jq() {
        assert!(interpret(r#"{"appearance":{"wallpaperTheming":{"enableTerminal":true}}}"#).unwrap());
        assert!(
            interpret(r#"{"appearance":{"wallpaperTheming":{"enableTerminal":"true"}}}"#).unwrap()
        );
        assert!(!interpret(r#"{"appearance":{"wallpaperTheming":{"enableTerminal":false}}}"#).unwrap());
        assert!(!interpret(r#"{"appearance":{}}"#).unwrap());
        assert!(interpret("not json").is_err());
    }
}
