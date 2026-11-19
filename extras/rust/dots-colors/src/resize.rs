//! Pillow's `Image.resize(..., Image.Resampling.BICUBIC)` for 8-bit channels.
//!
//! Ported from Pillow 13.0.0 `src/libImaging/Resample.c`: the coefficients are
//! precomputed per output pixel, converted to 22-bit fixed point, then a
//! separable horizontal/vertical convolution runs with the same operation
//! order.  The caller decides how many channels to carry.

const PRECISION_BITS: i32 = 32 - 8 - 2;

fn bicubic_filter(x: f64) -> f64 {
    let a = -0.5f64;
    let x = x.abs();
    if x < 1.0 {
        ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
    } else if x < 2.0 {
        (((x - 5.0) * x + 8.0) * x - 4.0) * a
    } else {
        0.0
    }
}

struct Coeffs {
    ksize: usize,
    bounds: Vec<(usize, usize)>,
    fixed: Vec<i32>,
}

fn precompute_coeffs(in_size: usize, out_size: usize) -> Coeffs {
    let scale = in_size as f64 / out_size as f64;
    let filterscale = if scale < 1.0 { 1.0 } else { scale };
    let support = 2.0 * filterscale;
    let ksize = support.ceil() as usize * 2 + 1;
    let mut bounds = Vec::with_capacity(out_size);
    let mut fixed = vec![0i32; out_size * ksize];
    let mut weights = vec![0f64; ksize];
    let inv_filterscale = 1.0 / filterscale;

    for xx in 0..out_size {
        let center = (xx as f64 + 0.5) * scale;
        let mut xmin = (center - support + 0.5) as i64;
        if xmin < 0 {
            xmin = 0;
        }
        let mut xmax = (center + support + 0.5) as i64;
        if xmax > in_size as i64 {
            xmax = in_size as i64;
        }
        let xmax = (xmax - xmin) as usize;
        let mut ww = 0.0;
        for x in 0..xmax {
            let w = bicubic_filter((x as f64 + xmin as f64 - center + 0.5) * inv_filterscale);
            weights[x] = w;
            ww += w;
        }
        let row = &mut fixed[xx * ksize..(xx + 1) * ksize];
        if ww != 0.0 {
            for x in 0..xmax {
                let w = weights[x] / ww;
                row[x] = if w < 0.0 {
                    (-0.5 + w * (1 << PRECISION_BITS) as f64) as i32
                } else {
                    (0.5 + w * (1 << PRECISION_BITS) as f64) as i32
                };
            }
        }
        bounds.push((xmin as usize, xmax));
    }

    Coeffs {
        ksize,
        bounds,
        fixed,
    }
}

#[inline]
fn clip8(value: i32) -> u8 {
    let value = value >> PRECISION_BITS;
    value.clamp(0, 255) as u8
}

fn horizontal(
    input: &[u8],
    in_w: usize,
    in_h: usize,
    out_w: usize,
    channels: usize,
    coeffs: &Coeffs,
) -> Vec<u8> {
    let mut out = vec![0u8; out_w * in_h * channels];
    for y in 0..in_h {
        for x in 0..out_w {
            let (xmin, xmax) = coeffs.bounds[x];
            let k = &coeffs.fixed[x * coeffs.ksize..(x + 1) * coeffs.ksize];
            for c in 0..channels {
                let mut ss = 1 << (PRECISION_BITS - 1);
                for i in 0..xmax {
                    ss += input[((y * in_w) + xmin + i) * channels + c] as i32 * k[i];
                }
                out[(y * out_w + x) * channels + c] = clip8(ss);
            }
        }
    }
    out
}

fn vertical(input: &[u8], in_w: usize, out_h: usize, channels: usize, coeffs: &Coeffs) -> Vec<u8> {
    let mut out = vec![0u8; in_w * out_h * channels];
    for y in 0..out_h {
        let (ymin, ymax) = coeffs.bounds[y];
        let k = &coeffs.fixed[y * coeffs.ksize..(y + 1) * coeffs.ksize];
        for x in 0..in_w {
            for c in 0..channels {
                let mut ss = 1 << (PRECISION_BITS - 1);
                for i in 0..ymax {
                    ss += input[((ymin + i) * in_w + x) * channels + c] as i32 * k[i];
                }
                out[(y * in_w + x) * channels + c] = clip8(ss);
            }
        }
    }
    out
}

/// Resize an RGB buffer exactly as Pillow's bicubic resampler does.
pub fn bicubic(
    input: &[u8],
    in_w: usize,
    in_h: usize,
    out_w: usize,
    out_h: usize,
    channels: usize,
) -> Vec<u8> {
    if in_w == out_w && in_h == out_h {
        return input.to_vec();
    }
    let horizontal_first = !((in_h > out_h) && (in_h - out_h) > (in_w - out_w) * 2);

    let vert = if out_h != in_h {
        Some(precompute_coeffs(in_h, out_h))
    } else {
        None
    };
    let horiz = if out_w != in_w {
        Some(precompute_coeffs(in_w, out_w))
    } else {
        None
    };

    let mut work = input.to_vec();
    let mut work_w = in_w;
    let mut work_h = in_h;
    if horizontal_first {
        if let Some(h) = &horiz {
            work = horizontal(&work, work_w, work_h, out_w, channels, h);
            work_w = out_w;
        }
        if let Some(v) = &vert {
            work = vertical(&work, work_w, out_h, channels, v);
            work_h = out_h;
        }
    } else {
        if let Some(v) = &vert {
            work = vertical(&work, work_w, out_h, channels, v);
            work_h = out_h;
        }
        if let Some(h) = &horiz {
            work = horizontal(&work, work_w, work_h, out_w, channels, h);
            work_w = out_w;
        }
    }
    debug_assert_eq!((work_w, work_h), (out_w, out_h));
    debug_assert_eq!(work.len(), out_w * out_h * channels);
    work
}
