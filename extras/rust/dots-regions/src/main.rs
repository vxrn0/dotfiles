mod regions;

use image::RgbImage;
use regions::{find_regions, Params, Rect};
use serde_json::{json, Value};
use std::env;
use std::process::exit;

const DEFAULT_IMAGE_PATH: &str = "/tmp/quickshell/media/screenshot/image";

struct Args {
    image: String,
    debug_output: Option<String>,
    min_width: i64,
    min_height: i64,
    max_width: Option<i64>,
    max_height: Option<i64>,
    hyprctl: bool,
    single: bool,
}

fn usage(message: String) -> ! {
    eprintln!("dots-regions: {message}");
    eprintln!(
        "usage: dots-regions [--image PATH] [--min-width N] [--min-height N] \
         [--max-width N] [--max-height N] [--hyprctl] [--single] [--debug-output PATH]"
    );
    exit(2)
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    args.next()
        .unwrap_or_else(|| usage(format!("{flag} needs a value")))
}

fn number(args: &mut impl Iterator<Item = String>, flag: &str) -> i64 {
    let text = value(args, flag);
    text.parse()
        .unwrap_or_else(|_| usage(format!("{flag} needs an integer, got '{text}'")))
}

fn parse_args() -> Args {
    let mut parsed = Args {
        image: DEFAULT_IMAGE_PATH.to_string(),
        debug_output: None,
        min_width: 200,
        min_height: 100,
        max_width: None,
        max_height: None,
        hyprctl: false,
        single: false,
    };
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "-i" | "--image" => parsed.image = value(&mut args, &flag),
            "-do" | "--debug-output" => parsed.debug_output = Some(value(&mut args, &flag)),
            "--min-width" => parsed.min_width = number(&mut args, &flag),
            "--min-height" => parsed.min_height = number(&mut args, &flag),
            "--max-width" => parsed.max_width = Some(number(&mut args, &flag)),
            "--max-height" => parsed.max_height = Some(number(&mut args, &flag)),
            "--hyprctl" => parsed.hyprctl = true,
            "--single" => parsed.single = true,
            other => usage(format!("unknown argument '{other}'")),
        }
    }
    parsed
}

fn load(path: &str) -> Option<RgbImage> {
    image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()
        .map(|image| image.to_rgb8())
}

fn draw_regions(image: &mut RgbImage, regions: &[Rect]) {
    let (width, height) = (image.width() as i64, image.height() as i64);
    let red = image::Rgb([255, 0, 0]);
    for rect in regions {
        let x0 = rect.x.clamp(0, width);
        let y0 = rect.y.clamp(0, height);
        let x1 = (rect.x + rect.width).clamp(x0, width);
        let y1 = (rect.y + rect.height).clamp(y0, height);
        for offset in 0..2 {
            let top = y0 + offset;
            let bottom = y1 - 1 - offset;
            for x in x0..x1 {
                if top < height {
                    image.put_pixel(x as u32, top as u32, red);
                }
                if bottom >= 0 {
                    image.put_pixel(x as u32, bottom as u32, red);
                }
            }
            let left = x0 + offset;
            let right = x1 - 1 - offset;
            for y in y0..y1 {
                if left < width {
                    image.put_pixel(left as u32, y as u32, red);
                }
                if right >= 0 {
                    image.put_pixel(right as u32, y as u32, red);
                }
            }
        }
    }
}

fn main() {
    let args = parse_args();
    let Some(mut image) = load(&args.image) else {
        eprintln!("Error: Could not load image {}", args.image);
        exit(1);
    };

    let mut found = find_regions(
        &image,
        Params {
            min_width: args.min_width,
            min_height: args.min_height,
            max_width: args.max_width,
            max_height: args.max_height,
        },
    );
    if args.single && !found.is_empty() {
        // NMS returns the boxes largest first.
        found.truncate(1);
    }

    let output: Vec<Value> = if args.hyprctl {
        found
            .iter()
            .map(|rect| {
                json!({
                    "at": [rect.x, rect.y],
                    "size": [rect.width, rect.height],
                })
            })
            .collect()
    } else {
        found
            .iter()
            .map(|rect| {
                json!({
                    "x": rect.x,
                    "y": rect.y,
                    "width": rect.width,
                    "height": rect.height,
                })
            })
            .collect()
    };
    println!("{}", serde_json::to_string(&output).expect("JSON output"));

    if let Some(path) = args.debug_output {
        draw_regions(&mut image, &found);
        if let Err(error) = image.save(&path) {
            eprintln!("dots-regions: could not write debug image {path}: {error}");
        }
    }
}
