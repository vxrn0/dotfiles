//! CAM16 conversion, trimmed to the hue/chroma that `Hct` reads.

use crate::color::*;
use crate::hct::ViewingConditions;
use std::f64::consts::PI;

pub struct Cam16 {
    pub hue: f64,
    pub chroma: f64,
}

impl Cam16 {
    pub fn from_int(argb: u32) -> Self {
        Self::from_int_in_viewing_conditions(argb, ViewingConditions::default())
    }

    pub fn from_int_in_viewing_conditions(argb: u32, vc: &ViewingConditions) -> Self {
        let red = ((argb & 0x00FF0000) >> 16) as f64;
        let green = ((argb & 0x0000FF00) >> 8) as f64;
        let blue = (argb & 0x000000FF) as f64;
        let red_l = linearized(red);
        let green_l = linearized(green);
        let blue_l = linearized(blue);

        let x = 0.41233895 * red_l + 0.35762064 * green_l + 0.18051042 * blue_l;
        let y = 0.2126 * red_l + 0.7152 * green_l + 0.0722 * blue_l;
        let z = 0.01932141 * red_l + 0.11916382 * green_l + 0.95034478 * blue_l;

        let r_c = 0.401288 * x + 0.650173 * y - 0.051461 * z;
        let g_c = -0.250268 * x + 1.204414 * y + 0.045854 * z;
        let b_c = -0.002079 * x + 0.048952 * y + 0.953127 * z;

        let r_d = vc.rgb_d[0] * r_c;
        let g_d = vc.rgb_d[1] * g_c;
        let b_d = vc.rgb_d[2] * b_c;

        let r_af = (vc.fl * r_d.abs() / 100.0).powf(0.42);
        let g_af = (vc.fl * g_d.abs() / 100.0).powf(0.42);
        let b_af = (vc.fl * b_d.abs() / 100.0).powf(0.42);

        let r_a = (signum(r_d) * 400.0 * r_af) / (r_af + 27.13);
        let g_a = (signum(g_d) * 400.0 * g_af) / (g_af + 27.13);
        let b_a = (signum(b_d) * 400.0 * b_af) / (b_af + 27.13);

        let a = (11.0 * r_a + -12.0 * g_a + b_a) / 11.0;
        let b = (r_a + g_a - 2.0 * b_a) / 9.0;
        let u = (20.0 * r_a + 20.0 * g_a + 21.0 * b_a) / 20.0;
        let p2 = (40.0 * r_a + 20.0 * g_a + b_a) / 20.0;
        let atan2 = b.atan2(a);
        let atan_degrees = (atan2 * 180.0) / PI;
        let hue = if atan_degrees < 0.0 {
            atan_degrees + 360.0
        } else {
            atan_degrees
        };
        let ac = p2 * vc.nbb;
        let j = 100.0 * (ac / vc.aw).powf(vc.c * vc.z);
        // q is unused by Hct.
        let hue_prime = if hue < 20.14 { hue + 360.0 } else { hue };
        let e_hue = 0.25 * (((hue_prime * PI) / 180.0 + 2.0).cos() + 3.8);
        let p1 = (50000.0 / 13.0) * e_hue * vc.nc * vc.ncb;
        let t = (p1 * (a * a + b * b).sqrt()) / (u + 0.305);
        let alpha = t.powf(0.9) * (1.64 - 0.29_f64.powf(vc.n)).powf(0.73);
        let chroma = alpha * (j / 100.0).sqrt();

        Cam16 { hue, chroma }
    }
}
