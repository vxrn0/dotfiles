//! `scheme_for_image.py`: colourfulness of an image, and the scheme it picks.
//!
//! The Python script decodes with OpenCV and computes Hasler and Süsstrunk's
//! metric with NumPy.  OpenCV's decode and INTER_AREA resize are reproduced in
//! `decode.rs` and `area.rs`; this file reproduces NumPy's reduction order.

/// NumPy's `pairwise_sum_DOUBLE` from `loops_utils.h.src` (block size 128).
fn pairwise_sum(values: &[f64]) -> f64 {
    fn recurse(values: &[f64]) -> f64 {
        let n = values.len();
        if n < 8 {
            let mut result = -0.0;
            for &value in values {
                result += value;
            }
            return result;
        }
        if n <= 128 {
            let mut r = [
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                values[7],
            ];
            let mut i = 8;
            while i < n - (n % 8) {
                for j in 0..8 {
                    r[j] += values[i + j];
                }
                i += 8;
            }
            let mut result = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
            while i < n {
                result += values[i];
                i += 1;
            }
            return result;
        }
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        recurse(&values[..n2]) + recurse(&values[n2..])
    }
    recurse(values)
}

fn mean(values: &[f64]) -> f64 {
    pairwise_sum(values) / values.len() as f64
}

fn std(values: &[f64]) -> f64 {
    let average = mean(values);
    let squares: Vec<f64> = values
        .iter()
        .map(|value| (value - average) * (value - average))
        .collect();
    (pairwise_sum(&squares) / values.len() as f64).sqrt()
}

/// `image_colorfulness` from the Python script, over RGB bytes.
pub fn colorfulness(rgb: &[u8]) -> f64 {
    let mut rg = Vec::with_capacity(rgb.len() / 3);
    let mut yb = Vec::with_capacity(rgb.len() / 3);
    for pixel in rgb.chunks_exact(3) {
        let red = pixel[0] as f64;
        let green = pixel[1] as f64;
        let blue = pixel[2] as f64;
        rg.push((red - green).abs());
        yb.push((0.5 * (red + green) - blue).abs());
    }
    let std_rg = std(&rg);
    let std_yb = std(&yb);
    let mean_rg = mean(&rg);
    let mean_yb = mean(&yb);
    (std_rg * std_rg + std_yb * std_yb).sqrt()
        + 0.3 * (mean_rg * mean_rg + mean_yb * mean_yb).sqrt()
}

pub fn pick_scheme(value: f64) -> &'static str {
    if value < 40.0 {
        "scheme-neutral"
    } else {
        "scheme-tonal-spot"
    }
}

/// Python's `str(float)` for the values this metric produces.
pub fn python_float(value: f64) -> String {
    if value == 0.0 {
        return "0.0".to_string();
    }
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    let magnitude = value.abs();
    if (1e-4..1e16).contains(&magnitude) {
        let text = format!("{value:?}");
        if text.contains('.') || text.contains('e') {
            text
        } else {
            format!("{text}.0")
        }
    } else {
        // Outside the fixed-notation window Python prints an exponent.
        let text = format!("{value:e}");
        if let Some((mantissa, exponent)) = text.split_once('e') {
            let exponent: i32 = exponent.parse().unwrap_or(0);
            format!("{mantissa}e{exponent:+03}")
        } else {
            text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_matches_numpy_small_and_block_boundaries() {
        // Values chosen so a plain left-to-right sum differs.
        let values: Vec<f64> = (0..1000).map(|i| ((i * 37) % 101) as f64 + 0.1).collect();
        assert_eq!(pairwise_sum(&values), 50110.000000000015);
        assert_eq!(pairwise_sum(&values[..7]), 272.7);
        assert_eq!(pairwise_sum(&values[..128]), 6333.800000000001);
        assert_eq!(pairwise_sum(&values[..129]), 6423.9000000000015);
        assert_eq!(pairwise_sum(&[]), 0.0);
    }
}
