//! Temperature cache and dislike filter, ported for the fidelity/content schemes.

use crate::color::{lab_from_argb, py_round, sanitize_degrees_double};
use crate::hct::Hct;

pub struct TemperatureCache {
    input: Hct,
    hcts: Vec<Hct>,
    temps: Vec<f64>,
    order: Vec<usize>,
}

impl TemperatureCache {
    pub fn new(input: Hct) -> Self {
        let hcts: Vec<Hct> = (0..=360)
            .map(|hue| Hct::from_hct(hue as f64, input.chroma, input.tone))
            .collect();
        let mut temps: Vec<f64> = hcts.iter().map(Self::raw_temperature).collect();
        temps.push(Self::raw_temperature(&input));

        let mut order: Vec<usize> = (0..=361).collect();
        order.sort_by(|a, b| temps[*a].partial_cmp(&temps[*b]).unwrap());

        TemperatureCache {
            input,
            hcts,
            temps,
            order,
        }
    }

    fn hct(&self, index: usize) -> &Hct {
        if index == 361 {
            &self.input
        } else {
            &self.hcts[index]
        }
    }

    fn warmest_index(&self) -> usize {
        *self.order.last().unwrap()
    }

    fn coldest_index(&self) -> usize {
        self.order[0]
    }

    fn temp(&self, index: usize) -> f64 {
        self.temps[index]
    }

    fn relative_temperature(&self, index: usize) -> f64 {
        let range_temp = self.temp(self.warmest_index()) - self.temp(self.coldest_index());
        if range_temp == 0.0 {
            return 0.5;
        }
        (self.temp(index) - self.temp(self.coldest_index())) / range_temp
    }

    fn input_relative_temperature(&self) -> f64 {
        self.relative_temperature(361)
    }

    pub fn analogous(&self, count: usize, divisions: usize) -> Vec<Hct> {
        let start_hue = py_round(self.input.hue) as usize;
        let last_temp = self.relative_temperature(start_hue);
        let mut all_colors = vec![start_hue];

        let mut absolute_total_temp_delta = 0.0;
        let mut last_temp = last_temp;
        for i in 0..360 {
            let hue = ((start_hue + i) as f64 % 360.0) as usize;
            let temp = self.relative_temperature(hue);
            let temp_delta = (temp - last_temp).abs();
            last_temp = temp;
            absolute_total_temp_delta += temp_delta;
        }

        let mut hue_addend = 1;
        let temp_step = absolute_total_temp_delta / divisions as f64;
        let mut total_temp_delta = 0.0;
        let mut last_temp = self.relative_temperature(start_hue);

        while all_colors.len() < divisions {
            let hue = ((start_hue + hue_addend) as f64 % 360.0) as usize;
            let temp = self.relative_temperature(hue);
            let temp_delta = (temp - last_temp).abs();
            total_temp_delta += temp_delta;

            let mut desired_total_temp_delta_for_index = all_colors.len() as f64 * temp_step;
            let mut index_satisfied = total_temp_delta >= desired_total_temp_delta_for_index;
            let mut index_addend = 1;

            while index_satisfied && all_colors.len() < divisions {
                all_colors.push(hue);
                desired_total_temp_delta_for_index =
                    (all_colors.len() + index_addend) as f64 * temp_step;
                index_satisfied = total_temp_delta >= desired_total_temp_delta_for_index;
                index_addend += 1;
            }

            last_temp = temp;
            hue_addend += 1;

            if hue_addend > 360 {
                while all_colors.len() < divisions {
                    all_colors.push(hue);
                }
                break;
            }
        }

        let mut answers: Vec<usize> = vec![361];

        let increase_hue_count = ((count - 1) as f64 / 2.0) as usize;
        for i in 1..=increase_hue_count {
            let index = wrap(0 - i as i64, all_colors.len());
            answers.insert(0, all_colors[index]);
        }

        let decrease_hue_count = count - increase_hue_count - 1;
        for i in 1..=decrease_hue_count {
            let index = wrap(i as i64, all_colors.len());
            answers.push(all_colors[index]);
        }

        answers.into_iter().map(|index| *self.hct(index)).collect()
    }

    pub fn complement(&self) -> Result<Hct, String> {
        let coldest = self.coldest_index();
        let warmest = self.warmest_index();
        let coldest_hue = self.hct(coldest).hue;
        let coldest_temp = self.temp(coldest);
        let warmest_hue = self.hct(warmest).hue;
        let warmest_temp = self.temp(warmest);
        let range_temp = warmest_temp - coldest_temp;
        let start_hue_is_coldest_to_warmest = is_between(self.input.hue, coldest_hue, warmest_hue);
        let start_hue = if start_hue_is_coldest_to_warmest {
            warmest_hue
        } else {
            coldest_hue
        };
        let end_hue = if start_hue_is_coldest_to_warmest {
            coldest_hue
        } else {
            warmest_hue
        };
        let direction_of_rotation = 1.0;
        let mut smallest_error = 1000.0;
        let mut answer = py_round(self.input.hue) as usize;

        let complement_relative_temp = 1.0 - self.input_relative_temperature();
        for hue_addend in 0..360 {
            let hue =
                sanitize_degrees_double(start_hue + direction_of_rotation * hue_addend as f64);
            if !is_between(hue, start_hue, end_hue) {
                continue;
            }
            let possible_answer = py_round(hue) as usize;
            if range_temp == 0.0 {
                return Err(
                    "temperature range is zero; the Python reference raises ZeroDivisionError"
                        .to_string(),
                );
            }
            let relative_temp = (self.temp(possible_answer) - coldest_temp) / range_temp;
            let error = (complement_relative_temp - relative_temp).abs();
            if error < smallest_error {
                smallest_error = error;
                answer = possible_answer;
            }
        }

        Ok(*self.hct(answer))
    }

    fn raw_temperature(color: &Hct) -> f64 {
        let lab = lab_from_argb(color.to_int());
        let hue = sanitize_degrees_double(lab[2].atan2(lab[1]) * 180.0 / std::f64::consts::PI);
        let chroma = ((lab[1] * lab[1]) + (lab[2] * lab[2])).sqrt();
        let temperature = -0.5
            + 0.02
                * chroma.powf(1.07)
                * (sanitize_degrees_double(hue - 50.0) * std::f64::consts::PI / 180.0).cos();
        temperature
    }
}

fn wrap(mut index: i64, len: usize) -> usize {
    while index < 0 {
        index += len as i64;
    }
    if index >= len as i64 {
        index %= len as i64;
    }
    index as usize
}

fn is_between(angle: f64, a: f64, b: f64) -> bool {
    if a < b {
        a <= angle && angle <= b
    } else {
        a <= angle || angle <= b
    }
}

pub struct DislikeAnalyzer;

impl DislikeAnalyzer {
    pub fn is_disliked(hct: &Hct) -> bool {
        let hue_passes = py_round(hct.hue) >= 90.0 && py_round(hct.hue) <= 111.0;
        let chroma_passes = py_round(hct.chroma) > 16.0;
        let tone_passes = py_round(hct.tone) < 65.0;
        hue_passes && chroma_passes && tone_passes
    }

    pub fn fix_if_disliked(hct: Hct) -> Hct {
        if Self::is_disliked(&hct) {
            Hct::from_hct(hct.hue, hct.chroma, 70.0)
        } else {
            hct
        }
    }
}
