use crate::{md5, png, uri};
use std::env;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const NORMAL: u32 = 128;
pub const LARGE: u32 = 256;
pub const X_LARGE: u32 = 512;
pub const XX_LARGE: u32 = 1024;

#[derive(Clone, Debug)]
struct Thumbnailer {
    mime_types: Vec<String>,
    command: String,
}

pub struct Factory {
    pub size_name: &'static str,
    pub size: u32,
    cache_home: PathBuf,
    data_home: PathBuf,
    data_dirs: Vec<PathBuf>,
    thumbnailers: Vec<Thumbnailer>,
}

impl Factory {
    pub fn new(size_name: &str) -> Result<Self, String> {
        let (size_name, size) = match size_name {
            "normal" => ("normal", NORMAL),
            "large" => ("large", LARGE),
            "x-large" => ("x-large", X_LARGE),
            "xx-large" => ("xx-large", XX_LARGE),
            other => return Err(format!("invalid thumbnail size '{other}'")),
        };
        let home = PathBuf::from(env::var_os("HOME").unwrap_or_else(|| "/".into()));
        let cache_home = env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| home.join(".cache"));
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| home.join(".local/share"));
        let data_dirs = env::var("XDG_DATA_DIRS")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".to_string())
            .split(':')
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .collect();
        let mut factory = Self {
            size_name,
            size,
            cache_home,
            data_home,
            data_dirs,
            thumbnailers: Vec::new(),
        };
        factory.load_thumbnailers();
        if std::env::var_os("DOTS_THUMBNAILS_DEBUG").is_some() {
            eprintln!(
                "dots-thumbnails: {} thumbnailers",
                factory.thumbnailers.len()
            );
        }
        Ok(factory)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache_home.join("thumbnails").join(self.size_name)
    }

    pub fn failed_dir(&self) -> PathBuf {
        self.cache_home
            .join("thumbnails/fail/gnome-thumbnail-factory")
    }

    pub fn thumbnail_path(&self, file_uri: &str) -> PathBuf {
        self.cache_dir()
            .join(format!("{}.png", md5::hex(file_uri.as_bytes())))
    }

    pub fn lookup(&self, file_uri: &str, mtime: i64) -> Option<PathBuf> {
        let path = self.thumbnail_path(file_uri);
        let bytes = fs::read(&path).ok()?;
        png::valid(&bytes, file_uri, mtime).then_some(path)
    }

    pub fn has_valid_failed_thumbnail(&self, file_uri: &str, mtime: i64) -> bool {
        let path = self
            .failed_dir()
            .join(format!("{}.png", md5::hex(file_uri.as_bytes())));
        fs::read(path)
            .map(|bytes| png::valid(&bytes, file_uri, mtime))
            .unwrap_or(false)
    }

    pub fn can_thumbnail(&self, file_uri: &str, mime_type: &str, mtime: i64) -> bool {
        if file_uri.starts_with("file:/") {
            let thumbnails = self.cache_home.join("thumbnails");
            let uri_path = file_uri.trim_start_matches("file://");
            if Path::new(uri_path).starts_with(&thumbnails) {
                return false;
            }
        }
        self.thumbnailer_for(mime_type).is_some()
            && !self.has_valid_failed_thumbnail(file_uri, mtime)
    }

    fn thumbnailer_for(&self, mime_type: &str) -> Option<&Thumbnailer> {
        self.thumbnailers.iter().find(|thumbnailer| {
            thumbnailer
                .mime_types
                .iter()
                .any(|value| value == mime_type)
        })
    }

    fn load_thumbnailers(&mut self) {
        let mut directories = vec![self.data_home.join("thumbnailers")];
        directories.extend(self.data_dirs.iter().map(|dir| dir.join("thumbnailers")));
        for directory in directories {
            let Ok(entries) = fs::read_dir(directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) != Some("thumbnailer") {
                    continue;
                }
                if let Some(thumbnailer) = parse_thumbnailer(&path) {
                    if self.thumbnailer_for(&thumbnailer.mime_types[0]).is_none() {
                        self.thumbnailers.push(thumbnailer);
                    } else {
                        for mime in &thumbnailer.mime_types {
                            if self.thumbnailer_for(mime).is_none() {
                                self.thumbnailers.push(thumbnailer.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }
        // The loop above can insert duplicates when the first MIME was taken but
        // a later one was free; registration order is all that matters.
        let mut seen = Vec::new();
        self.thumbnailers.retain(|thumbnailer| {
            let useful = thumbnailer
                .mime_types
                .iter()
                .any(|mime| !seen.contains(mime));
            seen.extend(thumbnailer.mime_types.iter().cloned());
            useful
        });
    }

    /// Mirrors thumbgen.py: lookup, can_thumbnail, generate, save. Debug
    /// messages are suppressed, like the reference's INFO-level stdout logger.
    pub fn process(&self, path: &Path) -> bool {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);
        let file_uri = uri::file_uri(path);
        if self.lookup(&file_uri, mtime).is_some() {
            return false;
        }
        let mime_type = mime_for(path);
        if !self.can_thumbnail(&file_uri, &mime_type, mtime) {
            return false;
        }
        let Some(thumbnailer) = self.thumbnailer_for(&mime_type) else {
            return false;
        };
        let Some(thumbnail) = generate(thumbnailer, path, &mime_type, self.size) else {
            return false;
        };
        let Some(tagged) = png::with_metadata(&thumbnail, &file_uri, mtime) else {
            if std::env::var_os("DOTS_THUMBNAILS_DEBUG").is_some() {
                eprintln!("tagging failed");
            }
            return false;
        };
        let saved = save(&self.thumbnail_path(&file_uri), &tagged);
        if std::env::var_os("DOTS_THUMBNAILS_DEBUG").is_some() {
            eprintln!("save: {saved:?}");
        }
        saved.is_ok()
    }

    pub fn generate_only(&self, path: &Path) -> Option<Vec<u8>> {
        let mime_type = mime_for(path);
        let thumbnailer = self.thumbnailer_for(&mime_type)?;
        generate(thumbnailer, path, &mime_type, self.size)
    }
}

fn parse_thumbnailer(path: &Path) -> Option<Thumbnailer> {
    let text = fs::read_to_string(path).ok()?;
    let mut mime_types = Vec::new();
    let mut try_exec = None;
    let mut command = None;
    let mut in_entry = false;
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let mut line = lines[index].trim().to_string();
        index += 1;
        while line.ends_with('\\') && index < lines.len() {
            line.pop();
            line.push_str(lines[index].trim());
            index += 1;
        }
        if line.starts_with('[') {
            in_entry = line == "[Thumbnailer Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "MimeType" => {
                    mime_types = value
                        .split(';')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                "TryExec" => try_exec = Some(value.trim().to_string()),
                "Exec" => command = Some(value.trim().to_string()),
                _ => {}
            }
        }
    }
    if mime_types.is_empty() || command.is_none() {
        return None;
    }
    if let Some(try_exec) = try_exec {
        if find_in_path(&try_exec).is_none() {
            return None;
        }
    }
    Some(Thumbnailer {
        mime_types,
        command: command.unwrap(),
    })
}

fn generate(
    thumbnailer: &Thumbnailer,
    source: &Path,
    mime_type: &str,
    size: u32,
) -> Option<Vec<u8>> {
    let (arguments, output, workdir) =
        expand_command(&thumbnailer.command, source, mime_type, size)?;
    let mut command = Command::new(&arguments[0]);
    command.args(&arguments[1..]);
    command.stdout(Stdio::inherit()).stderr(Stdio::piped());
    let mut child = command.spawn().ok()?;
    let status = child.wait().ok()?;
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let bytes = fs::read(&output).ok();
    if std::env::var_os("DOTS_THUMBNAILS_DEBUG").is_some() {
        eprintln!(
            "dots-thumbnails: thumbnailer {status}: {stderr} output={:?}",
            bytes.as_ref().map(Vec::len)
        );
    }
    let _ = fs::remove_dir_all(&workdir);
    if !status.success() {
        return None;
    }
    bytes.filter(|value| !value.is_empty())
}

fn expand_command(
    template: &str,
    source: &Path,
    mime_type: &str,
    size: u32,
) -> Option<(Vec<String>, PathBuf, PathBuf)> {
    let elements = split_command(template)?;
    let base = env::temp_dir().join(format!(
        "gnome-desktop-thumbnailer-{}-{}",
        std::process::id(),
        now_nanos()
    ));
    fs::create_dir(&base).ok()?;
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).ok()?;
    let extension = uri::extension(source).unwrap_or("");
    let input_name = if extension.is_empty() {
        "gnome-desktop-file-to-thumbnail".to_string()
    } else {
        format!("gnome-desktop-file-to-thumbnail.{extension}")
    };
    let input_host = base.join(&input_name);
    let output_host = base.join("gnome-desktop-thumbnailer.png");
    let bwrap = find_in_path("bwrap");
    // Inside bubblewrap /tmp is the scratch directory. Without bubblewrap the
    // same paths live in the host scratch directory.
    let input_arg = if bwrap.is_some() {
        PathBuf::from("/tmp").join(&input_name)
    } else {
        input_host.clone()
    };
    let output_arg = if bwrap.is_some() {
        PathBuf::from("/tmp/gnome-desktop-thumbnailer.png")
    } else {
        output_host.clone()
    };
    if bwrap.is_none() {
        let source_absolute = uri::absolute(source);
        if std::os::unix::fs::symlink(&source_absolute, &input_host).is_err() {
            fs::copy(source, &input_host).ok()?;
        }
    }
    let input_uri = uri::file_uri(&input_arg);

    let mut got_input = false;
    let mut got_output = false;
    let mut expanded = Vec::with_capacity(elements.len());
    for element in elements {
        let mut value = String::new();
        let mut characters = element.chars().peekable();
        while let Some(character) = characters.next() {
            if character != '%' {
                value.push(character);
                continue;
            }
            match characters.next() {
                Some('u') => {
                    value.push_str(&input_uri);
                    got_input = true;
                }
                Some('i') => {
                    value.push_str(&input_arg.to_string_lossy());
                    got_input = true;
                }
                Some('o') => {
                    value.push_str(&output_arg.to_string_lossy());
                    got_output = true;
                }
                Some('s') => value.push_str(&size.to_string()),
                Some('m') => value.push_str(mime_type),
                Some('%') => value.push('%'),
                Some(other) => {
                    value.push('%');
                    value.push(other);
                }
                None => value.push('%'),
            }
        }
        expanded.push(value);
    }
    if !got_input || !got_output || expanded.is_empty() {
        let _ = fs::remove_dir_all(&base);
        return None;
    }

    let program = find_in_path(&expanded[0]).unwrap_or_else(|| PathBuf::from(&expanded[0]));
    let Some(bwrap) = bwrap else {
        let mut command = expanded;
        command[0] = program.to_string_lossy().into_owned();
        return Some((command, output_host, base));
    };

    let source_absolute = uri::absolute(source);
    let mut command = vec![
        bwrap.to_string_lossy().into_owned(),
        "--ro-bind".into(),
        "/usr".into(),
        "/usr".into(),
        "--ro-bind-try".into(),
        "/etc".into(),
        "/etc".into(),
    ];
    for name in ["bin", "lib64", "lib", "sbin"] {
        let directory = PathBuf::from("/").join(name);
        if !directory.exists() {
            continue;
        }
        let usr = PathBuf::from("/usr").join(name);
        let usrmerged = fs::metadata(&directory)
            .ok()
            .zip(fs::metadata(&usr).ok())
            .map(|(left, right)| {
                use std::os::unix::fs::MetadataExt;
                left.dev() == right.dev() && left.ino() == right.ino()
            })
            .unwrap_or(false);
        if usrmerged {
            command.extend([
                "--symlink".into(),
                format!("usr/{name}"),
                format!("/{name}"),
            ]);
        } else {
            command.extend([
                "--ro-bind".into(),
                directory.to_string_lossy().into_owned(),
                directory.to_string_lossy().into_owned(),
            ]);
        }
    }
    command.extend([
        "--proc".into(),
        "/proc".into(),
        "--dev".into(),
        "/dev".into(),
        "--unshare-all".into(),
        "--die-with-parent".into(),
        "--bind".into(),
        base.to_string_lossy().into_owned(),
        "/tmp".into(),
        "--ro-bind".into(),
        source_absolute.to_string_lossy().into_owned(),
        input_arg.to_string_lossy().into_owned(),
        "--chdir".into(),
        "/".into(),
        "--setenv".into(),
        "GIO_USE_VFS".into(),
        "local".into(),
    ]);
    if let Some(parent) = program.parent() {
        let parent_text = parent.to_string_lossy().into_owned();
        if !parent_text.is_empty()
            && !parent.starts_with("/usr")
            && !parent.starts_with("/lib")
            && !parent.starts_with("/bin")
            && !parent.starts_with("/sbin")
        {
            command.extend(["--ro-bind".into(), parent_text.clone(), parent_text.clone()]);
        }
    }
    command.push(program.to_string_lossy().into_owned());
    command.extend(expanded.into_iter().skip(1));
    Some((command, output_host, base))
}

fn save(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent", path.display()))?;
    let existed = parent.exists();
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    if !existed {
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }
    let temp = parent.join(format!(".thumb.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&temp);
    fs::write(&temp, data).map_err(|error| format!("cannot write {}: {error}", temp.display()))?;
    fs::set_permissions(&temp, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("cannot chmod {}: {error}", temp.display()))?;
    fs::rename(&temp, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))?;
    Ok(())
}

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn find_in_path(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 {
        return is_executable(path).then(|| path.to_path_buf());
    }
    for directory in env::var_os("PATH")?.to_string_lossy().split(':') {
        let candidate = Path::new(directory).join(program);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// A small `g_shell_parse_argv`: whitespace splitting with single quotes,
/// double quotes and backslash escapes.
pub fn split_command(command: &str) -> Option<Vec<String>> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut characters = command.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            ' ' | '\t' | '\n' => {
                if started {
                    arguments.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            '\'' => {
                started = true;
                loop {
                    match characters.next()? {
                        '\'' => break,
                        other => current.push(other),
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match characters.next()? {
                        '"' => break,
                        '\\' => match characters.next()? {
                            '\n' => {}
                            other => current.push(other),
                        },
                        other => current.push(other),
                    }
                }
            }
            '\\' => {
                started = true;
                if let Some(next) = characters.next() {
                    if next != '\n' {
                        current.push(next);
                    }
                }
            }
            other => {
                started = true;
                current.push(other);
            }
        }
    }
    if started {
        arguments.push(current);
    }
    if arguments.is_empty() {
        None
    } else {
        Some(arguments)
    }
}

fn mime_for(path: &Path) -> String {
    let mut header = Vec::new();
    if let Ok(mut file) = fs::File::open(path) {
        let _ = file.by_ref().take(4096).read_to_end(&mut header);
    }
    let magic = if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if header.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if header.len() >= 12 && &header[..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        Some("image/webp")
    } else if header.starts_with(b"II*\0") || header.starts_with(b"MM\0*") {
        Some("image/tiff")
    } else if header.starts_with(b"BM") {
        Some("image/bmp")
    } else if header.starts_with(b"\0\0\x01\0") {
        Some("image/x-icon")
    } else if header.starts_with(b"\x1aE\xdf\xa3") {
        Some("video/webm")
    } else if header.starts_with(b"\x00\x00\x00") && header.get(4..8) == Some(b"ftyp") {
        match &header[8..12] {
            b"avif" | b"avis" => Some("image/avif"),
            b"heic" | b"heix" | b"mif1" => Some("image/heif"),
            _ => Some("video/mp4"),
        }
    } else if header.starts_with(b"\xff\x0a") || header.starts_with(b"\0\0\0\x0cJXL ") {
        Some("image/jxl")
    } else if looks_like_svg(&header) {
        Some("image/svg+xml")
    } else {
        None
    };
    if let Some(mime) = magic {
        return mime.to_string();
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "heic" | "heif" => "image/heif",
        "jxl" => "image/jxl",
        "ico" => "image/x-icon",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    };
    mime.to_string()
}

fn looks_like_svg(header: &[u8]) -> bool {
    let text = String::from_utf8_lossy(header);
    let trimmed = text.trim_start();
    if !(trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") || trimmed.starts_with("<!--"))
    {
        return false;
    }
    text.contains("<svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thumbnailer_entries() {
        let directory = env::temp_dir().join(format!("dots-thumbnailer-{}", std::process::id()));
        let _ = fs::create_dir_all(&directory);
        let path = directory.join("test.thumbnailer");
        fs::write(
            &path,
            "[Thumbnailer Entry]\nTryExec=/bin/sh\nExec=/bin/sh %i %o %s %u %m\nMimeType=image/png;image/jpeg;\n",
        )
        .unwrap();
        let thumbnailer = parse_thumbnailer(&path).unwrap();
        assert_eq!(thumbnailer.mime_types, ["image/png", "image/jpeg"]);
        assert_eq!(thumbnailer.command, "/bin/sh %i %o %s %u %m");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn shell_parser_handles_quotes() {
        assert_eq!(
            split_command("prog 'a b' \"c d\" e\\ f").unwrap(),
            ["prog", "a b", "c d", "e f"]
        );
        assert!(split_command("   ").is_none());
        assert!(split_command("'unterminated").is_none());
    }

    #[test]
    fn sniffs_the_formats_that_have_thumbnailers() {
        let directory = env::temp_dir().join(format!("dots-mime-{}", std::process::id()));
        let _ = fs::create_dir_all(&directory);
        let png = directory.join("a.bin");
        fs::write(&png, b"\x89PNG\r\n\x1a\nrest").unwrap();
        assert_eq!(mime_for(&png), "image/png");
        let svg = directory.join("a.bin");
        fs::write(&svg, b"<?xml version=\"1.0\"?><svg></svg>").unwrap();
        assert_eq!(mime_for(&svg), "image/svg+xml");
        let text = directory.join("a.txt");
        fs::write(&text, b"hello").unwrap();
        assert_eq!(mime_for(&text), "application/octet-stream");
        let _ = fs::remove_dir_all(directory);
    }
}
