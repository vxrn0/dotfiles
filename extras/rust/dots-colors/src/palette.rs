//! Tonal palette, including the key-colour search.

use crate::color::py_round;
use crate::hct::Hct;

pub struct TonalPalette {
    pub hue: f64,
    pub chroma: f64,
    pub key_color: Hct,
}

impl TonalPalette {
    pub fn from_int(argb: u32) -> Self {
        Self::from_hct(Hct::from_int(argb))
    }

    pub fn from_hct(hct: Hct) -> Self {
        TonalPalette {
            hue: hct.hue,
            chroma: hct.chroma,
            key_color: hct,
        }
    }

    pub fn from_hue_and_chroma(hue: f64, chroma: f64) -> Self {
        TonalPalette {
            hue,
            chroma,
            key_color: Self::create_key_color(hue, chroma),
        }
    }

    fn create_key_color(hue: f64, chroma: f64) -> Hct {
        let start_tone = 50.0;
        let mut smallest_delta_hct = Hct::from_hct(hue, chroma, start_tone);
        let mut smallest_delta = (smallest_delta_hct.chroma - chroma).abs();

        for delta in 1..50 {
            if py_round(chroma) == py_round(smallest_delta_hct.chroma) {
                return smallest_delta_hct;
            }

            let hct_add = Hct::from_hct(hue, chroma, start_tone + delta as f64);
            let hct_add_delta = (hct_add.chroma - chroma).abs();
            if hct_add_delta < smallest_delta {
                smallest_delta = hct_add_delta;
                smallest_delta_hct = hct_add;
            }

            let hct_subtract = Hct::from_hct(hue, chroma, start_tone - delta as f64);
            let hct_subtract_delta = (hct_subtract.chroma - chroma).abs();
            if hct_subtract_delta < smallest_delta {
                smallest_delta = hct_subtract_delta;
                smallest_delta_hct = hct_subtract;
            }
        }

        smallest_delta_hct
    }

    /// The same 8-bit colour as `tone(...)` then `Hct.from_int(...)` in Python.
    pub fn tone(&self, tone: f64) -> u32 {
        Hct::from_hct(self.hue, self.chroma, tone).to_int()
    }

    pub fn get_hct(&self, tone: f64) -> Hct {
        Hct::from_int(self.tone(tone))
    }
}
