use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".local/state"))
        .join("cliphist-prune.id");
    let database = env::var_os("CLIPHIST_DB")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".cache/cliphist/db"));

    let Some(newest) = newest_id(&database) else {
        return 0;
    };
    if newest.is_empty() {
        return 0;
    }

    if fs::metadata(&state)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
    {
        if let Ok(text) = fs::read_to_string(&state) {
            let mark = text.trim_end_matches('\n');
            if !mark.is_empty() && mark.bytes().all(|byte| byte.is_ascii_digit()) {
                if let Some(list) = listing(&database) {
                    let threshold = mark.parse::<f64>().unwrap_or(0.0);
                    let filtered = list
                        .lines()
                        .filter(|line| {
                            let field = line.split('\t').next().unwrap_or("");
                            numeric_prefix(field) <= threshold
                        })
                        .collect::<Vec<_>>();
                    let mut child = match Command::new("cliphist")
                        .args(["-db-path"])
                        .arg(&database)
                        .arg("delete")
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(error) => {
                            eprintln!("dots-prune: cannot run cliphist delete: {error}");
                            return 0;
                        }
                    };
                    if let Some(mut stdin) = child.stdin.take() {
                        for line in filtered {
                            let _ = writeln!(stdin, "{line}");
                        }
                    }
                    let _ = child.wait();
                }
            }
        }
    }

    if let Some(parent) = state.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("dots-prune: cannot create {}: {error}", parent.display());
            return 1;
        }
    }
    if let Err(error) = fs::write(&state, format!("{newest}\n")) {
        eprintln!("dots-prune: cannot write {}: {error}", state.display());
        return 1;
    }
    0
}

fn listing(database: &PathBuf) -> Option<String> {
    match Command::new("cliphist")
        .args(["-db-path"])
        .arg(database)
        .arg("list")
        .stderr(Stdio::inherit())
        .output()
    {
        Ok(output) => Some(String::from_utf8_lossy(&output.stdout).into_owned()),
        Err(error) => {
            eprintln!("dots-prune: cannot run cliphist list: {error}");
            None
        }
    }
}

fn newest_id(database: &PathBuf) -> Option<String> {
    let list = listing(database)?;
    Some(
        list.lines()
            .next()
            .unwrap_or("")
            .split('\t')
            .next()
            .unwrap_or("")
            .to_string(),
    )
}

/// awk's `$1+0`: optional leading whitespace and sign, then the longest numeric
/// prefix; anything else is zero.
fn numeric_prefix(value: &str) -> f64 {
    let trimmed = value.trim_start();
    let bytes = trimmed.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    while end < bytes.len() {
        let byte = bytes[end];
        if byte.is_ascii_digit() {
            seen_digit = true;
            end += 1;
        } else if (byte == b'+' || byte == b'-') && end == 0 {
            end += 1;
        } else if byte == b'.' && !seen_digit {
            end += 1;
        } else if (byte == b'.' || byte == b'e' || byte == b'E') && seen_digit {
            end += 1;
            if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
                end += 1;
            }
        } else {
            break;
        }
    }
    trimmed[..end].parse::<f64>().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::numeric_prefix;

    #[test]
    fn parses_the_awk_number() {
        assert_eq!(numeric_prefix("12"), 12.0);
        assert_eq!(numeric_prefix(" 12abc"), 12.0);
        assert_eq!(numeric_prefix("12.5"), 12.5);
        assert_eq!(numeric_prefix("-3"), -3.0);
        assert_eq!(numeric_prefix("abc"), 0.0);
        assert_eq!(numeric_prefix(""), 0.0);
    }
}
