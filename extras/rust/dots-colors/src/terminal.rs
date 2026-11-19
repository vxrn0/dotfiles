//! Terminal colour harmonisation, ported from the driver script.

use crate::color::*;
use crate::hct::Hct;

pub fn harmonize(design_color: u32, source_color: u32, threshold: f64, harmony: f64) -> u32 {
    let from_hct = Hct::from_int(design_color);
    let to_hct = Hct::from_int(source_color);
    let difference = difference_degrees(from_hct.hue, to_hct.hue);
    let rotation_degrees = (difference * harmony).min(threshold);
    let output_hue = sanitize_degrees_double(
        from_hct.hue + rotation_degrees * rotation_direction(from_hct.hue, to_hct.hue),
    );
    Hct::from_hct(output_hue, from_hct.chroma, from_hct.tone).to_int()
}

pub fn boost_chroma_tone(argb: u32, chroma: f64, tone: f64) -> u32 {
    let hct = Hct::from_int(argb);
    Hct::from_hct(hct.hue, hct.chroma * chroma, hct.tone * tone).to_int()
}
