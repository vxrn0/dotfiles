mod cli;

use cli::{GnomeArgs, MagickArgs, Mode};
use dots_thumbnails::thumbnails::Factory;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::dispatch(&args) {
        Mode::Gnome => run_gnome(cli::parse_gnome(&args)),
        Mode::Magick => run_magick(cli::parse_magick(&args)),
    }
}

fn run_gnome(args: GnomeArgs) -> i32 {
    if args.help {
        println!(
            "Usage: dots-thumbnails [OPTIONS]\n\n  -d, --img_dirs TEXT  directories separated by spaces\n  -s, --size TEXT      normal, large, x-large, xx-large\n  -w, --workers INTEGER\n  -i, --only_images\n  -r, --recursive\n  --machine_progress"
        );
        return 0;
    }
    let Some(ref size_name) = args.size_name else {
        eprintln!("Usage: dots-thumbnails [OPTIONS]\nTry 'dots-thumbnails --help' for help.\n\nError: Missing option '-d' / '--img_dirs'.");
        return 2;
    };
    if !["normal", "large", "x-large", "xx-large"].contains(&size_name.as_str()) {
        eprintln!(
            "Usage: dots-thumbnails [OPTIONS]\nTry 'dots-thumbnails --help' for help.\n\nError: Invalid value for '-s' / '--size': '{size_name}' is not one of 'normal', 'large', 'x-large', 'xx-large'."
        );
        return 2;
    }
    if args.workers.map(|value| value < 1).unwrap_or(false) {
        eprintln!("dots-thumbnails: workers must be at least 1");
        return 1;
    }
    let factory = match Factory::new(&size_name) {
        Ok(factory) => factory,
        Err(error) => {
            eprintln!("dots-thumbnails: {error}");
            return 1;
        }
    };
    for directory in &args.img_dirs {
        if let Err(error) = thumbnail_folder(&factory, &directory, &args) {
            eprintln!("dots-thumbnails: {error}");
            return 1;
        }
    }
    println!("Thumbnail Generation Completed!");
    0
}

fn thumbnail_folder(factory: &Factory, directory: &Path, args: &GnomeArgs) -> Result<(), String> {
    if !directory.is_dir() {
        let resolved = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
        return Err(format!(
            "{} doesn't exist or isn't a valid directory!",
            resolved.display()
        ));
    }
    let mut files = Vec::new();
    collect_files(directory, args.recursive, &mut files);
    println!(
        "Found {} files in the directory: {}",
        files.len(),
        fs::canonicalize(directory)
            .unwrap_or_else(|_| directory.to_path_buf())
            .display()
    );
    if args.only_images {
        files.retain(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("jpg" | "jpeg" | "png" | "gif")
            )
        });
        println!("Found {} images", files.len());
    }
    let total = files.len();
    for (index, path) in files.iter().enumerate() {
        factory.process(path);
        if args.machine_progress {
            println!("PROGRESS {}/{} FILE {}", index + 1, total, path.display());
        }
    }
    Ok(())
}

fn collect_files(directory: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            out.push(path);
        } else if file_type.is_dir() && recursive {
            collect_files(&path, true, out);
        } else if file_type.is_symlink() {
            if fs::metadata(&path)
                .map(|meta| meta.is_file())
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
}

fn run_magick(args: MagickArgs) -> i32 {
    if args.usage {
        println!(
            "Usage: {} --file <path> | --directory <path>",
            program_name()
        );
        return 1;
    }
    let sizes = [
        ("normal", 128u32),
        ("large", 256),
        ("x-large", 512),
        ("xx-large", 1024),
    ];
    let thumbnail_size = sizes
        .iter()
        .find(|(name, _)| *name == args.size_name)
        .map(|(_, size)| *size)
        .unwrap_or(128);
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| "/".into()));
    let cache = home.join(".cache/thumbnails").join(&args.size_name);
    let target = PathBuf::from(&args.target);

    match args.mode.as_str() {
        "file" => {
            if !target.is_file() {
                println!("File not found: {}", args.target);
                return 2;
            }
            let _ = fs::create_dir_all(&cache);
            generate_magick(&target, thumbnail_size, &cache)
        }
        "dir" => {
            if !target.is_dir() {
                println!("Directory not found: {}", args.target);
                return 2;
            }
            let _ = fs::create_dir_all(&cache);
            let mut entries = fs::read_dir(&target)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            path.file_name()
                                .and_then(|name| name.to_str())
                                .map(|name| !name.starts_with('.'))
                                .unwrap_or(false)
                                && path.is_file()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            entries.sort();
            for path in entries {
                let _ = generate_magick(&path, thumbnail_size, &cache);
            }
            0
        }
        _ => 1,
    }
}

fn generate_magick(source: &Path, size: u32, cache: &Path) -> i32 {
    let Ok(absolute) = fs::canonicalize(source) else {
        return 1;
    };
    let lower = absolute.to_string_lossy().to_ascii_lowercase();
    if [".gif", ".mp4", ".webm", ".mkv", ".avi", ".mov"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        return 0;
    }
    let file_uri = dots_thumbnails::uri::legacy_uri(&absolute);
    let hash = dots_thumbnails::md5::hex(file_uri.as_bytes());
    let output = cache.join(format!("{hash}.png"));
    if output.exists() {
        return 0;
    }
    let _ = fs::create_dir_all(cache);
    let status = Command::new("magick")
        .arg(&absolute)
        .arg("-resize")
        .arg(format!("{size}x{size}"))
        .arg(&output)
        .stdin(Stdio::null())
        .status();
    match status {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("dots-thumbnails: cannot run magick: {error}");
            1
        }
    }
}

fn program_name() -> String {
    std::env::args()
        .next()
        .and_then(|path| {
            Path::new(&path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "dots-thumbnails".to_string())
}
