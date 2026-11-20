use std::env;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

/// Gio's `g_file_new_for_path` canonicalises lexically (no symlink lookup).
pub fn absolute(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    let mut out = PathBuf::from("/");
    for component in joined.components() {
        match component {
            Component::RootDir | Component::Prefix(_) => {}
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// `file://` URI as Gio builds it. The allowed set is the one GLib's
/// `g_filename_to_uri` actually uses: unreserved characters plus
/// `!$&'()*+,=:@/`. Semicolon is escaped.
pub fn file_uri(path: &Path) -> String {
    let path = absolute(path);
    let mut out = String::with_capacity(path.as_os_str().len() + 7);
    out.push_str("file://");
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,=:@/".contains(byte) {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The fallback script builds the URI character by character with bash and
/// `printf '%02X' "'c"`. That prints a codepoint, not a UTF-8 byte, so a
/// non-ASCII path produces `%4E2D` instead of `%E4%B8%AD`. Kept for drop-in
/// compatibility with that script.
pub fn legacy_uri(path: &Path) -> String {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| absolute(path));
    let text = path.to_string_lossy();
    let mut out = String::from("file://");
    for character in text.chars() {
        if character.is_ascii_alphanumeric()
            || matches!(character, '.' | '~' | '_' | '-' | '/' | '(' | ')' | '*')
        {
            out.push(character);
        } else {
            out.push_str(&format!("%{:02X}", character as u32));
        }
    }
    out
}

pub fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(OsStr::to_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gio_escaping_matches_g_filename_to_uri() {
        assert_eq!(file_uri(Path::new("/tmp/a b.png")), "file:///tmp/a%20b.png");
        assert_eq!(
            file_uri(Path::new("/tmp/a#b%c?.png")),
            "file:///tmp/a%23b%25c%3F.png"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/a,b;c=d&e+f.png")),
            "file:///tmp/a,b%3Bc=d&e+f.png"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/ü中.png")),
            "file:///tmp/%C3%BC%E4%B8%AD.png"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/a!$@:.png")),
            "file:///tmp/a!$@:.png"
        );
    }

    #[test]
    fn lexical_absolute_keeps_symlinks_and_removes_dot_segments() {
        env::set_var("HOME", "/home/test");
        let cwd = env::current_dir().unwrap();
        assert_eq!(absolute(Path::new("a/./b/../c")), cwd.join("a/c"));
        assert_eq!(absolute(Path::new("/a//b")), Path::new("/a/b"));
    }

    #[test]
    fn legacy_encoder_matches_the_bash_loop() {
        assert_eq!(legacy_uri(Path::new("/tmp/a b")), "file:///tmp/a%20b");
        assert_eq!(
            legacy_uri(Path::new("/tmp/a,b;c=d&e+f")),
            "file:///tmp/a%2Cb%3Bc%3Dd%26e%2Bf"
        );
        assert_eq!(legacy_uri(Path::new("/tmp/ü中")), "file:///tmp/%FC%4E2D");
        assert_eq!(legacy_uri(Path::new("/tmp/a(b)*")), "file:///tmp/a(b)*");
    }
}
