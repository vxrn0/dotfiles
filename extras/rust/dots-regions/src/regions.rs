use image::RgbImage;

/// Longest side of the working copy. Screenshots up to 1080p are processed at
/// native resolution so one-pixel UI borders survive; 4K and larger are halved.
const MAX_WORK_SIDE: usize = 1920;
/// One 3x3 dilation closes one-pixel gaps in an edge outline.
const DILATE: i64 = 1;
/// Edge threshold: mean + this many standard deviations, kept within
/// [MIN_EDGE, MAX_EDGE]. A bright photo would otherwise push the threshold above
/// the Sobel magnitude of a 24-level UI border and lose whole panels.
const ADAPTIVE_STD: f64 = 1.2;
const MIN_EDGE: f64 = 40.0;
const MAX_EDGE: f64 = 96.0;
const NMS_IOU: f64 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl Rect {
    fn area(self) -> i64 {
        self.width * self.height
    }
}

#[derive(Clone, Copy)]
pub struct Params {
    pub min_width: i64,
    pub min_height: i64,
    pub max_width: Option<i64>,
    pub max_height: Option<i64>,
}

pub fn find_regions(image: &RgbImage, params: Params) -> Vec<Rect> {
    let (orig_w, orig_h) = image.dimensions();
    let orig_w = orig_w as i64;
    let orig_h = orig_h as i64;

    let factor = (image.width().max(image.height()) as usize)
        .div_ceil(MAX_WORK_SIDE)
        .max(1);
    let (gray, work_w, work_h) = to_luma(image, factor);
    let edges = edge_map(&gray, work_w, work_h);
    let joined = dilate(&edges, work_w, work_h);

    let mut candidates = Vec::new();
    for b in components(&joined, work_w, work_h) {
        // Undo the dilation so the box sits on the edge, not two pixels outside it.
        let x0 = b.x0 + DILATE;
        let y0 = b.y0 + DILATE;
        let x1 = b.x1 + 1 - DILATE;
        let y1 = b.y1 + 1 - DILATE;
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let x = (x0 * factor as i64).clamp(0, orig_w);
        let y = (y0 * factor as i64).clamp(0, orig_h);
        let right = (x1 * factor as i64).clamp(x, orig_w);
        let bottom = (y1 * factor as i64).clamp(y, orig_h);
        let rect = Rect {
            x,
            y,
            width: right - x,
            height: bottom - y,
        };
        if accepts(rect, orig_w, orig_h, params) {
            candidates.push(rect);
        }
    }
    non_max_suppression(candidates)
}

/// Python's filter, including its strict comparisons and the whole-image skip.
fn accepts(rect: Rect, orig_w: i64, orig_h: i64, params: Params) -> bool {
    if rect.width == orig_w && rect.height == orig_h && rect.x == 0 && rect.y == 0 {
        return false;
    }
    rect.width > params.min_width
        && rect.height > params.min_height
        && params.max_width.is_none_or(|max| rect.width < max)
        && params.max_height.is_none_or(|max| rect.height < max)
}

/// Largest first, then greedily drop anything overlapping the kept box by 0.7.
fn non_max_suppression(mut regions: Vec<Rect>) -> Vec<Rect> {
    regions.sort_by_key(|rect| std::cmp::Reverse(rect.area()));
    let mut keep: Vec<Rect> = Vec::new();
    'next: for rect in regions {
        for kept in &keep {
            if iou(rect, *kept) >= NMS_IOU {
                continue 'next;
            }
        }
        keep.push(rect);
    }
    keep
}

fn iou(a: Rect, b: Rect) -> f64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    let intersection = (right - left).max(0) as f64 * (bottom - top).max(0) as f64;
    let union = (a.area() + b.area()) as f64 - intersection;
    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

fn luma(pixel: &image::Rgb<u8>) -> u32 {
    (pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000
}

fn to_luma(image: &RgbImage, factor: usize) -> (Vec<u8>, usize, usize) {
    let width = (image.width() as usize / factor).max(1);
    let height = (image.height() as usize / factor).max(1);
    let mut gray = vec![0u8; width * height];
    let cells = (factor * factor) as u32;
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0u32;
            for dy in 0..factor {
                for dx in 0..factor {
                    let px = (x * factor + dx).min(image.width() as usize - 1);
                    let py = (y * factor + dy).min(image.height() as usize - 1);
                    sum += luma(image.get_pixel(px as u32, py as u32));
                }
            }
            gray[y * width + x] = (sum / cells) as u8;
        }
    }
    (gray, width, height)
}

fn edge_map(gray: &[u8], width: usize, height: usize) -> Vec<bool> {
    let mut magnitude = vec![0u16; width * height];
    let mut sum = 0f64;
    let mut square_sum = 0f64;
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let i = y * width + x;
            let gx =
                (gray[i - width + 1] as i32 + 2 * gray[i + 1] as i32 + gray[i + width + 1] as i32)
                    - (gray[i - width - 1] as i32
                        + 2 * gray[i - 1] as i32
                        + gray[i + width - 1] as i32);
            let gy = (gray[i + width - 1] as i32
                + 2 * gray[i + width] as i32
                + gray[i + width + 1] as i32)
                - (gray[i - width - 1] as i32
                    + 2 * gray[i - width] as i32
                    + gray[i - width + 1] as i32);
            let value = gx.unsigned_abs() + gy.unsigned_abs();
            magnitude[i] = value.min(u16::MAX as u32) as u16;
            sum += value as f64;
            square_sum += (value * value) as f64;
        }
    }
    let count = (width * height) as f64;
    let mean = sum / count;
    let variance = (square_sum / count - mean * mean).max(0.0);
    let threshold = (mean + ADAPTIVE_STD * variance.sqrt()).clamp(MIN_EDGE, MAX_EDGE);
    magnitude
        .iter()
        .map(|&value| value as f64 >= threshold)
        .collect()
}

fn dilate(mask: &[bool], width: usize, height: usize) -> Vec<bool> {
    let mut horizontal = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            horizontal[i] = mask[i] || (x > 0 && mask[i - 1]) || (x + 1 < width && mask[i + 1]);
        }
    }
    let mut out = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            out[i] = horizontal[i]
                || (y > 0 && horizontal[i - width])
                || (y + 1 < height && horizontal[i + width]);
        }
    }
    out
}

#[derive(Clone, Copy)]
struct BBox {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

fn components(mask: &[bool], width: usize, height: usize) -> Vec<BBox> {
    let mut seen = vec![false; mask.len()];
    let mut stack = Vec::new();
    let mut found = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut bbox = BBox {
            x0: (start % width) as i64,
            y0: (start / width) as i64,
            x1: (start % width) as i64,
            y1: (start / width) as i64,
        };
        while let Some(i) = stack.pop() {
            let x = (i % width) as i64;
            let y = (i / width) as i64;
            bbox.x0 = bbox.x0.min(x);
            bbox.y0 = bbox.y0.min(y);
            bbox.x1 = bbox.x1.max(x);
            bbox.y1 = bbox.y1.max(y);
            let ny0 = (y - 1).max(0);
            let nx0 = (x - 1).max(0);
            for ny in ny0..=(y + 1).min(height as i64 - 1) {
                for nx in nx0..=(x + 1).min(width as i64 - 1) {
                    let j = ny as usize * width + nx as usize;
                    if mask[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        found.push(bbox);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(min_width: i64, min_height: i64) -> Params {
        Params {
            min_width,
            min_height,
            max_width: None,
            max_height: None,
        }
    }

    #[test]
    fn filters_use_pythons_strict_boundaries() {
        let rect = Rect {
            x: 10,
            y: 10,
            width: 200,
            height: 100,
        };
        assert!(!accepts(rect, 640, 480, params(200, 100)));
        assert!(accepts(rect, 640, 480, params(199, 99)));

        let mut with_max = params(0, 0);
        with_max.max_width = Some(200);
        with_max.max_height = Some(100);
        assert!(!accepts(rect, 640, 480, with_max));
        with_max.max_width = Some(201);
        with_max.max_height = Some(101);
        assert!(accepts(rect, 640, 480, with_max));
    }

    #[test]
    fn the_whole_image_never_counts() {
        let whole = Rect {
            x: 0,
            y: 0,
            width: 640,
            height: 480,
        };
        assert!(!accepts(whole, 640, 480, params(0, 0)));
    }

    #[test]
    fn nms_keeps_the_larger_box_and_drops_duplicates() {
        let big = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let same = big;
        let small_inside = Rect {
            x: 5,
            y: 5,
            width: 90,
            height: 90,
        };
        let apart = Rect {
            x: 200,
            y: 0,
            width: 100,
            height: 100,
        };
        let kept = non_max_suppression(vec![same, small_inside, apart, big]);
        assert_eq!(kept.len(), 2);
        assert!(kept.contains(&big));
        assert!(kept.contains(&apart));
    }
}
