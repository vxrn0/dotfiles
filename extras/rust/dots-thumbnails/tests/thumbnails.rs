use dots_thumbnails::{md5, png as thumbpng, uri};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

fn tmp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_png(path: &Path, color: [u8; 4]) {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&color.repeat(4)).unwrap();
    }
    fs::write(path, out).unwrap();
}

struct Sandbox {
    root: PathBuf,
    home: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    data: PathBuf,
    run: PathBuf,
    bin: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root = tmp_dir(name);
        let home = root.join("home");
        let config = root.join("config");
        let cache = root.join("cache");
        let data = root.join("data");
        let run = root.join("run");
        let bin = root.join("bin");
        for dir in [&home, &config, &cache, &data, &run, &bin] {
            fs::create_dir_all(dir).unwrap();
        }
        Self {
            root,
            home,
            config,
            cache,
            data,
            run,
            bin,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dots-thumbnails"));
        command
            .env_clear()
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_CACHE_HOME", &self.cache)
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_RUNTIME_DIR", &self.run)
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .current_dir(&self.root);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn add_thumbnailer(&self, body: &str) {
        let thumbnailers = self.data.join("thumbnailers");
        fs::create_dir_all(&thumbnailers).unwrap();
        fs::write(
            thumbnailers.join("fake.thumbnailer"),
            format!(
                "[Thumbnailer Entry]\nTryExec={}\nExec={}\nMimeType=image/png;image/jpeg;\n",
                self.bin.join("fake-thumbnailer").display(),
                body
            ),
        )
        .unwrap();
        let script = self.bin.join("fake-thumbnailer");
        fs::write(
            &script,
            "#!/bin/bash\nprintf 'THUMB %s %s %s\\n' \"$2\" \"$4\" \"$6\"\nprintf 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==' | base64 -d > \"$4\"\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn add_magick(&self) {
        let script = self.bin.join("magick");
        fs::write(
            &script,
            "#!/bin/bash\nprintf 'MAGICK %s %s %s\\n' \"$1\" \"$2 $3\" \"$4\"\ncp \"$1\" \"$4\"\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn metadata_mtime(path: &Path) -> i64 {
    fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn cache_files(sandbox: &Sandbox) -> Vec<PathBuf> {
    let mut files = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().and_then(|value| value.to_str()) == Some("png")
                    && !path.to_string_lossy().contains("/thumbnails/fail/")
                {
                    out.push(path);
                }
            }
        }
    }
    walk(&sandbox.cache.join("thumbnails"), &mut files);
    walk(&sandbox.home.join(".cache/thumbnails"), &mut files);
    files.sort();
    files
}

#[test]
fn gnome_mode_writes_spec_thumbnails_and_honours_freshness() {
    let sandbox = Sandbox::new("gnome");
    let pictures = sandbox.home.join("Pictures");
    fs::create_dir_all(&pictures).unwrap();
    let first = pictures.join("one.png");
    write_png(&first, [255, 0, 0, 255]);
    let second = pictures.join("two.png");
    write_png(&second, [0, 255, 0, 255]);
    sandbox.add_thumbnailer("fake-thumbnailer --input %u --output %o --size %s --mime %m");

    let output = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("THUMB ").count(), 2, "{stdout}");
    assert!(stdout.contains("PROGRESS 2/2"));
    assert!(stdout.contains("Thumbnail Generation Completed!"));

    let files = cache_files(&sandbox);
    assert_eq!(files.len(), 2);
    for (source, size) in [(&first, 128), (&second, 128)] {
        let file_uri = uri::file_uri(source);
        let thumbnail = sandbox
            .cache
            .join("thumbnails/normal")
            .join(format!("{}.png", md5::hex(file_uri.as_bytes())));
        assert!(files.contains(&thumbnail));
        let bytes = fs::read(&thumbnail).unwrap();
        assert!(thumbpng::valid(&bytes, &file_uri, metadata_mtime(source)));
        assert_eq!(
            fs::metadata(&thumbnail).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _ = size;
    }

    let again = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert!(!String::from_utf8_lossy(&again.stdout).contains("THUMB "));

    let times = fs::FileTimes::new().set_modified(SystemTime::now() + Duration::from_secs(10));
    fs::OpenOptions::new()
        .write(true)
        .open(&first)
        .unwrap()
        .set_times(times)
        .unwrap();
    let touched = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert_eq!(
        String::from_utf8_lossy(&touched.stdout)
            .matches("THUMB ")
            .count(),
        1
    );
}

#[test]
fn failed_thumbnails_and_other_sizes_are_respected() {
    let sandbox = Sandbox::new("failed");
    let pictures = sandbox.home.join("Pictures");
    fs::create_dir_all(&pictures).unwrap();
    let source = pictures.join("one.png");
    write_png(&source, [0, 0, 255, 255]);
    sandbox.add_thumbnailer("fake-thumbnailer --input %i --output %o --size %s --mime %m");

    let mtime = metadata_mtime(&source);
    let file_uri = uri::file_uri(&source);
    let failed = sandbox
        .cache
        .join("thumbnails/fail/gnome-thumbnail-factory")
        .join(format!("{}.png", md5::hex(file_uri.as_bytes())));
    fs::create_dir_all(failed.parent().unwrap()).unwrap();
    let failure = thumbpng::with_metadata(
        &{
            let mut out = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut out, 1, 1);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().unwrap();
                writer.write_image_data(&[0, 0, 0, 255]).unwrap();
            }
            out
        },
        &file_uri,
        mtime,
    )
    .unwrap();
    fs::write(&failed, failure).unwrap();

    let skipped = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert!(!String::from_utf8_lossy(&skipped.stdout).contains("THUMB "));
    assert!(cache_files(&sandbox).is_empty());

    let times = fs::FileTimes::new().set_modified(SystemTime::now() + Duration::from_secs(20));
    fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_times(times)
        .unwrap();
    let large = sandbox.run(&[
        "--size",
        "large",
        "--machine_progress",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert!(large.status.success());
    let files = cache_files(&sandbox);
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("/thumbnails/large/"));
}

#[test]
fn only_images_recursive_and_bad_directory_follow_the_reference() {
    let sandbox = Sandbox::new("layout");
    let pictures = sandbox.home.join("Pictures");
    let nested = pictures.join("nested");
    fs::create_dir_all(&nested).unwrap();
    write_png(&pictures.join("top.png"), [1, 2, 3, 255]);
    write_png(&nested.join("deep.png"), [4, 5, 6, 255]);
    fs::write(pictures.join("notes.txt"), b"hello").unwrap();
    sandbox.add_thumbnailer("fake-thumbnailer --input %i --output %o --size %s");

    let top = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "--only_images",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&top.stdout);
    assert!(stdout.contains("Found 2 files"));
    assert!(stdout.contains("Found 1 images"));
    assert_eq!(stdout.matches("THUMB ").count(), 1);

    let _ = fs::remove_dir_all(sandbox.cache.join("thumbnails"));
    let recursive = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "--only_images",
        "--recursive",
        "-d",
        pictures.to_str().unwrap(),
    ]);
    assert_eq!(
        String::from_utf8_lossy(&recursive.stdout)
            .matches("THUMB ")
            .count(),
        2
    );

    let missing = sandbox.run(&[
        "--size",
        "normal",
        "--machine_progress",
        "-d",
        "/no/such/directory",
    ]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("isn't a valid directory"));
}

#[test]
fn magick_mode_matches_the_fallback_contract() {
    let sandbox = Sandbox::new("magick");
    sandbox.add_magick();
    let pictures = sandbox.home.join("Pictures");
    fs::create_dir_all(&pictures).unwrap();
    write_png(&pictures.join("one.png"), [9, 8, 7, 255]);
    write_png(&pictures.join("skip.gif"), [9, 8, 7, 255]);
    write_png(&pictures.join("skip.mp4"), [9, 8, 7, 255]);

    let directory = sandbox.run(&["--size", "normal", "-d", pictures.to_str().unwrap()]);
    assert!(directory.status.success());
    assert_eq!(
        String::from_utf8_lossy(&directory.stdout)
            .matches("MAGICK ")
            .count(),
        1
    );
    let files = cache_files(&sandbox);
    assert_eq!(files.len(), 1);

    let again = sandbox.run(&["--size", "normal", "-d", pictures.to_str().unwrap()]);
    assert!(!String::from_utf8_lossy(&again.stdout).contains("MAGICK "));

    let file = sandbox.run(&[
        "--size",
        "normal",
        "--file",
        pictures.join("one.png").to_str().unwrap(),
    ]);
    assert!(file.status.success());
    // The file mode uses $HOME/.cache, independent of XDG_CACHE_HOME.
    assert!(sandbox
        .home
        .join(".cache/thumbnails/normal")
        .read_dir()
        .unwrap()
        .next()
        .is_some());

    let missing = sandbox.run(&["--file", "/no/such.png"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stdout).contains("File not found: /no/such.png"));
    let usage = sandbox.run(&["--file"]);
    assert_eq!(usage.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&usage.stdout).contains("Usage:"));
}
