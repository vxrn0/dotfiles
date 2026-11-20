use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const KEY: &str = "\"material-code.primaryColor\"";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".config"));
    let state_home = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| home.join(".local/state"));
    let color_file = state_home.join("quickshell/user/generated/color.txt");
    let new_color = match fs::read_to_string(&color_file) {
        Ok(text) => text.trim_end_matches('\n').to_string(),
        Err(error) => {
            eprintln!(
                "dots-material-code: cannot read {}: {error}",
                color_file.display()
            );
            String::new()
        }
    };

    let settings = [
        "Code/User/settings.json",
        "VSCodium/User/settings.json",
        "Code - OSS/User/settings.json",
        "Code - Insiders/User/settings.json",
        "Cursor/User/settings.json",
        "Antigravity/User/settings.json",
        "Windsurf/User/settings.json",
    ];
    for relative in settings {
        let path = config_home.join(relative);
        if !path.is_file() {
            continue;
        }
        match fs::read(&path) {
            Ok(bytes) => {
                let updated = update(&bytes, &new_color);
                if let Err(error) = write_atomic(&path, &updated) {
                    eprintln!(
                        "dots-material-code: cannot write {}: {error}",
                        path.display()
                    );
                }
            }
            Err(error) => eprintln!(
                "dots-material-code: cannot read {}: {error}",
                path.display()
            ),
        }
    }
    0
}

fn update(bytes: &[u8], new_color: &str) -> Vec<u8> {
    if contains(bytes, KEY.as_bytes()) {
        let text = String::from_utf8_lossy(bytes);
        let mut out = String::with_capacity(text.len());
        for line in text.split_inclusive('\n') {
            out.push_str(&replace_first_in_line(line, new_color));
        }
        out.into_bytes()
    } else {
        append_key(bytes, new_color)
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

/// `s/("material-code.primaryColor"\s*:\s*")[^"]*(")/\1<new>\2/` without `g`.
fn replace_first_in_line(line: &str, new_color: &str) -> String {
    let Some(key_at) = line.find(KEY) else {
        return line.to_string();
    };
    let bytes = line.as_bytes();
    let mut index = key_at + KEY.len();
    while index < bytes.len() && is_space(bytes[index]) {
        index += 1;
    }
    if index >= bytes.len() || bytes[index] != b':' {
        return line.to_string();
    }
    index += 1;
    while index < bytes.len() && is_space(bytes[index]) {
        index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'"' {
        return line.to_string();
    }
    let value_start = index + 1;
    let Some(relative_end) = line[value_start..].find('"') else {
        return line.to_string();
    };
    let value_end = value_start + relative_end;
    let mut out = String::with_capacity(line.len() + new_color.len());
    out.push_str(&line[..value_start]);
    out.push_str(new_color);
    out.push_str(&line[value_end..]);
    out
}

/// `$ s/}/,\n  "material-code.primaryColor": "<colour>"\n}/` on the last line.
fn append_key(bytes: &[u8], new_color: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return bytes.to_vec();
    }
    let last = lines.len() - 1;
    let (body, newline) = match lines[last].strip_suffix('\n') {
        Some(body) => (body, true),
        None => (lines[last].as_str(), false),
    };
    let Some(brace) = body.find('}') else {
        return bytes.to_vec();
    };
    let mut replaced = String::with_capacity(body.len() + new_color.len() + 64);
    replaced.push_str(&body[..brace]);
    replaced.push_str(",\n  \"material-code.primaryColor\": \"");
    replaced.push_str(new_color);
    replaced.push_str("\"\n}");
    replaced.push_str(&body[brace + 1..]);
    lines[last] = replaced;
    let mut out = String::new();
    for line in &lines {
        out.push_str(line);
    }
    if newline {
        out.push('\n');
    }
    out.into_bytes()
}

/// sed -i replaces the directory entry, so a symlinked settings file becomes a
/// regular file and the link target is untouched. Keep that observable detail.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mode = fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o7777)
        .unwrap_or(0o644);
    let temp = parent.join(format!(".material-code.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temp);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    file.write_all(data)?;
    file.flush()?;
    fs::set_permissions(&temp, fs::Permissions::from_mode(mode))?;
    fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_the_first_value_on_each_line() {
        let input = b"{\n  \"material-code.primaryColor\" : \"#111111\", \"material-code.primaryColor\": \"#222222\"\n}\n";
        let output = String::from_utf8(update(input, "#abcdef")).unwrap();
        assert_eq!(
            output,
            "{\n  \"material-code.primaryColor\" : \"#abcdef\", \"material-code.primaryColor\": \"#222222\"\n}\n"
        );
    }

    #[test]
    fn appends_like_the_sed_script() {
        let output = String::from_utf8(update(b"{\n}\n", "#abcdef")).unwrap();
        assert_eq!(
            output,
            "{\n,\n  \"material-code.primaryColor\": \"#abcdef\"\n}\n"
        );
        assert_eq!(
            update(b"{}", "#abcdef"),
            b"{,\n  \"material-code.primaryColor\": \"#abcdef\"\n}"
        );
        assert_eq!(
            update(b"{\n}", "#abcdef"),
            b"{\n,\n  \"material-code.primaryColor\": \"#abcdef\"\n}"
        );
        assert_eq!(update(b"", "#abcdef"), b"");
        assert_eq!(update(b"{\n\"x\": 1\n", "#abcdef"), b"{\n\"x\": 1\n");
    }

    #[test]
    fn a_key_without_a_closing_quote_is_untouched() {
        assert_eq!(
            update(b"{\"material-code.primaryColor\": \"oops}\n", "#abcdef"),
            b"{\"material-code.primaryColor\": \"oops}\n"
        );
    }
}
