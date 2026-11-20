use image::{Rgb, RgbImage};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn tmp_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("regions");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn save(name: &str, image: &RgbImage) -> PathBuf {
    let path = tmp_dir().join(name);
    image.save(&path).unwrap();
    path
}

/// Every command in this file gets a private HOME and private XDG directories.
fn sandbox() -> Command {
    let home = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sandbox-home");
    for sub in ["config", "cache", "state", "data", "runtime", "tmp"] {
        fs::create_dir_all(home.join(sub)).unwrap();
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_dots-regions"));
    command
        .env_clear()
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_RUNTIME_DIR", home.join("runtime"))
        .env("TMPDIR", home.join("tmp"))
        .current_dir(&home);
    command
}

fn run(args: &[&str]) -> Output {
    sandbox().args(args).output().unwrap()
}

fn run_on(path: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--image", path.to_str().unwrap()];
    args.extend_from_slice(extra);
    run(&args)
}

fn rects(output: &Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "exit {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is a JSON array")
}

fn field(value: &Value, name: &str) -> i64 {
    value[name]
        .as_i64()
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn iou(value: &Value, expected: (i64, i64, i64, i64)) -> f64 {
    let (x, y, width, height) = (
        field(value, "x"),
        field(value, "y"),
        field(value, "width"),
        field(value, "height"),
    );
    let (ex, ey, ew, eh) = expected;
    let left = x.max(ex);
    let top = y.max(ey);
    let right = (x + width).min(ex + ew);
    let bottom = (y + height).min(ey + eh);
    let intersection = (right - left).max(0) as f64 * (bottom - top).max(0) as f64;
    let union = (width * height + ew * eh) as f64 - intersection;
    intersection / union
}

fn canvas(width: u32, height: u32, color: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(width, height, Rgb(color))
}

/// Inclusive left/top, exclusive right/bottom.
fn fill(image: &mut RgbImage, x0: u32, y0: u32, x1: u32, y1: u32, color: [u8; 3]) {
    for y in y0..y1 {
        for x in x0..x1 {
            image.put_pixel(x, y, Rgb(color));
        }
    }
}

fn outline(image: &mut RgbImage, x0: u32, y0: u32, x1: u32, y1: u32, color: [u8; 3]) {
    for x in x0..x1 {
        image.put_pixel(x, y0, Rgb(color));
        image.put_pixel(x, y1 - 1, Rgb(color));
    }
    for y in y0..y1 {
        image.put_pixel(x0, y, Rgb(color));
        image.put_pixel(x1 - 1, y, Rgb(color));
    }
}

#[test]
fn synthetic_rectangles_are_found() {
    let mut checks = 0;

    // One high-contrast rectangle.
    let mut one = canvas(640, 480, [10, 10, 10]);
    fill(&mut one, 100, 90, 400, 290, [250, 250, 250]);
    let one = save("one-rect.png", &one);

    // Two rectangles of different colours and sizes.
    let mut two = canvas(640, 480, [20, 20, 20]);
    fill(&mut two, 60, 60, 240, 180, [240, 240, 240]);
    fill(&mut two, 360, 250, 580, 410, [30, 120, 230]);
    let two = save("two-rects.png", &two);

    // A rectangle with a textured interior.
    let mut textured = canvas(640, 480, [40, 40, 40]);
    for y in (100..300).step_by(10) {
        for x in (120..440).step_by(10) {
            if (x / 10 + y / 10) % 2 == 0 {
                fill(&mut textured, x, y, x + 10, y + 10, [220, 220, 220]);
            }
        }
    }
    outline(&mut textured, 120, 100, 440, 300, [255, 60, 60]);
    let textured = save("textured-rect.png", &textured);

    // A photo-like gradient with a hard-edged panel.
    let mut photo = canvas(640, 480, [0, 0, 0]);
    for y in 0..480 {
        for x in 0..640 {
            photo.put_pixel(
                x,
                y,
                Rgb([
                    (30 + x * 180 / 640) as u8,
                    (60 + y * 120 / 480) as u8,
                    (90 + (x + y) * 100 / 1120) as u8,
                ]),
            );
        }
    }
    fill(&mut photo, 90, 70, 330, 260, [245, 200, 20]);
    outline(&mut photo, 90, 70, 330, 260, [0, 0, 0]);
    let photo = save("gradient-panel.png", &photo);

    let cases = [
        (one, vec![(100, 90, 300, 200)]),
        (two, vec![(60, 60, 180, 120), (360, 250, 220, 160)]),
        (textured, vec![(120, 100, 320, 200)]),
        (photo, vec![(90, 70, 240, 190)]),
    ];

    for (path, expected) in cases {
        let output = run_on(&path, &["--min-width", "20", "--min-height", "20"]);
        let found = rects(&output);
        assert!(!found.is_empty(), "{} found nothing", path.display());
        for target in expected {
            let best = found
                .iter()
                .map(|rect| iou(rect, target))
                .fold(0.0, f64::max);
            assert!(
                best >= 0.7,
                "{}: expected {:?}, best IoU {best:.3} in {found:?}",
                path.display(),
                target
            );
            checks += 1;
        }
    }

    println!("synthetic: {checks} expected rectangles matched at IoU >= 0.7");
}

#[test]
fn blank_images_yield_an_empty_array() {
    for (name, color) in [
        ("blank-black.png", [0, 0, 0]),
        ("blank-white.png", [255, 255, 255]),
    ] {
        let path = save(name, &canvas(320, 240, color));
        let output = run_on(&path, &["--min-width", "20", "--min-height", "20"]);
        assert!(
            rects(&output).is_empty(),
            "{name} produced regions: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn flag_contract_matches_the_python() {
    let mut image = canvas(640, 480, [10, 10, 10]);
    fill(&mut image, 100, 100, 400, 300, [250, 250, 250]);
    let path = save("flags.png", &image);
    let path = path.to_str().unwrap();

    // Min filters are strict (>), so values above the real size remove the box.
    let present = rects(&run(&[
        "--image",
        path,
        "--min-width",
        "20",
        "--min-height",
        "20",
    ]));
    assert!(present
        .iter()
        .any(|rect| iou(rect, (100, 100, 300, 200)) >= 0.7));
    for extra in [
        ["--min-width", "400"],
        ["--min-height", "250"],
        ["--max-width", "200"],
        ["--max-height", "150"],
    ] {
        let filtered = rects(&run(&[
            "--image",
            path,
            "--min-width",
            "20",
            "--min-height",
            "20",
            extra[0],
            extra[1],
        ]));
        assert!(filtered.is_empty(), "{extra:?} left {filtered:?}");
    }

    // --hyprctl replaces x/y/width/height with at/size.
    let hyprctl = rects(&run(&["--image", path, "--hyprctl"]));
    assert!(!hyprctl.is_empty());
    for rect in &hyprctl {
        assert_eq!(rect["at"].as_array().unwrap().len(), 2);
        assert_eq!(rect["size"].as_array().unwrap().len(), 2);
        assert!(rect.get("x").is_none());
    }

    // --debug-output draws but does not change stdout.
    let debug = tmp_dir().join("flags-debug.png");
    let output = run(&[
        "--image",
        path,
        "--min-width",
        "20",
        "--min-height",
        "20",
        "--debug-output",
        debug.to_str().unwrap(),
    ]);
    let plain = rects(&run(&[
        "--image",
        path,
        "--min-width",
        "20",
        "--min-height",
        "20",
    ]));
    assert_eq!(rects(&output), plain);
    let overlay = image::open(&debug).unwrap().to_rgb8();
    assert_eq!(overlay.dimensions(), (640, 480));
    let red = overlay
        .pixels()
        .filter(|pixel| pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60)
        .count();
    assert!(red > 500, "overlay has only {red} red pixels");

    // The one-output flag Python calls --single.
    let largest = rects(&run(&[
        "--image",
        path,
        "--min-width",
        "20",
        "--min-height",
        "20",
        "--single",
    ]));
    assert_eq!(largest.len(), 1);
}

#[test]
fn a_missing_or_unreadable_image_exits_one() {
    let missing = run(&["--image", "/nonexistent/dots-regions-screenshot.png"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("Could not load image"), "stderr: {stderr}");

    let broken = tmp_dir().join("broken.png");
    fs::write(&broken, b"not a png").unwrap();
    let output = run_on(&broken, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn unknown_arguments_exit_two() {
    let output = run(&["--not-a-flag"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

fn real_dir() -> PathBuf {
    std::env::var_os("DOTS_REGIONS_IMAGES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/aiden/extra/from"))
}

#[test]
fn real_images_stay_inside_the_contract() {
    let dir = real_dir();
    if !dir.is_dir() {
        eprintln!(
            "real-image fixtures not found at {}; set DOTS_REGIONS_IMAGES to run this test",
            dir.display()
        );
        return;
    }

    let names = [
        "Meta_AI_wanderer_wallpaper_1920x1080_20261116_220907.jpg",
        "francesco-ungaro-RdELgb1bNz8-unsplash.jpg",
        "sidecar_cutout.png",
    ];
    let mut checked = 0;
    for name in names {
        let path = dir.join(name);
        assert!(
            path.is_file(),
            "missing fixture {} (unset DOTS_REGIONS_IMAGES to skip this test)",
            path.display()
        );
        let (image_width, image_height) = image::image_dimensions(&path).unwrap();
        let output = run_on(&path, &["--max-width", "960", "--max-height", "540"]);
        let found = rects(&output);
        assert!(found.len() < 40, "{name}: {} regions", found.len());
        assert!(!found.is_empty(), "{name}: no regions");
        let mut seen = HashSet::new();
        for rect in &found {
            let (x, y, width, height) = (
                field(rect, "x"),
                field(rect, "y"),
                field(rect, "width"),
                field(rect, "height"),
            );
            assert!(
                x >= 0
                    && y >= 0
                    && width > 0
                    && height > 0
                    && x + width <= image_width as i64
                    && y + height <= image_height as i64,
                "{name}: {rect} outside {image_width}x{image_height}"
            );
            assert!(width < 960 && height < 540, "{name}: {rect} ignores max");
            assert!(
                seen.insert((x, y, width, height)),
                "{name}: duplicate {rect}"
            );
        }

        if name.starts_with("Meta") {
            let debug = tmp_dir().join("meta-debug.png");
            let with_debug = run_on(
                &path,
                &[
                    "--max-width",
                    "960",
                    "--max-height",
                    "540",
                    "--debug-output",
                    debug.to_str().unwrap(),
                ],
            );
            assert_eq!(rects(&with_debug), found);
            assert_eq!(
                image::image_dimensions(&debug).unwrap(),
                (image_width, image_height)
            );
            println!("debug overlay: {}", debug.display());
        }
        checked += 1;
    }
    assert_eq!(checked, 3);
    println!("real fixtures: 3 images, {checked} checked");
}

#[test]
#[ignore = "timing gate; run in release: cargo test --release -- --ignored --nocapture"]
fn runtime_1080p_png_is_under_300ms() {
    let path = tmp_dir().join("screenshot-1080p.png");
    if !path.exists() {
        let mut image = canvas(1920, 1080, [18, 18, 22]);
        for y in 0..1080 {
            for x in 0..1920 {
                let value = (20 + (x * 30 / 1920) + (y * 20 / 1080)) as u8;
                image.put_pixel(x, y, Rgb([value, value, value + 4]));
            }
        }
        fill(&mut image, 80, 60, 900, 700, [238, 238, 238]);
        outline(&mut image, 80, 60, 900, 700, [10, 10, 10]);
        fill(&mut image, 1000, 90, 1850, 520, [40, 90, 180]);
        for line in 0..18 {
            fill(
                &mut image,
                1020,
                120 + line * 20,
                1800,
                128 + line * 20,
                [235, 235, 235],
            );
        }
        image.save(&path).unwrap();
    }

    let mut best = Duration::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let output = run_on(&path, &["--max-width", "960", "--max-height", "540"]);
        let elapsed = start.elapsed();
        assert!(output.status.success());
        best = best.min(elapsed);
    }
    println!("1080p PNG screenshot: {} ms", best.as_millis());
    assert!(best < Duration::from_millis(300), "{best:?} is over 300 ms");
}
