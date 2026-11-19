//! Colour maths ported line for line from `materialyoucolor` so the arithmetic
//! order, and therefore the rounded 8-bit result, matches the Python reference.

pub const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.41233895, 0.35762064, 0.18051042],
    [0.2126, 0.7152, 0.0722],
    [0.01932141, 0.11916382, 0.95034478],
];

pub const WHITE_POINT_D65: [f64; 3] = [95.047, 100.0, 108.883];

/// Python's `round()`: ties go to the nearest even integer, unlike `f64::round`.
pub fn py_round(x: f64) -> f64 {
    let floor = x.floor();
    let diff = x - floor;
    if diff < 0.5 {
        floor
    } else if diff > 0.5 {
        floor + 1.0
    } else if floor % 2.0 == 0.0 {
        floor
    } else {
        floor + 1.0
    }
}

fn py_int(x: f64) -> i64 {
    x.trunc() as i64
}

pub fn signum(x: f64) -> f64 {
    if x < 0.0 {
        -1.0
    } else if x == 0.0 {
        0.0
    } else {
        1.0
    }
}

pub fn lerp(start: f64, stop: f64, amount: f64) -> f64 {
    (1.0 - amount) * start + amount * stop
}

pub fn clamp_double(min: f64, max: f64, value: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub fn sanitize_degrees_double(degrees: f64) -> f64 {
    let mut degrees = degrees % 360.0;
    if degrees < 0.0 {
        degrees += 360.0;
    }
    degrees
}

pub fn rotation_direction(from_angle: f64, to_angle: f64) -> f64 {
    let increasing_difference = sanitize_degrees_double(to_angle - from_angle);
    if increasing_difference <= 180.0 {
        1.0
    } else {
        -1.0
    }
}

pub fn difference_degrees(a: f64, b: f64) -> f64 {
    180.0 - ((a - b).abs() - 180.0).abs()
}

pub fn matrix_multiply(row: [f64; 3], matrix: [[f64; 3]; 3]) -> [f64; 3] {
    [
        row[0] * matrix[0][0] + row[1] * matrix[0][1] + row[2] * matrix[0][2],
        row[0] * matrix[1][0] + row[1] * matrix[1][1] + row[2] * matrix[1][2],
        row[0] * matrix[2][0] + row[1] * matrix[2][1] + row[2] * matrix[2][2],
    ]
}

pub fn linearized(rgb_component: f64) -> f64 {
    let normalized = rgb_component / 255.0;
    if normalized <= 0.040449936 {
        normalized / 12.92 * 100.0
    } else {
        ((normalized + 0.055) / 1.055).powf(2.4) * 100.0
    }
}

/// `clamp_int + round` as in Python; the intermediate is still a float here.
pub fn delinearized(rgb_component: f64) -> f64 {
    let normalized = rgb_component / 100.0;
    let delinearized = if normalized <= 0.0031308 {
        normalized * 12.92
    } else {
        1.055 * normalized.powf(1.0 / 2.4) - 0.055
    };
    clamp_double(0.0, 255.0, py_round(delinearized * 255.0))
}

fn lab_f(t: f64) -> f64 {
    let e = 216.0 / 24389.0;
    let kappa = 24389.0 / 27.0;
    if t > e {
        t.powf(1.0 / 3.0)
    } else {
        (kappa * t + 16.0) / 116.0
    }
}

fn lab_invf(ft: f64) -> f64 {
    let e = 216.0 / 24389.0;
    let kappa = 24389.0 / 27.0;
    let ft3 = ft * ft * ft;
    if ft3 > e {
        ft3
    } else {
        (116.0 * ft - 16.0) / kappa
    }
}

pub fn y_from_lstar(lstar: f64) -> f64 {
    100.0 * lab_invf((lstar + 16.0) / 116.0)
}

pub fn lstar_from_y(y: f64) -> f64 {
    lab_f(y / 100.0) * 116.0 - 16.0
}

pub fn argb_from_rgb(red: f64, green: f64, blue: f64) -> u32 {
    let alpha = 255_i64;
    (alpha << 24 | (py_int(red) & 255) << 16 | (py_int(green) & 255) << 8 | py_int(blue) & 255)
        as u32
}

pub fn argb_from_linrgb(linrgb: [f64; 3]) -> u32 {
    argb_from_rgb(
        delinearized(linrgb[0]),
        delinearized(linrgb[1]),
        delinearized(linrgb[2]),
    )
}

pub fn xyz_from_argb(argb: u32) -> [f64; 3] {
    let r = linearized(((argb >> 16) & 255) as f64);
    let g = linearized(((argb >> 8) & 255) as f64);
    let b = linearized((argb & 255) as f64);
    matrix_multiply([r, g, b], SRGB_TO_XYZ)
}

pub fn argb_from_lstar(lstar: f64) -> u32 {
    let y = y_from_lstar(lstar);
    let component = delinearized(y);
    argb_from_rgb(component, component, component)
}

pub fn lstar_from_argb(argb: u32) -> f64 {
    let y = xyz_from_argb(argb)[1];
    116.0 * lab_f(y / 100.0) - 16.0
}

pub fn lab_from_argb(argb: u32) -> [f64; 3] {
    let r = linearized(((argb >> 16) & 255) as f64);
    let g = linearized(((argb >> 8) & 255) as f64);
    let b = linearized((argb & 255) as f64);
    let m = SRGB_TO_XYZ;
    let x = m[0][0] * r + m[0][1] * g + m[0][2] * b;
    let y = m[1][0] * r + m[1][1] * g + m[1][2] * b;
    let z = m[2][0] * r + m[2][1] * g + m[2][2] * b;
    let x_normalized = x / WHITE_POINT_D65[0];
    let y_normalized = y / WHITE_POINT_D65[1];
    let z_normalized = z / WHITE_POINT_D65[2];
    let fx = lab_f(x_normalized);
    let fy = lab_f(y_normalized);
    let fz = lab_f(z_normalized);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

pub fn rgba_from_argb(argb: u32) -> [u32; 4] {
    [
        (argb >> 16) & 255,
        (argb >> 8) & 255,
        argb & 255,
        (argb >> 24) & 255,
    ]
}

pub fn argb_hex(argb: u32) -> String {
    let [r, g, b, _] = rgba_from_argb(argb);
    format!("#{r:02X}{g:02X}{b:02X}")
}

pub fn display_color(argb: u32) -> String {
    let [r, g, b, _] = rgba_from_argb(argb);
    format!("\x1b[38;2;{r};{g};{b}m\x1b[7m   \x1b[7m\x1b[0m")
}

pub fn hex_to_argb(hex_code: &str) -> Result<u32, String> {
    let chars: Vec<char> = hex_code.chars().collect();
    let part = |start: usize, end: usize| -> Result<f64, String> {
        let text: String = chars
            .get(start..end.min(chars.len()))
            .unwrap_or_default()
            .iter()
            .collect();
        i64::from_str_radix(text.trim(), 16)
            .map(|value| value as f64)
            .map_err(|_| format!("invalid hex colour '{hex_code}'"))
    };
    Ok(argb_from_rgb(
        part(1, 3)?,
        part(3, 5)?,
        part(5, chars.len())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_round_matches_bankers_rounding() {
        let cases = [
            (0.5, 0.0),
            (1.5, 2.0),
            (2.5, 2.0),
            (3.5, 4.0),
            (-0.5, 0.0),
            (-1.5, -2.0),
            (-2.5, -2.0),
            (2.675, 3.0),
            (0.49999999999999994, 0.0),
            (1.4999999999999998, 1.0),
            (49.5, 50.0),
            (50.5, 50.0),
        ];
        for (input, expected) in cases {
            assert_eq!(py_round(input), expected, "round({input})");
        }
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex_to_argb("#336699").unwrap(), 0xFF336699);
        assert_eq!(argb_hex(0xFF336699), "#336699");
        assert!(hex_to_argb("#zzzzzz").is_err());
    }
}
