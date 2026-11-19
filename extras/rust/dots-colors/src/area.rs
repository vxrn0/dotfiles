//! `cv2.resize(..., INTER_AREA)` for 8-bit BGR/RGB, downscaling only.
//!
//! Ported from OpenCV 4.x `modules/imgproc/src/resize.cpp`.  The fast path
//! averages integer blocks; the general path builds fractional area tables and
//! accumulates in float32 exactly like OpenCV's `resizeArea_` for 8-bit input.

fn area_tab(source: usize, dest: usize, channels: usize) -> Vec<Vec<(usize, f32)>> {
    // cv::resize stores inv_scale = dest/source and hal::resize converts it
    // back with 1/inv_scale, so the round trip must be reproduced.
    let inv_scale = dest as f64 / source as f64;
    let scale = 1.0 / inv_scale;
    let mut out = Vec::with_capacity(dest);
    for d in 0..dest {
        let mut row = Vec::new();
        let fsx1 = d as f64 * scale;
        let fsx2 = fsx1 + scale;
        let cell_width = scale.min(source as f64 - fsx1);
        let mut sx1 = fsx1.ceil() as i64;
        let mut sx2 = fsx2.floor() as i64;
        sx2 = sx2.min(source as i64 - 1);
        sx1 = sx1.min(sx2);

        if sx1 as f64 - fsx1 > 1e-3 {
            row.push((
                (sx1 - 1) as usize * channels,
                ((sx1 as f64 - fsx1) / cell_width) as f32,
            ));
        }
        for sx in sx1..sx2 {
            row.push((sx as usize * channels, (1.0 / cell_width) as f32));
        }
        if fsx2 - sx2 as f64 > 1e-3 {
            row.push((
                sx2 as usize * channels,
                ((fsx2 - sx2 as f64).min(1.0).min(cell_width) / cell_width) as f32,
            ));
        }
        out.push(row);
    }
    out
}

fn round_even(value: f32) -> u8 {
    value.round_ties_even().clamp(0.0, 255.0) as u8
}

pub fn area_resize(
    rgb: &[u8],
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<u8> {
    const CHANNELS: usize = 3;

    if width == out_width && height == out_height {
        return rgb.to_vec();
    }

    let scale_x = 1.0 / (out_width as f64 / width as f64);
    let scale_y = 1.0 / (out_height as f64 / height as f64);
    let iscale_x = scale_x.round_ties_even() as i64;
    let iscale_y = scale_y.round_ties_even() as i64;
    let fast = (scale_x - iscale_x as f64).abs() < f64::EPSILON
        && (scale_y - iscale_y as f64).abs() < f64::EPSILON
        && iscale_x >= 1
        && iscale_y >= 1;

    let mut out = vec![0u8; out_width * out_height * CHANNELS];

    if fast {
        let sx = iscale_x as usize;
        let sy = iscale_y as usize;
        let area = sx * sy;
        let scale = 1.0f32 / area as f32;
        let full_width = (width / sx) * CHANNELS;
        for dy in 0..out_height {
            let sy0 = dy * sy;
            let row_start = sy0 * width * CHANNELS;
            let w = if sy0 + sy <= height { full_width } else { 0 };
            for dx in 0..out_width * CHANNELS {
                let dst = (dy * out_width * CHANNELS) + dx;
                if sy0 >= height {
                    out[dst] = 0;
                    continue;
                }
                if dx < w {
                    let sx0 = (dx / CHANNELS) * sx;
                    let offset = row_start + sx0 * CHANNELS + (dx % CHANNELS);
                    let mut sum = 0i32;
                    for y in 0..sy {
                        for x in 0..sx {
                            sum += rgb[offset + (y * width + x) * CHANNELS] as i32;
                        }
                    }
                    out[dst] = if area == 4 {
                        ((sum + 2) >> 2) as u8
                    } else {
                        round_even(sum as f32 * scale)
                    };
                } else {
                    let mut sum = 0i32;
                    let mut count = 0i32;
                    let sx0 = (dx / CHANNELS) * sx;
                    for y in 0..sy {
                        if sy0 + y >= height {
                            break;
                        }
                        for x in 0..sx {
                            if sx0 + x >= width {
                                break;
                            }
                            sum += rgb[((sy0 + y) * width + sx0 + x) * CHANNELS + (dx % CHANNELS)]
                                as i32;
                            count += 1;
                        }
                    }
                    out[dst] = round_even(sum as f32 / count as f32);
                }
            }
        }
        return out;
    }

    let xtab = area_tab(width, out_width, CHANNELS);
    let ytab = area_tab(height, out_height, 1);
    let mut sum = vec![0f32; out_width * CHANNELS];
    let mut buf = vec![0f32; out_width * CHANNELS];

    for dy in 0..out_height {
        for value in sum.iter_mut() {
            *value = 0.0;
        }
        for &(sy, beta) in &ytab[dy] {
            for value in buf.iter_mut() {
                *value = 0.0;
            }
            let row = &rgb[sy * width * CHANNELS..(sy + 1) * width * CHANNELS];
            for (dx, entries) in xtab.iter().enumerate() {
                for &(sx, alpha) in entries {
                    for c in 0..CHANNELS {
                        buf[dx * CHANNELS + c] += row[sx + c] as f32 * alpha;
                    }
                }
            }
            for i in 0..sum.len() {
                sum[i] += beta * buf[i];
            }
        }
        for i in 0..sum.len() {
            out[dy * out_width * CHANNELS + i] = round_even(sum[i]);
        }
    }
    out
}
