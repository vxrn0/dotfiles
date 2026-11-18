mod cli;
mod config;
mod monitors;
mod paths;
mod plan;

use config::ShellConfig;
use paths::Paths;
use plan::{Target, Tuning};
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const SCHEMA: &str = "org.gnome.desktop.interface";

fn main() {
    if let Err(error) = run() {
        eprintln!("dots-theme: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let paths = Paths::from_env()?;
    let args: Vec<String> = env::args().skip(1).collect();
    let cli = cli::parse(&args)?;
    let mut config = ShellConfig::load(&paths.shell_config())?;

    if let Some(action) = &cli.color {
        let value = match action {
            cli::ColorArg::Hex(color) => color.clone(),
            cli::ColorArg::Clear => String::new(),
            cli::ColorArg::Pick => pick_color()?,
        };
        config.set_string(&["appearance", "palette", "accentColor"], &value)?;
    }

    let mut color = config
        .scalar(&["appearance", "palette", "accentColor"])
        .filter(|value| cli::is_hex(value));

    let mut raw_type = cli.kind.clone();
    if raw_type.as_deref().is_none_or(str::is_empty) {
        raw_type = Some(
            config
                .scalar(&["appearance", "palette", "type"])
                .unwrap_or_else(|| "auto".to_string()),
        );
    }

    let mut image = cli.image.clone().filter(|value| !value.is_empty());
    if cli.noswitch && image.is_none() {
        image = config
            .scalar(&["wallpaper", "path"])
            .filter(|value| !value.is_empty());
    }
    if image.is_none() && color.is_none() && !cli.noswitch {
        image = pick_wallpaper(&paths)?;
    }

    if image.is_some() && !cli.noswitch {
        config.set_string(&["appearance", "palette", "accentColor"], "")?;
        color = None;
    }

    let detected = match &image {
        Some(path)
            if plan::needs_detection(raw_type.as_deref().unwrap_or("auto"))
                && Path::new(path).is_file() =>
        {
            detect_scheme_type(&paths, path)
        }
        _ => None,
    };
    let (scheme, warnings) =
        plan::resolve_scheme_type(raw_type.as_deref().unwrap_or("auto"), detected.as_deref());
    for warning in warnings {
        eprintln!("{warning}");
    }

    if let (Some(path), Some(mode)) = (&image, cli.mode.as_deref()) {
        if mode == "dark" || mode == "light" {
            image = Some(plan::with_mode_variant(path, mode));
        }
    }

    let mode = cli
        .mode
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(detect_mode);

    let target = match color {
        Some(color) => Some(Target::Color(color)),
        None => image.map(Target::Image),
    };

    switch(&paths, &mut config, target, &mode, &scheme)
}

fn switch(
    paths: &Paths,
    config: &mut ShellConfig,
    target: Option<Target>,
    mode: &str,
    scheme: &str,
) -> Result<(), String> {
    let state_dir = paths.state_dir();
    fs::create_dir_all(&state_dir)
        .map_err(|error| format!("cannot create {}: {error}", state_dir.display()))?;
    let _lock = acquire_lock(&state_dir.join("switchwall.lock"))?;

    let Some(target) = target else {
        println!("Aborted");
        return Ok(());
    };

    let mut color_source = target.clone();
    let mut wallpaper_for_post = String::new();

    if let Target::Image(image) = &target {
        wallpaper_for_post = image.clone();
        kill_existing_mpvpaper()?;

        if plan::is_video(image) {
            let thumbnail_dir = paths.custom_scripts_dir().join("mpvpaper_thumbnails");
            fs::create_dir_all(&thumbnail_dir)
                .map_err(|error| format!("cannot create {}: {error}", thumbnail_dir.display()))?;
            check_video_dependencies()?;
            config.set_string(&["wallpaper", "path"], image)?;

            let monitors = monitors::load()?;
            for name in &monitors.names {
                Command::new("mpvpaper")
                    .arg("-o")
                    .arg(plan::VIDEO_OPTS)
                    .arg(name)
                    .arg(image)
                    .spawn()
                    .map_err(|error| format!("cannot start mpvpaper for {name}: {error}"))?;
                thread::sleep(Duration::from_millis(100));
            }

            let thumbnail = plan::thumbnail_path(&thumbnail_dir, image)?;
            let thumbnail_text = thumbnail.to_string_lossy().into_owned();
            let status = Command::new("ffmpeg")
                .args(["-y", "-i", image, "-vframes", "1", &thumbnail_text])
                .status()
                .map_err(|error| format!("cannot run ffmpeg: {error}"))?;
            config.set_string(&["wallpaper", "thumbnailPath"], &thumbnail_text)?;
            if !status.success() {
                remove_restore(paths)?;
                return Err(format!("ffmpeg failed to extract a thumbnail from {image}"));
            }
            if !thumbnail.is_file() {
                remove_restore(paths)?;
                return Err("Cannot create image to colorgen".to_string());
            }
            color_source = Target::Image(thumbnail_text);
            create_restore_script(paths, image)?;
        } else {
            config.set_string(&["wallpaper", "path"], image)?;
            config.set_string(&["wallpaper", "thumbnailPath"], "")?;
            remove_restore(paths)?;
        }
    }

    pre_process(paths, mode)?;

    if config
        .scalar(&["appearance", "wallpaperTheming", "enableAppsAndShell"])
        .as_deref()
        == Some("false")
    {
        println!("App and shell theming disabled, skipping matugen and color generation");
        return Ok(());
    }

    let tuning = Tuning {
        harmony: config.scalar(&[
            "appearance",
            "wallpaperTheming",
            "terminalGenerationProps",
            "harmony",
        ]),
        harmonize_threshold: config.scalar(&[
            "appearance",
            "wallpaperTheming",
            "terminalGenerationProps",
            "harmonizeThreshold",
        ]),
        term_fg_boost: config.scalar(&[
            "appearance",
            "wallpaperTheming",
            "terminalGenerationProps",
            "termFgBoost",
        ]),
    };

    let matugen = plan::matugen_args(&color_source, mode, scheme);
    run_checked("matugen", &matugen)?;
    generate_colors(paths, config, &color_source, mode, scheme, &tuning)?;
    post_process(paths, &wallpaper_for_post)
}

fn pre_process(paths: &Paths, mode: &str) -> Result<(), String> {
    match mode {
        "dark" => {
            gsettings_set("color-scheme", "prefer-dark")?;
            gsettings_set("gtk-theme", "adw-gtk3-dark")?;
            write_gtk_prefer_dark(paths, true)?;
        }
        "light" => {
            gsettings_set("color-scheme", "prefer-light")?;
            gsettings_set("gtk-theme", "adw-gtk3")?;
            write_gtk_prefer_dark(paths, false)?;
        }
        _ => {}
    }
    fs::create_dir_all(paths.cache_generated_dir()).map_err(|error| {
        format!(
            "cannot create {}: {error}",
            paths.cache_generated_dir().display()
        )
    })
}

fn write_gtk_prefer_dark(paths: &Paths, dark: bool) -> Result<(), String> {
    for directory in ["gtk-3.0", "gtk-4.0"] {
        let directory = paths.config_home.join(directory);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        let file = directory.join("settings.ini");
        let text = format!(
            "[Settings]\ngtk-application-prefer-dark-theme={}\n",
            if dark { 1 } else { 0 }
        );
        fs::write(&file, text)
            .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
    }
    Ok(())
}

fn generate_colors(
    paths: &Paths,
    config: &ShellConfig,
    source: &Target,
    mode: &str,
    scheme: &str,
    tuning: &Tuning,
) -> Result<(), String> {
    let colors = paths.colors_dir();
    let python = python_for(paths);
    let script = colors.join("generate_colors_material.py");
    let terminalscheme = colors.join("terminal/scheme-base.json");
    let cache = paths.generated_dir().join("color.txt");
    let force_dark = config
        .scalar(&[
            "appearance",
            "wallpaperTheming",
            "terminalGenerationProps",
            "forceDarkMode",
        ])
        .as_deref()
        == Some("true");
    let args = plan::generate_args(
        source,
        mode,
        scheme,
        &terminalscheme,
        &cache,
        force_dark,
        tuning,
    );
    let output = Command::new(&python)
        .arg(&script)
        .args(&args)
        .output()
        .map_err(|error| format!("cannot run {}: {error}", script.display()))?;
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let generated = paths.generated_dir();
    let temporary = generated.join("material_colors.scss.tmp");
    let destination = generated.join("material_colors.scss");
    if !output.status.success() {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "{} exited with {}",
            script.display(),
            output.status
        ));
    }
    if output.stdout.is_empty() {
        let _ = fs::remove_file(&temporary);
        return Err("material color generation produced no colors".to_string());
    }
    fs::write(&temporary, &output.stdout)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &destination)
        .map_err(|error| format!("cannot replace {}: {error}", destination.display()))?;

    let applycolor = colors.join("applycolor.sh");
    let status = Command::new(&applycolor)
        .status()
        .map_err(|error| format!("cannot run {}: {error}", applycolor.display()))?;
    if !status.success() {
        return Err(format!("{} exited with {status}", applycolor.display()));
    }
    Ok(())
}

fn post_process(paths: &Paths, wallpaper: &str) -> Result<(), String> {
    let monitors = monitors::load()?;
    let (width, height) = monitors.min_size()?;
    let script = paths.colors_dir().join("code/material-code-set-color.sh");
    Command::new(&script)
        .arg(width.to_string())
        .arg(height.to_string())
        .arg(wallpaper)
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", script.display()))?;
    Ok(())
}

fn acquire_lock(path: &Path) -> Result<fs::File, String> {
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            return Ok(file);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
            return Err(format!("cannot lock {}: {error}", path.display()));
        }
        if Instant::now() >= deadline {
            return Err("Another switchwall is running, giving up".to_string());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn kill_existing_mpvpaper() -> Result<(), String> {
    Command::new("pkill")
        .args(["-x", "mpvpaper"])
        .status()
        .map_err(|error| format!("cannot run pkill: {error}"))?;
    for _ in 0..50 {
        let status = Command::new("pgrep")
            .args(["-x", "mpvpaper"])
            .status()
            .map_err(|error| format!("cannot run pgrep: {error}"))?;
        if !status.success() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Command::new("pkill")
        .args(["-9", "-x", "mpvpaper"])
        .status()
        .map_err(|error| format!("cannot run pkill: {error}"))?;
    Ok(())
}

fn check_video_dependencies() -> Result<(), String> {
    let mut missing = Vec::new();
    for program in ["mpvpaper", "ffmpeg"] {
        if !command_exists(program) {
            missing.push(program);
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    println!("Missing deps: {}", missing.join(" "));
    println!("Arch: sudo pacman -S {}", missing.join(" "));
    Err(format!(
        "missing dependencies for video wallpaper: {}",
        missing.join(", ")
    ))
}

fn detect_mode() -> String {
    match Command::new("gsettings")
        .args(["get", SCHEMA, "color-scheme"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let value = String::from_utf8_lossy(&output.stdout).replace('\'', "");
            if value.trim() == "prefer-light" {
                "light".to_string()
            } else {
                "dark".to_string()
            }
        }
        Ok(output) => {
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }
            eprintln!("dots-theme: warning: gsettings failed; assuming dark mode");
            "dark".to_string()
        }
        Err(error) => {
            eprintln!("dots-theme: warning: cannot run gsettings ({error}); assuming dark mode");
            "dark".to_string()
        }
    }
}

fn gsettings_set(key: &str, value: &str) -> Result<(), String> {
    let status = Command::new("gsettings")
        .args(["set", SCHEMA, key, value])
        .status()
        .map_err(|error| format!("cannot run gsettings: {error}"))?;
    if !status.success() {
        return Err(format!("gsettings set {key} failed: {status}"));
    }
    Ok(())
}

fn detect_scheme_type(paths: &Paths, image: &str) -> Option<String> {
    let script = paths.colors_dir().join("scheme_for_image.py");
    let python = python_for(paths);
    match Command::new(&python).arg(&script).arg(image).output() {
        Ok(output) => {
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Some(String::from_utf8_lossy(&output.stdout).replace('\n', ""))
        }
        Err(error) => {
            eprintln!(
                "dots-theme: warning: cannot run {}: {error}",
                script.display()
            );
            None
        }
    }
}

fn pick_color() -> Result<String, String> {
    let output = Command::new("hyprpicker")
        .arg("--no-fancy")
        .output()
        .map_err(|error| format!("cannot run hyprpicker: {error}"))?;
    if !output.status.success() {
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        return Err(format!("hyprpicker exited with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string())
}

fn pick_wallpaper(paths: &Paths) -> Result<Option<String>, String> {
    let pictures = Command::new("xdg-user-dir")
        .arg("PICTURES")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim_end_matches('\n')
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    let directory = pictures
        .map(|base| base.join("Wallpapers/showcase"))
        .filter(|candidate| candidate.is_dir())
        .or_else(|| {
            let extra = paths.home.join("extra");
            extra.is_dir().then_some(extra)
        })
        .unwrap_or_else(|| paths.home.clone());
    if !directory.is_dir() {
        return Err(format!(
            "no directory to pick a wallpaper from: {}",
            directory.display()
        ));
    }

    let mut command = if command_exists("kdialog") {
        let mut command = Command::new("kdialog");
        command
            .arg("--getopenfilename")
            .arg(".")
            .arg("--title")
            .arg("Choose wallpaper");
        command
    } else {
        let mut command = Command::new("zenity");
        command
            .arg("--file-selection")
            .arg("--title=Choose wallpaper")
            .arg(format!("--filename={}/", directory.display()));
        command
    };
    command.current_dir(&directory);
    let output = command
        .output()
        .map_err(|error| format!("cannot run the wallpaper picker: {error}"))?;
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        return Err(format!("wallpaper picker exited with {}", output.status));
    }
    let selection = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string();
    Ok(if selection.is_empty() {
        None
    } else {
        Some(selection)
    })
}

fn create_restore_script(paths: &Paths, video: &str) -> Result<(), String> {
    let date = Command::new("date")
        .output()
        .map_err(|error| format!("cannot run date: {error}"))?;
    if !date.status.success() {
        return Err(format!("date exited with {}", date.status));
    }
    let date = String::from_utf8_lossy(&date.stdout)
        .trim_end_matches('\n')
        .to_string();
    write_restore(paths, &plan::restore_video_script(video, &date), true)
}

fn remove_restore(paths: &Paths) -> Result<(), String> {
    write_restore(paths, plan::EMPTY_RESTORE, false)
}

fn write_restore(paths: &Paths, script: &str, executable: bool) -> Result<(), String> {
    let path = paths
        .custom_scripts_dir()
        .join("__restore_video_wallpaper.sh");
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, script)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))?;
    if executable {
        let mode = fs::metadata(&path)
            .map_err(|error| format!("cannot stat {}: {error}", path.display()))?
            .permissions()
            .mode();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode | 0o111))
            .map_err(|error| format!("cannot chmod {}: {error}", path.display()))?;
    }
    Ok(())
}

fn run_checked(program: &str, args: &[String]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    Ok(())
}

fn python_for(paths: &Paths) -> PathBuf {
    if let Some(venv) = env::var_os("DOTFILES_VIRTUAL_ENV").filter(|value| !value.is_empty()) {
        return PathBuf::from(venv).join("bin/python3");
    }
    let local = paths.home.join(".local/state/quickshell/.venv/bin/python3");
    if local.is_file() {
        return local;
    }
    PathBuf::from("python3")
}

fn command_exists(program: &str) -> bool {
    env::var_os("PATH")
        .map(|path| {
            env::split_paths(&path).any(|directory| {
                let candidate = directory.join(program);
                fs::metadata(&candidate)
                    .map(|metadata| {
                        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}
