#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColorArg {
    Hex(String),
    Clear,
    Pick,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub mode: Option<String>,
    pub kind: Option<String>,
    pub color: Option<ColorArg>,
    pub image: Option<String>,
    pub noswitch: bool,
}

/// Mirrors the argument loop in switchwall.sh: there is no help flag, unknown
/// flags are positional image paths, and the first positional wins.
pub fn parse(args: &[String]) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "option --mode requires a value".to_string())?;
                cli.mode = Some(value.clone());
                i += 2;
            }
            "--type" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "option --type requires a value".to_string())?;
                cli.kind = Some(value.clone());
                i += 2;
            }
            "--color" => match args.get(i + 1) {
                Some(value) if is_hex(value) => {
                    cli.color = Some(ColorArg::Hex(value.clone()));
                    i += 2;
                }
                Some(value) if value == "clear" => {
                    cli.color = Some(ColorArg::Clear);
                    i += 2;
                }
                Some(_) => {
                    cli.color = Some(ColorArg::Pick);
                    i += 1;
                }
                None => {
                    cli.color = Some(ColorArg::Pick);
                    i += 1;
                }
            },
            "--image" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "option --image requires a value".to_string())?;
                cli.image = Some(value.clone());
                i += 2;
            }
            "--noswitch" => {
                cli.noswitch = true;
                i += 1;
            }
            value => {
                if cli.image.is_none() {
                    cli.image = Some(value.to_string());
                }
                i += 1;
            }
        }
    }
    Ok(cli)
}

pub fn is_hex(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = match bytes.first() {
        Some(b'#') => &bytes[1..],
        _ => bytes,
    };
    digits.len() == 6 && digits.iter().all(u8::is_ascii_hexdigit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn maps_every_switchwall_flag() {
        let cli = parse(&args(&[
            "--mode",
            "light",
            "--type",
            "scheme-expressive",
            "--color",
            "#AABBCC",
            "--image",
            "/tmp/pic.png",
        ]))
        .unwrap();
        assert_eq!(cli.mode.as_deref(), Some("light"));
        assert_eq!(cli.kind.as_deref(), Some("scheme-expressive"));
        assert_eq!(cli.color, Some(ColorArg::Hex("#AABBCC".into())));
        assert_eq!(cli.image.as_deref(), Some("/tmp/pic.png"));
        assert!(!cli.noswitch);
    }

    #[test]
    fn positional_image_and_noswitch() {
        let cli = parse(&args(&["/tmp/a.png", "--noswitch", "/tmp/b.png"])).unwrap();
        assert_eq!(cli.image.as_deref(), Some("/tmp/a.png"));
        assert!(cli.noswitch);
    }

    #[test]
    fn image_flag_overrides_an_earlier_positional() {
        let cli = parse(&args(&["/tmp/a.png", "--image", "/tmp/b.png"])).unwrap();
        assert_eq!(cli.image.as_deref(), Some("/tmp/b.png"));
    }

    #[test]
    fn color_forms_match_the_bash_regex() {
        assert_eq!(
            parse(&args(&["--color", "112233"])).unwrap().color,
            Some(ColorArg::Hex("112233".into()))
        );
        assert_eq!(
            parse(&args(&["--color", "#a1B2c3"])).unwrap().color,
            Some(ColorArg::Hex("#a1B2c3".into()))
        );
        assert_eq!(
            parse(&args(&["--color", "clear"])).unwrap().color,
            Some(ColorArg::Clear)
        );
        assert_eq!(
            parse(&args(&["--color"])).unwrap().color,
            Some(ColorArg::Pick)
        );
        assert_eq!(
            parse(&args(&["--color", "not-a-color"])).unwrap().color,
            Some(ColorArg::Pick)
        );
        assert!(!is_hex("#12345"));
        assert!(!is_hex("#1234567"));
        assert!(!is_hex("12345g"));
    }

    #[test]
    fn color_without_a_value_does_not_eat_the_next_flag() {
        let cli = parse(&args(&["--color", "--mode", "dark"])).unwrap();
        assert_eq!(cli.color, Some(ColorArg::Pick));
        assert_eq!(cli.mode.as_deref(), Some("dark"));
    }

    #[test]
    fn help_is_an_image_path_like_the_script() {
        let cli = parse(&args(&["--help"])).unwrap();
        assert_eq!(cli.image.as_deref(), Some("--help"));
        assert_eq!(parse(&args(&["-h"])).unwrap().image.as_deref(), Some("-h"));
    }

    #[test]
    fn missing_option_values_are_errors() {
        assert!(parse(&args(&["--mode"])).is_err());
        assert!(parse(&args(&["--type"])).is_err());
        assert!(parse(&args(&["--image"])).is_err());
    }
}
