//! The nine scheme variants the Python driver dispatches to.

use crate::color::sanitize_degrees_double;
use crate::hct::Hct;
use crate::palette::TonalPalette;
use crate::temperature::{DislikeAnalyzer, TemperatureCache};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Monochrome,
    Spritz,
    TonalSpot,
    Vibrant,
    Expressive,
    Fidelity,
    Content,
    Rainbow,
    FruitSalad,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Primary,
    Secondary,
    Tertiary,
    Neutral,
    NeutralVariant,
    Error,
}

pub struct DynamicScheme {
    pub source_hct: Hct,
    pub variant: Variant,
    pub contrast_level: f64,
    pub is_dark: bool,
    pub primary_palette: TonalPalette,
    pub secondary_palette: TonalPalette,
    pub tertiary_palette: TonalPalette,
    pub neutral_palette: TonalPalette,
    pub neutral_variant_palette: TonalPalette,
    pub error_palette: TonalPalette,
}

impl DynamicScheme {
    pub fn for_scheme(
        name: &str,
        source: Hct,
        is_dark: bool,
        contrast_level: f64,
    ) -> Result<Self, String> {
        let hue = source.hue;
        let chroma = source.chroma;

        let vibrant_hues = [0.0, 41.0, 61.0, 101.0, 131.0, 181.0, 251.0, 301.0, 360.0];
        let vibrant_secondary = [18.0, 15.0, 10.0, 12.0, 15.0, 18.0, 15.0, 12.0, 12.0];
        let vibrant_tertiary = [35.0, 30.0, 20.0, 25.0, 30.0, 35.0, 30.0, 25.0, 25.0];
        let expressive_hues = [0.0, 21.0, 51.0, 121.0, 151.0, 191.0, 271.0, 321.0, 360.0];
        let expressive_secondary = [45.0, 95.0, 45.0, 20.0, 45.0, 90.0, 45.0, 45.0, 45.0];
        let expressive_tertiary = [120.0, 120.0, 20.0, 45.0, 20.0, 15.0, 20.0, 120.0, 120.0];

        let (variant, primary, secondary, tertiary, neutral, neutral_variant) = match name {
            "scheme-fruit-salad" => (
                Variant::FruitSalad,
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue - 50.0), 48.0),
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue - 50.0), 36.0),
                TonalPalette::from_hue_and_chroma(hue, 36.0),
                TonalPalette::from_hue_and_chroma(hue, 10.0),
                TonalPalette::from_hue_and_chroma(hue, 16.0),
            ),
            "scheme-expressive" => (
                Variant::Expressive,
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue + 240.0), 40.0),
                TonalPalette::from_hue_and_chroma(
                    Self::get_rotated_hue(&source, &expressive_hues, &expressive_secondary),
                    24.0,
                ),
                TonalPalette::from_hue_and_chroma(
                    Self::get_rotated_hue(&source, &expressive_hues, &expressive_tertiary),
                    32.0,
                ),
                TonalPalette::from_hue_and_chroma(hue + 15.0, 8.0),
                TonalPalette::from_hue_and_chroma(hue + 15.0, 12.0),
            ),
            "scheme-monochrome" => (
                Variant::Monochrome,
                TonalPalette::from_hue_and_chroma(hue, 0.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
            ),
            "scheme-rainbow" => (
                Variant::Rainbow,
                TonalPalette::from_hue_and_chroma(hue, 48.0),
                TonalPalette::from_hue_and_chroma(hue, 16.0),
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue + 60.0), 24.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
                TonalPalette::from_hue_and_chroma(hue, 0.0),
            ),
            "scheme-tonal-spot" => (
                Variant::TonalSpot,
                TonalPalette::from_hue_and_chroma(hue, 36.0),
                TonalPalette::from_hue_and_chroma(hue, 16.0),
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue + 60.0), 24.0),
                TonalPalette::from_hue_and_chroma(hue, 6.0),
                TonalPalette::from_hue_and_chroma(hue, 8.0),
            ),
            "scheme-neutral" => (
                Variant::Spritz,
                TonalPalette::from_hue_and_chroma(hue, 12.0),
                TonalPalette::from_hue_and_chroma(hue, 8.0),
                TonalPalette::from_hue_and_chroma(hue, 16.0),
                TonalPalette::from_hue_and_chroma(hue, 2.0),
                TonalPalette::from_hue_and_chroma(hue, 2.0),
            ),
            "scheme-fidelity" => {
                let tertiary = TemperatureCache::new(source).complement()?;
                (
                    Variant::Fidelity,
                    TonalPalette::from_hue_and_chroma(hue, chroma),
                    TonalPalette::from_hue_and_chroma(hue, (chroma - 32.0).max(chroma * 0.5)),
                    TonalPalette::from_int(DislikeAnalyzer::fix_if_disliked(tertiary).to_int()),
                    TonalPalette::from_hue_and_chroma(hue, chroma / 8.0),
                    TonalPalette::from_hue_and_chroma(hue, chroma / 8.0 + 4.0),
                )
            }
            "scheme-content" => {
                let analogous = TemperatureCache::new(source).analogous(3, 6);
                let tertiary = DislikeAnalyzer::fix_if_disliked(analogous[2]);
                (
                    Variant::Content,
                    TonalPalette::from_hue_and_chroma(hue, chroma),
                    TonalPalette::from_hue_and_chroma(hue, (chroma - 32.0).max(chroma * 0.5)),
                    TonalPalette::from_int(tertiary.to_int()),
                    TonalPalette::from_hue_and_chroma(hue, chroma / 8.0),
                    TonalPalette::from_hue_and_chroma(hue, chroma / 8.0 + 4.0),
                )
            }
            "scheme-vibrant" => (
                Variant::Vibrant,
                TonalPalette::from_hue_and_chroma(hue, 200.0),
                TonalPalette::from_hue_and_chroma(
                    Self::get_rotated_hue(&source, &vibrant_hues, &vibrant_secondary),
                    24.0,
                ),
                TonalPalette::from_hue_and_chroma(
                    Self::get_rotated_hue(&source, &vibrant_hues, &vibrant_tertiary),
                    32.0,
                ),
                TonalPalette::from_hue_and_chroma(hue, 10.0),
                TonalPalette::from_hue_and_chroma(hue, 12.0),
            ),
            _ => (
                Variant::TonalSpot,
                TonalPalette::from_hue_and_chroma(hue, 36.0),
                TonalPalette::from_hue_and_chroma(hue, 16.0),
                TonalPalette::from_hue_and_chroma(sanitize_degrees_double(hue + 60.0), 24.0),
                TonalPalette::from_hue_and_chroma(hue, 6.0),
                TonalPalette::from_hue_and_chroma(hue, 8.0),
            ),
        };

        Ok(DynamicScheme {
            source_hct: source,
            variant,
            contrast_level,
            is_dark,
            primary_palette: primary,
            secondary_palette: secondary,
            tertiary_palette: tertiary,
            neutral_palette: neutral,
            neutral_variant_palette: neutral_variant,
            error_palette: TonalPalette::from_hue_and_chroma(25.0, 84.0),
        })
    }

    fn get_rotated_hue(source_color: &Hct, hues: &[f64], rotations: &[f64]) -> f64 {
        let source_hue = source_color.hue;
        if rotations.len() == 1 {
            return sanitize_degrees_double(source_hue + rotations[0]);
        }
        let size = hues.len();
        for i in 0..size - 1 {
            if hues[i] < source_hue && source_hue < hues[i + 1] {
                return sanitize_degrees_double(source_hue + rotations[i]);
            }
        }
        source_hue
    }

    pub fn is_fidelity(&self) -> bool {
        self.variant == Variant::Fidelity || self.variant == Variant::Content
    }

    pub fn is_monochrome(&self) -> bool {
        self.variant == Variant::Monochrome
    }

    pub fn palette(&self, kind: PaletteKind) -> &TonalPalette {
        match kind {
            PaletteKind::Primary => &self.primary_palette,
            PaletteKind::Secondary => &self.secondary_palette,
            PaletteKind::Tertiary => &self.tertiary_palette,
            PaletteKind::Neutral => &self.neutral_palette,
            PaletteKind::NeutralVariant => &self.neutral_variant_palette,
            PaletteKind::Error => &self.error_palette,
        }
    }

    pub fn tone(&self, kind: PaletteKind, tone: f64) -> u32 {
        self.palette(kind).tone(tone)
    }

    pub fn key_tone(&self, kind: PaletteKind) -> f64 {
        self.palette(kind).key_color.tone
    }
}
