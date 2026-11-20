use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Gnome,
    Magick,
}

#[derive(Clone, Debug, Default)]
pub struct GnomeArgs {
    pub img_dirs: Vec<PathBuf>,
    pub size_name: Option<String>,
    pub workers: Option<i64>,
    pub only_images: bool,
    pub recursive: bool,
    pub machine_progress: bool,
    pub help: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MagickArgs {
    pub mode: String,
    pub target: String,
    pub size_name: String,
    pub usage: bool,
}

pub fn dispatch(args: &[String]) -> Mode {
    let name = std::env::args()
        .next()
        .and_then(|path| {
            Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    if name.contains("generate-thumbnails-magick") {
        return Mode::Magick;
    }
    if name.contains("thumbgen") {
        return Mode::Gnome;
    }
    let gnome = args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--machine_progress"
                | "--only_images"
                | "--recursive"
                | "--img_dirs"
                | "--img-dirs"
                | "--workers"
        ) || arg.starts_with("--img_dirs=")
            || arg.starts_with("--img-dirs=")
            || arg.starts_with("--only_images=")
            || arg.starts_with("--recursive=")
            || arg.starts_with("--workers=")
    });
    let magick = args.iter().any(|arg| {
        matches!(arg.as_str(), "--file" | "--directory" | "-f" | "-d")
            || arg.starts_with("--file=")
            || arg.starts_with("--directory=")
    });
    if gnome {
        Mode::Gnome
    } else if magick {
        Mode::Magick
    } else {
        Mode::Gnome
    }
}

pub fn parse_gnome(args: &[String]) -> GnomeArgs {
    let mut parsed = GnomeArgs::default();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        let (flag, inline) = match argument.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value.to_string())),
            _ => (argument.as_str(), None),
        };
        let next = |index: &mut usize| -> Option<String> {
            if let Some(value) = inline.clone() {
                return Some(value);
            }
            *index += 1;
            args.get(*index).cloned()
        };
        match flag {
            "-h" | "--help" => parsed.help = true,
            "-d" | "--img_dirs" | "--img-dirs" => {
                if let Some(value) = next(&mut index) {
                    parsed.img_dirs = value
                        .split_whitespace()
                        .map(PathBuf::from)
                        .collect::<Vec<_>>();
                } else {
                    missing(&flag);
                }
            }
            "-s" | "--size" => {
                if let Some(value) = next(&mut index) {
                    parsed.size_name = Some(value);
                } else {
                    missing(&flag);
                }
            }
            "-w" | "--workers" => {
                if let Some(value) = next(&mut index) {
                    match value.parse::<i64>() {
                        Ok(value) => parsed.workers = Some(value),
                        Err(_) => invalid(&flag, &value),
                    }
                } else {
                    missing(&flag);
                }
            }
            "-i" | "--only_images" | "--only-images" => parsed.only_images = true,
            "-r" | "--recursive" => parsed.recursive = true,
            "--machine_progress" | "--machine-progress" => parsed.machine_progress = true,
            other => unknown(other),
        }
        index += 1;
    }
    if parsed.img_dirs.is_empty() && !parsed.help {
        missing("-d");
    }
    parsed
}

pub fn parse_magick(args: &[String]) -> MagickArgs {
    let mut parsed = MagickArgs {
        size_name: "normal".to_string(),
        ..MagickArgs::default()
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--file" | "-f" => {
                parsed.mode = "file".to_string();
                parsed.target = args.get(index + 1).cloned().unwrap_or_default();
                break;
            }
            "--directory" | "-d" => {
                parsed.mode = "dir".to_string();
                parsed.target = args.get(index + 1).cloned().unwrap_or_default();
                break;
            }
            "--size" | "-s" => {
                parsed.size_name = args.get(index + 1).cloned().unwrap_or_default();
                index += 2;
            }
            _ => {
                parsed.usage = true;
                return parsed;
            }
        }
    }
    if parsed.mode.is_empty() || parsed.target.is_empty() {
        parsed.usage = true;
    }
    parsed
}

fn missing(flag: &str) -> ! {
    eprintln!(
        "Usage: dots-thumbnails [OPTIONS]\nTry 'dots-thumbnails --help' for help.\n\nError: Option '{flag}' requires an argument."
    );
    std::process::exit(2)
}

fn invalid(flag: &str, value: &str) -> ! {
    eprintln!(
        "Usage: dots-thumbnails [OPTIONS]\nTry 'dots-thumbnails --help' for help.\n\nError: Invalid value for '{flag}': '{value}'."
    );
    std::process::exit(2)
}

fn unknown(flag: &str) -> ! {
    eprintln!(
        "Usage: dots-thumbnails [OPTIONS]\nTry 'dots-thumbnails --help' for help.\n\nError: No such option: {flag}"
    );
    std::process::exit(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn chooses_the_cli_dialect_the_caller_uses() {
        assert_eq!(
            dispatch(&args(&[
                "--size",
                "normal",
                "--machine_progress",
                "-d",
                "/tmp"
            ])),
            Mode::Gnome
        );
        assert_eq!(
            dispatch(&args(&["--size", "normal", "-d", "/tmp"])),
            Mode::Magick
        );
        assert_eq!(
            dispatch(&args(&["--size", "normal", "--directory", "/tmp"])),
            Mode::Magick
        );
    }

    #[test]
    fn gnome_values_are_split_like_click() {
        let parsed = parse_gnome(&args(&[
            "-d",
            "one two",
            "-s",
            "large",
            "-w",
            "2",
            "-i",
            "-r",
            "--machine_progress",
        ]));
        assert_eq!(
            parsed.img_dirs,
            [PathBuf::from("one"), PathBuf::from("two")]
        );
        assert_eq!(parsed.size_name.as_deref(), Some("large"));
        assert_eq!(parsed.workers, Some(2));
        assert!(parsed.only_images && parsed.recursive && parsed.machine_progress);
    }

    #[test]
    fn magick_stops_after_the_mode() {
        let parsed = parse_magick(&args(&[
            "-s", "large", "--file", "a.png", "--size", "normal",
        ]));
        assert_eq!(parsed.mode, "file");
        assert_eq!(parsed.target, "a.png");
        assert_eq!(parsed.size_name, "large");
        assert!(!parsed.usage);
    }
}
