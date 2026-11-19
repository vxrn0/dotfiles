mod area;
mod cam16;
mod color;
mod contrast;
mod decode;
mod dynamic;
mod hct;
mod palette;
mod quantize;
mod resize;
mod scheme;
mod scheme_for_image;
mod temperature;
mod terminal;

use color::{argb_hex, display_color, hex_to_argb, py_round};
use dynamic::ROLES;
use hct::Hct;
use scheme::DynamicScheme;
use std::env;
use std::fs;
use std::process::exit;

struct Args {
    path: Option<String>,
    color: Option<String>,
    size: i64,
    mode: Mode,
    scheme: String,
    transparency: Transparency,
    termscheme: Option<String>,
    harmony: f64,
    harmonize_threshold: f64,
    term_fg_boost: f64,
    blend_bg_fg: bool,
    cache: Option<String>,
    smart: bool,
    debug: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Dark,
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Transparency {
    Opaque,
    Transparent,
}

enum Action {
    Run(Args),
    Help,
}

fn parse_args(argv: Vec<String>) -> Result<Action, String> {
    let mut args = Args {
        path: None,
        color: None,
        size: 128,
        mode: Mode::Dark,
        scheme: "vibrant".to_string(),
        transparency: Transparency::Opaque,
        termscheme: None,
        harmony: 0.8,
        harmonize_threshold: 100.0,
        term_fg_boost: 0.35,
        blend_bg_fg: false,
        cache: None,
        smart: false,
        debug: false,
    };

    let mut index = 0;
    while index < argv.len() {
        let raw = &argv[index];
        index += 1;

        if raw == "-h" || raw == "--help" {
            return Ok(Action::Help);
        }

        let (name, inline_value) = match raw.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (raw.as_str(), None),
        };

        let mut take_value = |what: &str| -> Result<String, String> {
            if let Some(value) = inline_value.clone() {
                return Ok(value);
            }
            if index < argv.len() {
                let value = argv[index].clone();
                index += 1;
                Ok(value)
            } else {
                Err(format!("argument {what}: expected one argument"))
            }
        };

        match name {
            "--path" => args.path = Some(take_value("--path")?),
            "--color" => args.color = Some(take_value("--color")?),
            "--scheme" => args.scheme = take_value("--scheme")?,
            "--termscheme" => args.termscheme = Some(take_value("--termscheme")?),
            "--cache" => args.cache = Some(take_value("--cache")?),
            "--size" => {
                args.size = take_value("--size")?
                    .parse::<i64>()
                    .map_err(|_| "argument --size: invalid int value".to_string())?;
            }
            "--mode" => {
                args.mode = match take_value("--mode")?.as_str() {
                    "dark" => Mode::Dark,
                    "light" => Mode::Light,
                    other => {
                        return Err(format!(
                            "argument --mode: invalid choice: '{other}' (choose from 'dark', 'light')"
                        ))
                    }
                }
            }
            "--transparency" => {
                args.transparency = match take_value("--transparency")?.as_str() {
                    "opaque" => Transparency::Opaque,
                    "transparent" => Transparency::Transparent,
                    other => {
                        return Err(format!(
                            "argument --transparency: invalid choice: '{other}' (choose from 'opaque', 'transparent')"
                        ))
                    }
                }
            }
            "--harmony" => {
                args.harmony = take_value("--harmony")?
                    .parse()
                    .map_err(|_| "argument --harmony: invalid float value".to_string())?;
            }
            "--harmonize_threshold" => {
                args.harmonize_threshold = take_value("--harmonize_threshold")?
                    .parse()
                    .map_err(|_| {
                        "argument --harmonize_threshold: invalid float value".to_string()
                    })?;
            }
            "--term_fg_boost" => {
                args.term_fg_boost = take_value("--term_fg_boost")?
                    .parse()
                    .map_err(|_| "argument --term_fg_boost: invalid float value".to_string())?;
            }
            "--smart" => {
                if inline_value.is_some() {
                    return Err("argument --smart: ignored explicit argument".to_string());
                }
                args.smart = true;
            }
            "--blend_bg_fg" => {
                if inline_value.is_some() {
                    return Err("argument --blend_bg_fg: ignored explicit argument".to_string());
                }
                args.blend_bg_fg = true;
            }
            "--debug" => {
                if inline_value.is_some() {
                    return Err("argument --debug: ignored explicit argument".to_string());
                }
                args.debug = true;
            }
            other => return Err(format!("unrecognized argument '{other}'")),
        }
    }

    Ok(Action::Run(args))
}

fn print_help() {
    println!("usage: dots-colors [-h] [--path PATH] [--size SIZE] [--color COLOR]");
    println!("                   [--mode {{dark,light}}] [--scheme SCHEME] [--smart]");
    println!(
        "                   [--transparency {{opaque,transparent}}] [--termscheme TERMSCHEME]"
    );
    println!("                   [--harmony HARMONY] [--harmonize_threshold HARMONIZE_THRESHOLD]");
    println!("                   [--term_fg_boost TERM_FG_BOOST] [--blend_bg_fg] [--cache CACHE]");
    println!("                   [--debug]");
    println!();
    println!("Color generation script");
    println!();
    println!("options:");
    println!("  -h, --help            show this help message and exit");
    println!("  --path PATH           generate colorscheme from image");
    println!("  --size SIZE           bitmap image size");
    println!("  --color COLOR         generate colorscheme from color");
    println!("  --mode {{dark,light}}  dark or light mode");
    println!("  --scheme SCHEME       material scheme to use");
    println!("  --smart               decide scheme type based on image color");
    println!("  --transparency {{opaque,transparent}}");
    println!("                        enable transparency");
    println!("  --termscheme TERMSCHEME");
    println!(
        "                        JSON file containg the terminal scheme for generating term colors"
    );
    println!("  --harmony HARMONY     (0-1) Color hue shift towards accent");
    println!("  --harmonize_threshold HARMONIZE_THRESHOLD");
    println!("                        (0-180) Max threshold angle to limit color hue shift");
    println!("  --term_fg_boost TERM_FG_BOOST");
    println!("                        Make terminal foreground more different from the background");
    println!("  --blend_bg_fg         Shift terminal background or foreground towards accent");
    println!("  --cache CACHE         file path to store the generated color");
    println!("  --debug               debug mode");
}

enum TermValue {
    Raw(String),
    Argb(u32),
}

struct TermColor {
    name: String,
    source: String,
    value: TermValue,
}

fn load_term_scheme(path: &str, dark: bool) -> Result<Vec<(String, String)>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let root: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("invalid JSON in {path}: {error}"))?;
    let section = root
        .get(if dark { "dark" } else { "light" })
        .ok_or_else(|| {
            format!(
                "{path}: missing '{}' section",
                if dark { "dark" } else { "light" }
            )
        })?;
    let object = section
        .as_object()
        .ok_or_else(|| format!("{path}: terminal section is not an object"))?;

    let mut entries = Vec::new();
    for (name, value) in object {
        let value = value
            .as_str()
            .ok_or_else(|| format!("{path}: terminal colour '{name}' is not a string"))?;
        entries.push((name.clone(), value.to_string()));
    }
    Ok(entries)
}

fn calculate_optimal_size(width: usize, height: usize, bitmap_size: i64) -> (usize, usize) {
    let image_area = width as i128 * height as i128;
    let bitmap_area = bitmap_size as i128 * bitmap_size as i128;
    let scale = if image_area > bitmap_area {
        (bitmap_area as f64 / image_area as f64).sqrt()
    } else {
        1.0
    };
    let new_width = py_round(width as f64 * scale) as i64;
    let new_height = py_round(height as f64 * scale) as i64;
    (new_width.max(1) as usize, new_height.max(1) as usize)
}

fn image_argb(
    path: &str,
    size: i64,
    smart: bool,
    scheme: &mut String,
) -> Result<(u32, Hct, (usize, usize, usize, usize)), String> {
    let image = decode::decode(path)?;
    let (width, height) = (image.width, image.height);
    let (new_width, new_height) = calculate_optimal_size(width, height, size);
    let rgb = if new_width < width || new_height < height {
        decode::resize(
            &image.rgb,
            image.alpha.as_deref(),
            width,
            height,
            new_width,
            new_height,
        )
    } else {
        image.rgb.clone()
    };

    let pixels: Vec<u32> = rgb
        .chunks_exact(3)
        .map(|pixel| {
            0xFF000000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | pixel[2] as u32
        })
        .collect();
    let colors = quantize::quantize_celebi(&pixels, 128);
    let scored = quantize::score(&colors);
    let argb = *scored.first().unwrap_or(&0xFF4285F4);
    let hct = Hct::from_int(argb);
    if smart && hct.chroma < 20.0 {
        *scheme = "neutral".to_string();
    }
    Ok((argb, hct, (width, height, new_width, new_height)))
}

fn run(mut args: Args) -> Result<(), String> {
    let dark = args.mode == Mode::Dark;
    let transparent = args.transparency == Transparency::Transparent;

    let (argb, hct, image_size) = if let Some(path) = &args.path {
        let (argb, hct, sizes) = image_argb(path, args.size, args.smart, &mut args.scheme)?;
        if let Some(cache) = &args.cache {
            fs::write(cache, argb_hex(argb))
                .map_err(|error| format!("cannot write {cache}: {error}"))?;
        }
        (argb, hct, Some(sizes))
    } else {
        let color_text = args
            .color
            .as_deref()
            .ok_or_else(|| "one of --color or --path is required".to_string())?;
        let argb = hex_to_argb(color_text)?;
        (argb, Hct::from_int(argb), None)
    };
    let scheme = DynamicScheme::for_scheme(&args.scheme, hct, dark, 0.0)?;

    let material: Vec<(&str, u32)> = ROLES
        .iter()
        .map(|(name, role)| (*name, dynamic::get_argb(*role, &scheme)))
        .collect();

    let mut terms: Vec<TermColor> = Vec::new();
    if let Some(path) = &args.termscheme {
        let source_colors = load_term_scheme(path, dark)?;
        let primary_argb = material
            .iter()
            .find(|(name, _)| *name == "primary_paletteKeyColor")
            .map(|(_, argb)| *argb)
            .expect("primary key colour is always generated");

        for (name, source) in source_colors {
            let value = if args.scheme == "monochrome" {
                TermValue::Raw(source.clone())
            } else if args.blend_bg_fg && name == "term0" {
                let surface = material
                    .iter()
                    .find(|(role, _)| *role == "surfaceContainerLow")
                    .map(|(_, argb)| *argb)
                    .unwrap();
                TermValue::Argb(terminal::boost_chroma_tone(surface, 1.2, 0.95))
            } else if args.blend_bg_fg && name == "term15" {
                let on_surface = material
                    .iter()
                    .find(|(role, _)| *role == "onSurface")
                    .map(|(_, argb)| *argb)
                    .unwrap();
                TermValue::Argb(terminal::boost_chroma_tone(on_surface, 3.0, 1.0))
            } else {
                let harmonized = terminal::harmonize(
                    hex_to_argb(&source)?,
                    primary_argb,
                    args.harmonize_threshold,
                    args.harmony,
                );
                let tone = 1.0 + args.term_fg_boost * if dark { 1.0 } else { -1.0 };
                TermValue::Argb(terminal::boost_chroma_tone(harmonized, 1.0, tone))
            };
            terms.push(TermColor {
                name,
                source,
                value,
            });
        }
    }

    if !args.debug {
        println!("$darkmode: {};", if dark { "True" } else { "False" });
        println!(
            "$transparent: {};",
            if transparent { "True" } else { "False" }
        );
        for (name, argb) in &material {
            println!("${name}: {};", argb_hex(*argb));
        }
        for (name, code) in dynamic::success_colors(dark) {
            println!("${name}: {code};");
        }
        for term in &terms {
            println!("${}: {};", term.name, term_hex(&term.value));
        }
        return Ok(());
    }

    println!();
    if let Some((width, height, new_width, new_height)) = image_size {
        println!("--------------Image properties-----------------");
        println!("Image size: {width} x {height}");
        println!("Resized image: {new_width} x {new_height}");
        println!();
    }
    println!("---------------Selected color------------------");
    println!("Dark mode: {}", if dark { "True" } else { "False" });
    println!("Scheme: {}", args.scheme);
    println!("Accent color: {} {}", display_color(argb), argb_hex(argb));
    println!("HCT: {:.2}  {:.2}  {:.2}", hct.hue, hct.chroma, hct.tone);
    println!();
    println!("---------------Material colors-----------------");
    for (name, argb) in &material {
        println!(
            "{:<32} : {}  {}",
            name,
            display_color(*argb),
            argb_hex(*argb)
        );
    }
    for (name, code) in dynamic::success_colors(dark) {
        let argb = hex_to_argb(code)?;
        println!("{:<32} : {}  {}", name, display_color(argb), code);
    }
    println!();
    println!("----------Harmonize terminal colors------------");
    for term in &terms {
        let argb = match &term.value {
            TermValue::Raw(value) => hex_to_argb(value)?,
            TermValue::Argb(argb) => *argb,
        };
        let source_argb = hex_to_argb(&term.source)?;
        println!(
            "{:<6} : {} {} --> {} {}",
            term.name,
            display_color(source_argb),
            term.source,
            display_color(argb),
            argb_hex(argb)
        );
    }
    println!("-----------------------------------------------");

    Ok(())
}

fn term_hex(value: &TermValue) -> String {
    match value {
        TermValue::Raw(raw) => raw.clone(),
        TermValue::Argb(argb) => argb_hex(*argb),
    }
}

fn scheme_for_image_main(argv: Vec<String>) -> ! {
    let mut colorfulness_mode = false;
    let mut rest = Vec::new();
    let mut removed = false;
    for arg in argv {
        if !colorfulness_mode && !removed && arg == "--colorfulness" {
            colorfulness_mode = true;
            removed = true;
        } else {
            rest.push(arg);
        }
    }

    let Some(path) = rest.first() else {
        println!("scheme-tonal-spot");
        exit(1);
    };
    let decoded = match decode::decode_first(path) {
        Ok(image) => image,
        Err(_) => {
            println!("scheme-tonal-spot");
            exit(1);
        }
    };

    let (width, height) = (decoded.width, decoded.height);
    let rgb = if width.max(height) > 128 {
        let scale = 128.0 / width.max(height) as f64;
        let new_width = (width as f64 * scale) as usize;
        let new_height = (height as f64 * scale) as usize;
        if new_width == 0 || new_height == 0 {
            eprintln!("dots-colors: cannot resize {width}x{height} for colorfulness");
            exit(1);
        }
        area::area_resize(&decoded.rgb, width, height, new_width, new_height)
    } else {
        decoded.rgb
    };

    let value = scheme_for_image::colorfulness(&rgb);
    if colorfulness_mode {
        println!("{}", scheme_for_image::python_float(value));
    } else {
        println!("{}", scheme_for_image::pick_scheme(value));
    }
    exit(0);
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("scheme-for-image") {
        scheme_for_image_main(argv[1..].to_vec());
    }
    match parse_args(argv) {
        Ok(Action::Help) => print_help(),
        Ok(Action::Run(args)) => {
            if let Err(error) = run(args) {
                eprintln!("dots-colors: {error}");
                exit(1);
            }
        }
        Err(error) => {
            eprintln!("dots-colors: {error}");
            exit(2);
        }
    }
}
