//! Port of the quantiser used by `materialyoucolor` 2.0.9.
//!
//! `QuantizeCelebi` in that package is a pybind11 wrapper around the C++
//! implementation in `material-color-utilities`: Wu's histogram quantiser
//! produces the starting clusters, then weighted squared-error means refines
//! them.  This file is a line-for-line port of `wu.cc` and `wsmeans.cc`, with
//! `rand()` taken from libc so the cluster seeding is identical.

use crate::color::{difference_degrees, py_round};
use crate::hct::Hct;
use std::collections::{BTreeMap, HashMap};

const INDEX_BITS: usize = 5;
const INDEX_COUNT: usize = (1 << INDEX_BITS) + 1;
const TOTAL_SIZE: usize = INDEX_COUNT * INDEX_COUNT * INDEX_COUNT;
const MAX_COLORS: usize = 256;

fn get_index(r: i32, g: i32, b: i32) -> usize {
    ((r << (INDEX_BITS * 2)) + (r << (INDEX_BITS + 1)) + (g << INDEX_BITS) + r + g + b) as usize
}

#[derive(Clone, Copy, Default)]
struct Box3 {
    r0: i32,
    r1: i32,
    g0: i32,
    g1: i32,
    b0: i32,
    b1: i32,
    vol: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Red,
    Green,
    Blue,
}

fn top(cube: &Box3, direction: Direction, position: i32, moment: &[i64]) -> i64 {
    match direction {
        Direction::Red => {
            moment[get_index(position, cube.g1, cube.b1)]
                - moment[get_index(position, cube.g1, cube.b0)]
                - moment[get_index(position, cube.g0, cube.b1)]
                + moment[get_index(position, cube.g0, cube.b0)]
        }
        Direction::Green => {
            moment[get_index(cube.r1, position, cube.b1)]
                - moment[get_index(cube.r1, position, cube.b0)]
                - moment[get_index(cube.r0, position, cube.b1)]
                + moment[get_index(cube.r0, position, cube.b0)]
        }
        Direction::Blue => {
            moment[get_index(cube.r1, cube.g1, position)]
                - moment[get_index(cube.r1, cube.g0, position)]
                - moment[get_index(cube.r0, cube.g1, position)]
                + moment[get_index(cube.r0, cube.g0, position)]
        }
    }
}

fn bottom(cube: &Box3, direction: Direction, moment: &[i64]) -> i64 {
    match direction {
        Direction::Red => {
            -moment[get_index(cube.r0, cube.g1, cube.b1)]
                + moment[get_index(cube.r0, cube.g1, cube.b0)]
                + moment[get_index(cube.r0, cube.g0, cube.b1)]
                - moment[get_index(cube.r0, cube.g0, cube.b0)]
        }
        Direction::Green => {
            -moment[get_index(cube.r1, cube.g0, cube.b1)]
                + moment[get_index(cube.r1, cube.g0, cube.b0)]
                + moment[get_index(cube.r0, cube.g0, cube.b1)]
                - moment[get_index(cube.r0, cube.g0, cube.b0)]
        }
        Direction::Blue => {
            -moment[get_index(cube.r1, cube.g1, cube.b0)]
                + moment[get_index(cube.r1, cube.g0, cube.b0)]
                + moment[get_index(cube.r0, cube.g1, cube.b0)]
                - moment[get_index(cube.r0, cube.g0, cube.b0)]
        }
    }
}

fn vol(cube: &Box3, moment: &[i64]) -> i64 {
    moment[get_index(cube.r1, cube.g1, cube.b1)]
        - moment[get_index(cube.r1, cube.g1, cube.b0)]
        - moment[get_index(cube.r1, cube.g0, cube.b1)]
        + moment[get_index(cube.r1, cube.g0, cube.b0)]
        - moment[get_index(cube.r0, cube.g1, cube.b1)]
        + moment[get_index(cube.r0, cube.g1, cube.b0)]
        + moment[get_index(cube.r0, cube.g0, cube.b1)]
        - moment[get_index(cube.r0, cube.g0, cube.b0)]
}

fn variance(
    cube: &Box3,
    weights: &[i64],
    m_r: &[i64],
    m_g: &[i64],
    m_b: &[i64],
    moments: &[f64],
) -> f64 {
    let dr = vol(cube, m_r) as f64;
    let dg = vol(cube, m_g) as f64;
    let db = vol(cube, m_b) as f64;
    let xx = moments[get_index(cube.r1, cube.g1, cube.b1)]
        - moments[get_index(cube.r1, cube.g1, cube.b0)]
        - moments[get_index(cube.r1, cube.g0, cube.b1)]
        + moments[get_index(cube.r1, cube.g0, cube.b0)]
        - moments[get_index(cube.r0, cube.g1, cube.b1)]
        + moments[get_index(cube.r0, cube.g1, cube.b0)]
        + moments[get_index(cube.r0, cube.g0, cube.b1)]
        - moments[get_index(cube.r0, cube.g0, cube.b0)];
    let hypotenuse = dr * dr + dg * dg + db * db;
    let volume = vol(cube, weights) as f64;
    xx - hypotenuse / volume
}

#[allow(clippy::too_many_arguments)]
fn maximize(
    cube: &Box3,
    direction: Direction,
    first: i32,
    last: i32,
    cut: &mut i32,
    whole_w: i64,
    whole_r: i64,
    whole_g: i64,
    whole_b: i64,
    weights: &[i64],
    m_r: &[i64],
    m_g: &[i64],
    m_b: &[i64],
) -> f64 {
    let bottom_r = bottom(cube, direction, m_r);
    let bottom_g = bottom(cube, direction, m_g);
    let bottom_b = bottom(cube, direction, m_b);
    let bottom_w = bottom(cube, direction, weights);

    let mut max = 0.0;
    *cut = -1;

    for i in first..last {
        let mut half_r = bottom_r + top(cube, direction, i, m_r);
        let mut half_g = bottom_g + top(cube, direction, i, m_g);
        let mut half_b = bottom_b + top(cube, direction, i, m_b);
        let mut half_w = bottom_w + top(cube, direction, i, weights);
        if half_w == 0 {
            continue;
        }

        let mut temp = (half_r as f64 * half_r as f64
            + half_g as f64 * half_g as f64
            + half_b as f64 * half_b as f64)
            / half_w as f64;

        half_r = whole_r - half_r;
        half_g = whole_g - half_g;
        half_b = whole_b - half_b;
        half_w = whole_w - half_w;
        if half_w == 0 {
            continue;
        }
        temp += (half_r as f64 * half_r as f64
            + half_g as f64 * half_g as f64
            + half_b as f64 * half_b as f64)
            / half_w as f64;

        if temp > max {
            max = temp;
            *cut = i;
        }
    }
    max
}

fn cut(
    box1: &mut Box3,
    box2: &mut Box3,
    weights: &[i64],
    m_r: &[i64],
    m_g: &[i64],
    m_b: &[i64],
) -> bool {
    let whole_r = vol(box1, m_r);
    let whole_g = vol(box1, m_g);
    let whole_b = vol(box1, m_b);
    let whole_w = vol(box1, weights);

    let mut cut_r = 0;
    let mut cut_g = 0;
    let mut cut_b = 0;
    let max_r = maximize(
        box1,
        Direction::Red,
        box1.r0 + 1,
        box1.r1,
        &mut cut_r,
        whole_w,
        whole_r,
        whole_g,
        whole_b,
        weights,
        m_r,
        m_g,
        m_b,
    );
    let max_g = maximize(
        box1,
        Direction::Green,
        box1.g0 + 1,
        box1.g1,
        &mut cut_g,
        whole_w,
        whole_r,
        whole_g,
        whole_b,
        weights,
        m_r,
        m_g,
        m_b,
    );
    let max_b = maximize(
        box1,
        Direction::Blue,
        box1.b0 + 1,
        box1.b1,
        &mut cut_b,
        whole_w,
        whole_r,
        whole_g,
        whole_b,
        weights,
        m_r,
        m_g,
        m_b,
    );

    let direction;
    if max_r >= max_g && max_r >= max_b {
        direction = Direction::Red;
        if cut_r < 0 {
            return false;
        }
    } else if max_g >= max_r && max_g >= max_b {
        direction = Direction::Green;
    } else {
        direction = Direction::Blue;
    }

    box2.r1 = box1.r1;
    box2.g1 = box1.g1;
    box2.b1 = box1.b1;

    match direction {
        Direction::Red => {
            box2.r0 = cut_r;
            box1.r1 = cut_r;
            box2.g0 = box1.g0;
            box2.b0 = box1.b0;
        }
        Direction::Green => {
            box2.r0 = box1.r0;
            box2.g0 = cut_g;
            box1.g1 = cut_g;
            box2.b0 = box1.b0;
        }
        Direction::Blue => {
            box2.r0 = box1.r0;
            box2.g0 = box1.g0;
            box2.b0 = cut_b;
            box1.b1 = cut_b;
        }
    }

    box1.vol = (box1.r1 - box1.r0) as i64 * (box1.g1 - box1.g0) as i64 * (box1.b1 - box1.b0) as i64;
    box2.vol = (box2.r1 - box2.r0) as i64 * (box2.g1 - box2.g0) as i64 * (box2.b1 - box2.b0) as i64;
    true
}

fn quantize_wu(pixels: &[u32], max_colors: usize) -> Vec<u32> {
    if max_colors == 0 || max_colors > MAX_COLORS || pixels.is_empty() {
        return Vec::new();
    }

    let mut weights = vec![0i64; TOTAL_SIZE];
    let mut m_r = vec![0i64; TOTAL_SIZE];
    let mut m_g = vec![0i64; TOTAL_SIZE];
    let mut m_b = vec![0i64; TOTAL_SIZE];
    let mut moments = vec![0f64; TOTAL_SIZE];

    for pixel in pixels {
        let red = ((pixel >> 16) & 0xff) as i32;
        let green = ((pixel >> 8) & 0xff) as i32;
        let blue = (pixel & 0xff) as i32;
        let bits_to_remove = 8 - INDEX_BITS as i32;
        let index_r = (red >> bits_to_remove) + 1;
        let index_g = (green >> bits_to_remove) + 1;
        let index_b = (blue >> bits_to_remove) + 1;
        let index = get_index(index_r, index_g, index_b);
        weights[index] += 1;
        m_r[index] += red as i64;
        m_g[index] += green as i64;
        m_b[index] += blue as i64;
        moments[index] += (red * red + green * green + blue * blue) as f64;
    }

    for r in 1..INDEX_COUNT {
        let mut area = [0i64; INDEX_COUNT];
        let mut area_r = [0i64; INDEX_COUNT];
        let mut area_g = [0i64; INDEX_COUNT];
        let mut area_b = [0i64; INDEX_COUNT];
        let mut area_2 = [0f64; INDEX_COUNT];
        for g in 1..INDEX_COUNT {
            let mut line = 0i64;
            let mut line_r = 0i64;
            let mut line_g = 0i64;
            let mut line_b = 0i64;
            let mut line_2 = 0f64;
            for b in 1..INDEX_COUNT {
                let index = get_index(r as i32, g as i32, b as i32);
                line += weights[index];
                line_r += m_r[index];
                line_g += m_g[index];
                line_b += m_b[index];
                line_2 += moments[index];

                area[b] += line;
                area_r[b] += line_r;
                area_g[b] += line_g;
                area_b[b] += line_b;
                area_2[b] += line_2;

                let previous_index = get_index(r as i32 - 1, g as i32, b as i32);
                weights[index] = weights[previous_index] + area[b];
                m_r[index] = m_r[previous_index] + area_r[b];
                m_g[index] = m_g[previous_index] + area_g[b];
                m_b[index] = m_b[previous_index] + area_b[b];
                moments[index] = moments[previous_index] + area_2[b];
            }
        }
    }

    let mut cubes = vec![Box3::default(); MAX_COLORS];
    cubes[0].r0 = 0;
    cubes[0].g0 = 0;
    cubes[0].b0 = 0;
    cubes[0].r1 = INDEX_COUNT as i32 - 1;
    cubes[0].g1 = INDEX_COUNT as i32 - 1;
    cubes[0].b1 = INDEX_COUNT as i32 - 1;

    let mut volume_variance = vec![0f64; MAX_COLORS];
    let mut next = 0usize;
    let mut limit = max_colors;
    let mut i = 1usize;
    while i < limit {
        let (left, right) = cubes.split_at_mut(i);
        if cut(&mut left[next], &mut right[0], &weights, &m_r, &m_g, &m_b) {
            volume_variance[next] = if cubes[next].vol > 1 {
                variance(&cubes[next], &weights, &m_r, &m_g, &m_b, &moments)
            } else {
                0.0
            };
            volume_variance[i] = if cubes[i].vol > 1 {
                variance(&cubes[i], &weights, &m_r, &m_g, &m_b, &moments)
            } else {
                0.0
            };
        } else {
            volume_variance[next] = 0.0;
            i -= 1;
        }

        next = 0;
        let mut temp = volume_variance[0];
        for j in 1..=i {
            if volume_variance[j] > temp {
                temp = volume_variance[j];
                next = j;
            }
        }
        if temp <= 0.0 {
            limit = i + 1;
            break;
        }
        i += 1;
    }

    let mut out = Vec::new();
    for cube in cubes.iter().take(limit) {
        let weight = vol(cube, &weights);
        if weight > 0 {
            let red = vol(cube, &m_r) / weight;
            let green = vol(cube, &m_g) / weight;
            let blue = vol(cube, &m_b) / weight;
            out.push(
                0xFF000000
                    | ((red as u32 & 0xff) << 16)
                    | ((green as u32 & 0xff) << 8)
                    | (blue as u32 & 0xff),
            );
        }
    }
    out
}

fn linearized(component: i32) -> f64 {
    let normalized = component as f64 / 255.0;
    if normalized <= 0.040449936 {
        normalized / 12.92 * 100.0
    } else {
        ((normalized + 0.055) / 1.055).powf(2.4) * 100.0
    }
}

fn delinearized(component: f64) -> i32 {
    let normalized = component / 100.0;
    let value = if normalized <= 0.0031308 {
        normalized * 12.92
    } else {
        1.055 * normalized.powf(1.0 / 2.4) - 0.055
    };
    (value * 255.0).round().clamp(0.0, 255.0) as i32
}

fn lab_from_int(argb: u32) -> [f64; 3] {
    let red = ((argb & 0x00ff0000) >> 16) as i32;
    let green = ((argb & 0x0000ff00) >> 8) as i32;
    let blue = (argb & 0x000000ff) as i32;
    let red_l = linearized(red);
    let green_l = linearized(green);
    let blue_l = linearized(blue);
    let x = 0.41233895 * red_l + 0.35762064 * green_l + 0.18051042 * blue_l;
    let y = 0.2126 * red_l + 0.7152 * green_l + 0.0722 * blue_l;
    let z = 0.01932141 * red_l + 0.11916382 * green_l + 0.95034478 * blue_l;
    let y_normalized = y / 100.0;
    let e = 216.0 / 24389.0;
    let kappa = 24389.0 / 27.0;
    let fy = if y_normalized > e {
        y_normalized.powf(1.0 / 3.0)
    } else {
        (kappa * y_normalized + 16.0) / 116.0
    };
    let x_normalized = x / 95.047;
    let fx = if x_normalized > e {
        x_normalized.powf(1.0 / 3.0)
    } else {
        (kappa * x_normalized + 16.0) / 116.0
    };
    let z_normalized = z / 108.883;
    let fz = if z_normalized > e {
        z_normalized.powf(1.0 / 3.0)
    } else {
        (kappa * z_normalized + 16.0) / 116.0
    };
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn int_from_lab(lab: [f64; 3]) -> u32 {
    let e = 216.0 / 24389.0;
    let kappa = 24389.0 / 27.0;
    let ke = 8.0;

    let fy = (lab[0] + 16.0) / 116.0;
    let fx = (lab[1] / 500.0) + fy;
    let fz = fy - (lab[2] / 200.0);
    let fx3 = fx * fx * fx;
    let x_normalized = if fx3 > e {
        fx3
    } else {
        (116.0 * fx - 16.0) / kappa
    };
    let y_normalized = if lab[0] > ke {
        fy * fy * fy
    } else {
        lab[0] / kappa
    };
    let fz3 = fz * fz * fz;
    let z_normalized = if fz3 > e {
        fz3
    } else {
        (116.0 * fz - 16.0) / kappa
    };
    let x = x_normalized * 95.047;
    let y = y_normalized * 100.0;
    let z = z_normalized * 108.883;

    let r_l = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g_l = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b_l = 0.0557 * x - 0.2040 * y + 1.0570 * z;

    let red = delinearized(r_l);
    let green = delinearized(g_l);
    let blue = delinearized(b_l);
    0xFF000000 | ((red as u32 & 0xff) << 16) | ((green as u32 & 0xff) << 8) | (blue as u32 & 0xff)
}

fn delta_e(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d_l = a[0] - b[0];
    let d_a = a[1] - b[1];
    let d_b = a[2] - b[2];
    d_l * d_l + d_a * d_a + d_b * d_b
}

fn quantize_wsmeans(
    input_pixels: &[u32],
    starting_clusters: &[u32],
    max_colors: usize,
) -> BTreeMap<u32, u32> {
    let mut result = BTreeMap::new();
    if max_colors == 0 || input_pixels.is_empty() {
        return result;
    }
    let max_colors = max_colors.min(MAX_COLORS);

    let mut pixel_to_count: HashMap<u32, i32> = HashMap::new();
    let mut pixels = Vec::with_capacity(input_pixels.len());
    let mut points = Vec::with_capacity(input_pixels.len());
    for &pixel in input_pixels {
        match pixel_to_count.get_mut(&pixel) {
            Some(count) => *count += 1,
            None => {
                pixels.push(pixel);
                points.push(lab_from_int(pixel));
                pixel_to_count.insert(pixel, 1);
            }
        }
    }

    let mut cluster_count = max_colors.min(points.len());
    if !starting_clusters.is_empty() {
        cluster_count = cluster_count.min(starting_clusters.len());
    }
    if cluster_count == 0 {
        return result;
    }

    let mut pixel_count_sums = [0i32; MAX_COLORS];
    let mut clusters: Vec<[f64; 3]> = starting_clusters
        .iter()
        .map(|&argb| lab_from_int(argb))
        .collect();

    let mut cluster_indices: Vec<i32> = Vec::with_capacity(points.len());
    unsafe {
        libc::srand(42688);
        let additional_clusters_needed = cluster_count as i64 - clusters.len() as i64;
        if starting_clusters.is_empty() && additional_clusters_needed > 0 {
            for _ in 0..additional_clusters_needed {
                let l = libc::rand() as f64 / 2147483647.0 * 100.0;
                let a = libc::rand() as f64 / 2147483647.0 * 200.0 - 100.0;
                let b = libc::rand() as f64 / 2147483647.0 * 200.0 - 100.0;
                clusters.push([l, a, b]);
            }
        }

        libc::srand(42688);
        for _ in 0..points.len() {
            cluster_indices.push(libc::rand() as i32 % cluster_count as i32);
        }
    }

    let mut distance_to_index_matrix = vec![vec![0f64; cluster_count]; cluster_count];

    for iteration in 0..100 {
        for i in 0..cluster_count {
            distance_to_index_matrix[i][i] = 0.0;
            for j in i + 1..cluster_count {
                let distance = delta_e(clusters[i], clusters[j]);
                distance_to_index_matrix[j][i] = distance;
                distance_to_index_matrix[i][j] = distance;
            }
        }

        let mut color_moved = false;
        for i in 0..points.len() {
            let point = points[i];
            let previous_cluster_index = cluster_indices[i] as usize;
            let previous_cluster = clusters[previous_cluster_index];
            let previous_distance = delta_e(point, previous_cluster);
            let mut minimum_distance = previous_distance;
            let mut new_cluster_index: i32 = -1;

            for j in 0..cluster_count {
                if distance_to_index_matrix[previous_cluster_index][j] >= 4.0 * previous_distance {
                    continue;
                }
                let distance = delta_e(point, clusters[j]);
                if distance < minimum_distance {
                    minimum_distance = distance;
                    new_cluster_index = j as i32;
                }
            }
            if new_cluster_index != -1 {
                // The C++ reference calls integer `abs()` on a double here, so
                // the difference is truncated before the comparison.  Keep the
                // quirk: it decides whether a cluster actually moves.
                let distance_change = (minimum_distance.sqrt() - previous_distance.sqrt())
                    .trunc()
                    .abs();
                if distance_change > 3.0 {
                    color_moved = true;
                    cluster_indices[i] = new_cluster_index;
                }
            }
        }

        if !color_moved && iteration != 0 {
            break;
        }

        let mut component_a_sums = [0f64; MAX_COLORS];
        let mut component_b_sums = [0f64; MAX_COLORS];
        let mut component_c_sums = [0f64; MAX_COLORS];
        for sum in pixel_count_sums.iter_mut() {
            *sum = 0;
        }

        for i in 0..points.len() {
            let cluster_index = cluster_indices[i] as usize;
            let point = points[i];
            let count = pixel_to_count[&pixels[i]];
            pixel_count_sums[cluster_index] += count;
            component_a_sums[cluster_index] += point[0] * count as f64;
            component_b_sums[cluster_index] += point[1] * count as f64;
            component_c_sums[cluster_index] += point[2] * count as f64;
        }

        for i in 0..cluster_count {
            let count = pixel_count_sums[i];
            if count == 0 {
                clusters[i] = [0.0, 0.0, 0.0];
                continue;
            }
            clusters[i] = [
                component_a_sums[i] / count as f64,
                component_b_sums[i] / count as f64,
                component_c_sums[i] / count as f64,
            ];
        }
    }

    for i in 0..cluster_count {
        let count = pixel_count_sums[i];
        if count == 0 {
            continue;
        }
        let argb = int_from_lab(clusters[i]);
        *result.entry(argb).or_insert(0) += count as u32;
    }
    result
}

/// `QuantizeCelebi(pixels, 128)` from the Python package.
pub fn quantize_celebi(pixels: &[u32], max_colors: usize) -> Vec<(u32, u32)> {
    let max_colors = max_colors.min(MAX_COLORS);
    let wu_result = quantize_wu(pixels, max_colors);
    quantize_wsmeans(pixels, &wu_result, max_colors)
        .into_iter()
        .collect()
}

/// `Score.score(colors)` from `materialyoucolor/score/score.py`.
pub fn score(colors_to_population: &[(u32, u32)]) -> Vec<u32> {
    const TARGET_CHROMA: f64 = 48.0;
    const WEIGHT_PROPORTION: f64 = 0.7;
    const WEIGHT_CHROMA_ABOVE: f64 = 0.3;
    const WEIGHT_CHROMA_BELOW: f64 = 0.1;
    const CUTOFF_CHROMA: f64 = 5.0;
    const CUTOFF_EXCITED_PROPORTION: f64 = 0.01;
    const DESIRED: usize = 4;
    const FALLBACK: u32 = 0xFF4285F4;

    let mut hue_population = [0i64; 360];
    let mut population_sum = 0i64;
    let mut colors_hct = Vec::with_capacity(colors_to_population.len());
    for &(argb, population) in colors_to_population {
        let hct = Hct::from_int(argb);
        let hue = hct.hue as i32;
        if !(0..360).contains(&hue) {
            continue;
        }
        hue_population[hue as usize] += population as i64;
        population_sum += population as i64;
        colors_hct.push(hct);
    }

    let mut hue_excited_proportions = [0f64; 360];
    if population_sum != 0 {
        for hue in 0..360 {
            let proportion = hue_population[hue] as f64 / population_sum as f64;
            for i in hue as i32 - 14..hue as i32 + 16 {
                let neighbor = ((i % 360) + 360) % 360;
                hue_excited_proportions[neighbor as usize] += proportion;
            }
        }
    }

    let mut scored: Vec<(f64, Hct)> = Vec::new();
    for hct in colors_hct {
        let hue = {
            let rounded = py_round(hct.hue);
            let mut d = rounded % 360.0;
            if d < 0.0 {
                d += 360.0;
            }
            d as usize
        };
        let proportion = hue_excited_proportions[hue.min(359)];

        if hct.chroma < CUTOFF_CHROMA || proportion <= CUTOFF_EXCITED_PROPORTION {
            continue;
        }

        let proportion_score = proportion * 100.0 * WEIGHT_PROPORTION;
        let chroma_weight = if hct.chroma < TARGET_CHROMA {
            WEIGHT_CHROMA_BELOW
        } else {
            WEIGHT_CHROMA_ABOVE
        };
        let chroma_score = (hct.chroma - TARGET_CHROMA) * chroma_weight;
        scored.push((proportion_score + chroma_score, hct));
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut chosen: Vec<Hct> = Vec::new();
    for difference in (15..=90).rev() {
        chosen.clear();
        for (_, hct) in &scored {
            let duplicate = chosen
                .iter()
                .any(|chosen_hct| difference_degrees(hct.hue, chosen_hct.hue) < difference as f64);
            if !duplicate {
                chosen.push(*hct);
            }
            if chosen.len() >= DESIRED {
                break;
            }
        }
        if chosen.len() >= DESIRED {
            break;
        }
    }

    if chosen.is_empty() {
        return vec![FALLBACK];
    }
    chosen.iter().map(|hct| hct.to_int()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celebi_and_score_match_the_python_reference() {
        let mut pixels: Vec<u32> = (0..16u32)
            .map(|i| 0xFF000000 | ((i * 16) << 16) | ((255 - i * 13) << 8) | ((i * 37) % 256))
            .collect();
        for _ in 0..40 {
            pixels.push(0xFF0AC81E);
        }
        for _ in 0..25 {
            pixels.push(0xFFC81E3C);
        }
        let colors = quantize_celebi(&pixels, 128);
        assert_eq!(colors.len(), 18);
        assert_eq!(colors[0], (0xFF00FF00, 1));
        assert_eq!(colors[1], (0xFF0AC81E, 40));
        assert_eq!(colors[14], (0xFFC81E3C, 25));
        assert_eq!(
            score(&colors),
            vec![0xFF00FF00, 0xFFF03C2B, 0xFFD056E1, 0xFF60B1DE]
        );
    }
}
