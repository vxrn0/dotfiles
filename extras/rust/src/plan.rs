use std::path::{Path, PathBuf};

pub const VIDEO_OPTS: &str = "no-audio loop hwdec=auto scale=bilinear interpolation=no video-sync=display-resample panscan=1.0 video-scale-x=1.0 video-scale-y=1.0 video-align-x=0.5 video-align-y=0.5 load-scripts=no";

pub const SCHEME_TYPES: [&str; 9] = [
    "scheme-content",
    "scheme-expressive",
    "scheme-fidelity",
    "scheme-fruit-salad",
    "scheme-monochrome",
    "scheme-neutral",
    "scheme-rainbow",
    "scheme-tonal-spot",
    "auto",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Color(String),
    Image(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tuning {
    pub harmony: Option<String>,
    pub harmonize_threshold: Option<String>,
    pub term_fg_boost: Option<String>,
}

pub fn is_video(path: &str) -> bool {
    match path.rsplit_once('.') {
        Some((_, extension)) => matches!(extension, "mp4" | "webm" | "mkv" | "avi" | "mov"),
        None => false,
    }
}

pub fn thumbnail_path(directory: &Path, video: &str) -> Result<PathBuf, String> {
    let name = Path::new(video)
        .file_name()
        .ok_or_else(|| format!("{video} has no file name"))?;
    let mut thumbnail = name.to_os_string();
    thumbnail.push(".jpg");
    Ok(directory.join(thumbnail))
}

/// Reproduces the -dark/-light variant lookup in the script, including its
/// string handling for names without an extension.
pub fn with_mode_variant(image: &str, mode: &str) -> String {
    let path = Path::new(image);
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().into_owned(),
        _ => ".".to_string(),
    };
    let base = match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => image.to_string(),
    };
    let (name, extension) = match base.rfind('.') {
        Some(index) => (&base[..index], &base[index + 1..]),
        None => (base.as_str(), base.as_str()),
    };
    let mut stripped = name.strip_suffix("-dark").unwrap_or(name);
    stripped = stripped.strip_suffix("-light").unwrap_or(stripped);
    let candidate = format!("{directory}/{stripped}-{mode}.{extension}");
    let fallback = format!("{directory}/{stripped}.{extension}");
    if Path::new(&candidate).is_file() {
        candidate
    } else if Path::new(&fallback).is_file() {
        fallback
    } else {
        image.to_string()
    }
}

/// True when the script would replace the requested type with a detected one,
/// either because it is "auto" or because it is invalid and falls back to auto.
pub fn needs_detection(raw: &str) -> bool {
    raw == "auto" || !SCHEME_TYPES.contains(&raw)
}

/// Validates the requested type and resolves "auto" to a detected type.
/// The script never leaves "auto" in place.
pub fn resolve_scheme_type(raw: &str, detected: Option<&str>) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut kind = raw.to_string();
    if !SCHEME_TYPES.contains(&kind.as_str()) {
        warnings.push(format!(
            "[switchwall.sh] Warning: Invalid type '{kind}', defaulting to 'auto'"
        ));
        kind = "auto".to_string();
    }
    if kind == "auto" {
        if let Some(found) =
            detected.filter(|value| *value != "auto" && SCHEME_TYPES.contains(value))
        {
            return (found.to_string(), warnings);
        }
        warnings.push(
            "[switchwall] Warning: Could not auto-detect a valid scheme, defaulting to 'scheme-tonal-spot'"
                .to_string(),
        );
        kind = "scheme-tonal-spot".to_string();
    }
    (kind, warnings)
}

pub fn matugen_args(source: &Target, mode: &str, scheme: &str) -> Vec<String> {
    let mut args = vec!["--source-color-index".to_string(), "0".to_string()];
    match source {
        Target::Color(color) => {
            args.push("color".to_string());
            args.push("hex".to_string());
            args.push(color.clone());
        }
        Target::Image(image) => {
            args.push("image".to_string());
            args.push(image.clone());
        }
    }
    args.push("--mode".to_string());
    args.push(mode.to_string());
    args.push("--type".to_string());
    args.push(scheme.to_string());
    args
}

pub fn generate_args(
    source: &Target,
    mode: &str,
    scheme: &str,
    terminalscheme: &Path,
    cache: &Path,
    force_dark: bool,
    tuning: &Tuning,
) -> Vec<String> {
    let mut args = Vec::new();
    match source {
        Target::Color(color) => {
            args.push("--color".to_string());
            args.push(color.clone());
        }
        Target::Image(image) => {
            args.push("--path".to_string());
            args.push(image.clone());
        }
    }
    args.push("--mode".to_string());
    args.push(if force_dark { "dark" } else { mode }.to_string());
    args.push("--scheme".to_string());
    args.push(scheme.to_string());
    args.push("--termscheme".to_string());
    args.push(terminalscheme.to_string_lossy().into_owned());
    args.push("--blend_bg_fg".to_string());
    args.push("--cache".to_string());
    args.push(cache.to_string_lossy().into_owned());
    if let Some(value) = &tuning.harmony {
        args.push("--harmony".to_string());
        args.push(value.clone());
    }
    if let Some(value) = &tuning.harmonize_threshold {
        args.push("--harmonize_threshold".to_string());
        args.push(value.clone());
    }
    if let Some(value) = &tuning.term_fg_boost {
        args.push("--term_fg_boost".to_string());
        args.push(value.clone());
    }
    args
}

pub const EMPTY_RESTORE: &str = "#!/bin/bash\n# The content of this script will be generated by switchwall.sh - Don't modify it by yourself.\n";

pub fn restore_video_script(video: &str, date: &str) -> String {
    format!(
        "#!/bin/bash\n\
# Generated by switchwall.sh - Don't modify it by yourself.\n\
# Time: {date}\n\
\n\
pkill -f -9 mpvpaper\n\
\n\
for monitor in $(hyprctl monitors -j | jq -r '.[] | .name'); do\n    \
mpvpaper -o \"{VIDEO_OPTS}\" \"$monitor\" \"{video}\" &\n    \
sleep 0.1\n\
done\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_detection_is_lowercase_extension_only() {
        for path in ["a.mp4", "/x/y.webm", "clip.mkv", "clip.avi", "clip.mov"] {
            assert!(is_video(path), "{path}");
        }
        for path in ["a.MP4", "a.png", "a", "/x.mp4/y.png", "a.mp4/"] {
            assert!(!is_video(path), "{path}");
        }
    }

    #[test]
    fn thumbnail_keeps_the_original_file_name() {
        let path = thumbnail_path(Path::new("/thumbs"), "/videos/clip.mp4").unwrap();
        assert_eq!(path, Path::new("/thumbs/clip.mp4.jpg"));
    }

    #[test]
    fn scheme_defaults_and_validation() {
        assert!(needs_detection("auto"));
        assert!(needs_detection("nonsense"));
        assert!(needs_detection(""));
        assert!(!needs_detection("scheme-neutral"));
        assert_eq!(
            resolve_scheme_type("scheme-expressive", None).0,
            "scheme-expressive"
        );
        assert_eq!(
            resolve_scheme_type("auto", Some("scheme-neutral")).0,
            "scheme-neutral"
        );
        assert_eq!(resolve_scheme_type("auto", None).0, "scheme-tonal-spot");
        assert_eq!(
            resolve_scheme_type("auto", Some("auto")).0,
            "scheme-tonal-spot"
        );
        assert_eq!(
            resolve_scheme_type("auto", Some("nonsense")).0,
            "scheme-tonal-spot"
        );
        assert_eq!(resolve_scheme_type("nonsense", None).0, "scheme-tonal-spot");
        assert_eq!(resolve_scheme_type("", None).0, "scheme-tonal-spot");
        for (kind, _) in [
            resolve_scheme_type("nonsense", Some("scheme-neutral")),
            resolve_scheme_type("auto", None),
        ] {
            assert_ne!(kind, "auto");
        }
    }

    #[test]
    fn matugen_argument_order_matches_the_script() {
        let color = Target::Color("#336699".into());
        assert_eq!(
            matugen_args(&color, "dark", "scheme-tonal-spot"),
            vec![
                "--source-color-index",
                "0",
                "color",
                "hex",
                "#336699",
                "--mode",
                "dark",
                "--type",
                "scheme-tonal-spot"
            ]
        );
    }

    #[test]
    fn generate_argument_order_and_force_dark() {
        let image = Target::Image("/tmp/a.png".into());
        let tuning = Tuning {
            harmony: Some("0.6".into()),
            harmonize_threshold: Some("100".into()),
            term_fg_boost: Some("0.35".into()),
        };
        let args = generate_args(
            &image,
            "light",
            "scheme-expressive",
            Path::new("/colors/scheme-base.json"),
            Path::new("/state/color.txt"),
            false,
            &tuning,
        );
        assert_eq!(
            args,
            vec![
                "--path",
                "/tmp/a.png",
                "--mode",
                "light",
                "--scheme",
                "scheme-expressive",
                "--termscheme",
                "/colors/scheme-base.json",
                "--blend_bg_fg",
                "--cache",
                "/state/color.txt",
                "--harmony",
                "0.6",
                "--harmonize_threshold",
                "100",
                "--term_fg_boost",
                "0.35"
            ]
        );
        let args = generate_args(
            &image,
            "light",
            "scheme-content",
            Path::new("/t"),
            Path::new("/c"),
            true,
            &Tuning::default(),
        );
        assert_eq!(args[3], "dark");
    }

    #[test]
    fn mode_variant_lookup_matches_bash_names() {
        let dir = std::env::temp_dir().join(format!("dots-theme-variant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pic.png"), b"x").unwrap();
        std::fs::write(dir.join("pic-dark.png"), b"x").unwrap();
        std::fs::write(dir.join("pic-light.txt"), b"x").unwrap();
        std::fs::write(dir.join("pic.txt"), b"x").unwrap();
        assert_eq!(
            with_mode_variant(&dir.join("pic.png").to_string_lossy(), "dark"),
            dir.join("pic-dark.png").to_string_lossy()
        );
        assert_eq!(
            with_mode_variant(&dir.join("pic-light.txt").to_string_lossy(), "dark"),
            dir.join("pic.txt").to_string_lossy()
        );
        assert_eq!(
            with_mode_variant("/nonexistent/photo", "light"),
            "/nonexistent/photo"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn restore_script_text_matches_the_heredoc() {
        assert!(EMPTY_RESTORE.ends_with("Don't modify it by yourself.\n"));
        let script = restore_video_script("/videos/clip.mp4", "Thu Nov 19 00:00:00 2026");
        assert!(script.contains("pkill -f -9 mpvpaper\n"));
        assert!(
            script.contains("for monitor in $(hyprctl monitors -j | jq -r '.[] | .name'); do\n")
        );
        assert!(script.contains(&format!(
            "mpvpaper -o \"{VIDEO_OPTS}\" \"$monitor\" \"/videos/clip.mp4\" &\n"
        )));
    }
}
