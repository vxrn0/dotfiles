//! WCAG contrast helpers used by the dynamic colour roles.

use crate::color::*;

pub struct Contrast;

impl Contrast {
    pub fn ratio_of_tones(tone_a: f64, tone_b: f64) -> f64 {
        let tone_a = clamp_double(0.0, 100.0, tone_a);
        let tone_b = clamp_double(0.0, 100.0, tone_b);
        Self::ratio_of_ys(y_from_lstar(tone_a), y_from_lstar(tone_b))
    }

    pub fn ratio_of_ys(y1: f64, y2: f64) -> f64 {
        let lighter = if y1 > y2 { y1 } else { y2 };
        let darker = if lighter == y2 { y1 } else { y2 };
        (lighter + 5.0) / (darker + 5.0)
    }

    pub fn lighter(tone: f64, ratio: f64) -> f64 {
        if tone < 0.0 || tone > 100.0 {
            return -1.0;
        }

        let dark_y = y_from_lstar(tone);
        let light_y = ratio * (dark_y + 5.0) - 5.0;
        let real_contrast = Self::ratio_of_ys(light_y, dark_y);
        let delta = (real_contrast - ratio).abs();
        if real_contrast < ratio && delta > 0.04 {
            return -1.0;
        }

        let return_value = lstar_from_y(light_y) + 0.4;
        if return_value < 0.0 || return_value > 100.0 {
            return -1.0;
        }
        return_value
    }

    pub fn darker(tone: f64, ratio: f64) -> f64 {
        if tone < 0.0 || tone > 100.0 {
            return -1.0;
        }

        let light_y = y_from_lstar(tone);
        let dark_y = (light_y + 5.0) / ratio - 5.0;
        let real_contrast = Self::ratio_of_ys(light_y, dark_y);

        let delta = (real_contrast - ratio).abs();
        if real_contrast < ratio && delta > 0.04 {
            return -1.0;
        }

        let return_value = lstar_from_y(dark_y) - 0.4;
        if return_value < 0.0 || return_value > 100.0 {
            return -1.0;
        }
        return_value
    }

    pub fn lighter_unsafe(tone: f64, ratio: f64) -> f64 {
        let lighter_safe = Self::lighter(tone, ratio);
        if lighter_safe < 0.0 {
            100.0
        } else {
            lighter_safe
        }
    }

    pub fn darker_unsafe(tone: f64, ratio: f64) -> f64 {
        let darker_safe = Self::darker(tone, ratio);
        if darker_safe < 0.0 {
            0.0
        } else {
            darker_safe
        }
    }
}

pub struct ContrastCurve {
    pub low: f64,
    pub normal: f64,
    pub medium: f64,
    pub high: f64,
}

impl ContrastCurve {
    pub const fn new(low: f64, normal: f64, medium: f64, high: f64) -> Self {
        ContrastCurve {
            low,
            normal,
            medium,
            high,
        }
    }

    pub fn get(&self, contrast_level: f64) -> f64 {
        if contrast_level <= -1.0 {
            self.low
        } else if contrast_level < 0.0 {
            lerp(self.low, self.normal, (contrast_level - (-1.0)) / 1.0)
        } else if contrast_level < 0.5 {
            lerp(self.normal, self.medium, (contrast_level - 0.0) / 0.5)
        } else if contrast_level < 1.0 {
            lerp(self.medium, self.high, (contrast_level - 0.5) / 0.5)
        } else {
            self.high
        }
    }
}
