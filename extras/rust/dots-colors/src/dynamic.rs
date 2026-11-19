//! Material dynamic colour roles, including the tone-delta machinery.
//!
//! The reference builds these roles from lambdas; the same table is written
//! out explicitly here. Contrast level is always 0.0 from the driver, but the
//! original branches are kept.

use crate::color::clamp_double;
use crate::contrast::{Contrast, ContrastCurve};
use crate::hct::Hct;
use crate::scheme::{DynamicScheme, PaletteKind};
use crate::temperature::DislikeAnalyzer;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    PrimaryPaletteKeyColor,
    SecondaryPaletteKeyColor,
    TertiaryPaletteKeyColor,
    NeutralPaletteKeyColor,
    NeutralVariantPaletteKeyColor,
    Background,
    OnBackground,
    Surface,
    SurfaceDim,
    SurfaceBright,
    SurfaceContainerLowest,
    SurfaceContainerLow,
    SurfaceContainer,
    SurfaceContainerHigh,
    SurfaceContainerHighest,
    OnSurface,
    SurfaceVariant,
    OnSurfaceVariant,
    InverseSurface,
    InverseOnSurface,
    Outline,
    OutlineVariant,
    Shadow,
    Scrim,
    SurfaceTint,
    Primary,
    OnPrimary,
    PrimaryContainer,
    OnPrimaryContainer,
    InversePrimary,
    Secondary,
    OnSecondary,
    SecondaryContainer,
    OnSecondaryContainer,
    Tertiary,
    OnTertiary,
    TertiaryContainer,
    OnTertiaryContainer,
    Error,
    OnError,
    ErrorContainer,
    OnErrorContainer,
    PrimaryFixed,
    PrimaryFixedDim,
    OnPrimaryFixed,
    OnPrimaryFixedVariant,
    SecondaryFixed,
    SecondaryFixedDim,
    OnSecondaryFixed,
    OnSecondaryFixedVariant,
    TertiaryFixed,
    TertiaryFixedDim,
    OnTertiaryFixed,
    OnTertiaryFixedVariant,
}

/// Definition order in `material_dynamic_colors.py`, with the names Python
/// prints. The hex for each role follows in the same order.
pub const ROLES: [(&str, Role); 54] = [
    ("primary_paletteKeyColor", Role::PrimaryPaletteKeyColor),
    ("secondary_paletteKeyColor", Role::SecondaryPaletteKeyColor),
    ("tertiary_paletteKeyColor", Role::TertiaryPaletteKeyColor),
    ("neutral_paletteKeyColor", Role::NeutralPaletteKeyColor),
    (
        "neutral_variant_paletteKeyColor",
        Role::NeutralVariantPaletteKeyColor,
    ),
    ("background", Role::Background),
    ("onBackground", Role::OnBackground),
    ("surface", Role::Surface),
    ("surfaceDim", Role::SurfaceDim),
    ("surfaceBright", Role::SurfaceBright),
    ("surfaceContainerLowest", Role::SurfaceContainerLowest),
    ("surfaceContainerLow", Role::SurfaceContainerLow),
    ("surfaceContainer", Role::SurfaceContainer),
    ("surfaceContainerHigh", Role::SurfaceContainerHigh),
    ("surfaceContainerHighest", Role::SurfaceContainerHighest),
    ("onSurface", Role::OnSurface),
    ("surfaceVariant", Role::SurfaceVariant),
    ("onSurfaceVariant", Role::OnSurfaceVariant),
    ("inverseSurface", Role::InverseSurface),
    ("inverseOnSurface", Role::InverseOnSurface),
    ("outline", Role::Outline),
    ("outlineVariant", Role::OutlineVariant),
    ("shadow", Role::Shadow),
    ("scrim", Role::Scrim),
    ("surfaceTint", Role::SurfaceTint),
    ("primary", Role::Primary),
    ("onPrimary", Role::OnPrimary),
    ("primaryContainer", Role::PrimaryContainer),
    ("onPrimaryContainer", Role::OnPrimaryContainer),
    ("inversePrimary", Role::InversePrimary),
    ("secondary", Role::Secondary),
    ("onSecondary", Role::OnSecondary),
    ("secondaryContainer", Role::SecondaryContainer),
    ("onSecondaryContainer", Role::OnSecondaryContainer),
    ("tertiary", Role::Tertiary),
    ("onTertiary", Role::OnTertiary),
    ("tertiaryContainer", Role::TertiaryContainer),
    ("onTertiaryContainer", Role::OnTertiaryContainer),
    ("error", Role::Error),
    ("onError", Role::OnError),
    ("errorContainer", Role::ErrorContainer),
    ("onErrorContainer", Role::OnErrorContainer),
    ("primaryFixed", Role::PrimaryFixed),
    ("primaryFixedDim", Role::PrimaryFixedDim),
    ("onPrimaryFixed", Role::OnPrimaryFixed),
    ("onPrimaryFixedVariant", Role::OnPrimaryFixedVariant),
    ("secondaryFixed", Role::SecondaryFixed),
    ("secondaryFixedDim", Role::SecondaryFixedDim),
    ("onSecondaryFixed", Role::OnSecondaryFixed),
    ("onSecondaryFixedVariant", Role::OnSecondaryFixedVariant),
    ("tertiaryFixed", Role::TertiaryFixed),
    ("tertiaryFixedDim", Role::TertiaryFixedDim),
    ("onTertiaryFixed", Role::OnTertiaryFixed),
    ("onTertiaryFixedVariant", Role::OnTertiaryFixedVariant),
];

#[derive(Clone, Copy)]
enum Polarity {
    Nearer,
    Lighter,
    #[allow(dead_code)]
    Darker,
}

struct ToneDelta {
    role_a: Role,
    role_b: Role,
    delta: f64,
    polarity: Polarity,
    stay_together: bool,
}

struct DynColor {
    palette: PaletteKind,
    background: Option<Role>,
    second_background: Option<Role>,
    contrast_curve: Option<ContrastCurve>,
    tone_delta_pair: Option<ToneDelta>,
}

fn plain(palette: PaletteKind) -> DynColor {
    DynColor {
        palette,
        background: None,
        second_background: None,
        contrast_curve: None,
        tone_delta_pair: None,
    }
}

fn with_bg(palette: PaletteKind, background: Role, curve: [f64; 4]) -> DynColor {
    DynColor {
        palette,
        background: Some(background),
        second_background: None,
        contrast_curve: Some(ContrastCurve::new(curve[0], curve[1], curve[2], curve[3])),
        tone_delta_pair: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn with_tdp(
    palette: PaletteKind,
    background: Role,
    curve: [f64; 4],
    role_a: Role,
    role_b: Role,
    delta: f64,
    polarity: Polarity,
    stay_together: bool,
) -> DynColor {
    DynColor {
        palette,
        background: Some(background),
        second_background: None,
        contrast_curve: Some(ContrastCurve::new(curve[0], curve[1], curve[2], curve[3])),
        tone_delta_pair: Some(ToneDelta {
            role_a,
            role_b,
            delta,
            polarity,
            stay_together,
        }),
    }
}

fn highest_surface(scheme: &DynamicScheme) -> Role {
    if scheme.is_dark {
        Role::SurfaceBright
    } else {
        Role::SurfaceDim
    }
}

const ON_BACKGROUND_CURVE: [f64; 4] = [3.0, 3.0, 4.5, 7.0];
const ON_SURFACE_CURVE: [f64; 4] = [4.5, 7.0, 11.0, 21.0];
const ON_SURFACE_VARIANT_CURVE: [f64; 4] = [3.0, 4.5, 7.0, 11.0];
const OUTLINE_CURVE: [f64; 4] = [1.5, 3.0, 4.5, 7.0];
const OUTLINE_VARIANT_CURVE: [f64; 4] = [1.0, 1.0, 3.0, 4.5];
const ACCENT_CURVE: [f64; 4] = [3.0, 4.5, 7.0, 7.0];
const CONTAINER_CURVE: [f64; 4] = [1.0, 1.0, 3.0, 4.5];
const FIXED_CURVE: [f64; 4] = [1.0, 1.0, 3.0, 4.5];

fn spec(role: Role, s: &DynamicScheme) -> DynColor {
    use PaletteKind as P;
    use Role::*;

    match role {
        PrimaryPaletteKeyColor => plain(P::Primary),
        SecondaryPaletteKeyColor => plain(P::Secondary),
        TertiaryPaletteKeyColor => plain(P::Tertiary),
        NeutralPaletteKeyColor => plain(P::Neutral),
        NeutralVariantPaletteKeyColor => plain(P::NeutralVariant),
        Background => plain(P::Neutral),
        OnBackground => with_bg(P::Neutral, Background, ON_BACKGROUND_CURVE),
        Surface => plain(P::Neutral),
        SurfaceDim => plain(P::Neutral),
        SurfaceBright => plain(P::Neutral),
        SurfaceContainerLowest => plain(P::Neutral),
        SurfaceContainerLow => plain(P::Neutral),
        SurfaceContainer => plain(P::Neutral),
        SurfaceContainerHigh => plain(P::Neutral),
        SurfaceContainerHighest => plain(P::Neutral),
        OnSurface => with_bg(P::Neutral, highest_surface(s), ON_SURFACE_CURVE),
        SurfaceVariant => plain(P::NeutralVariant),
        OnSurfaceVariant => with_bg(
            P::NeutralVariant,
            highest_surface(s),
            ON_SURFACE_VARIANT_CURVE,
        ),
        InverseSurface => plain(P::Neutral),
        InverseOnSurface => with_bg(P::Neutral, InverseSurface, ON_SURFACE_CURVE),
        Outline => with_bg(P::NeutralVariant, highest_surface(s), OUTLINE_CURVE),
        OutlineVariant => with_bg(P::NeutralVariant, highest_surface(s), OUTLINE_VARIANT_CURVE),
        Shadow => plain(P::Neutral),
        Scrim => plain(P::Neutral),
        SurfaceTint => plain(P::Primary),
        Primary => with_tdp(
            P::Primary,
            highest_surface(s),
            ACCENT_CURVE,
            PrimaryContainer,
            Role::Primary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnPrimary => with_bg(P::Primary, Primary, ON_SURFACE_CURVE),
        PrimaryContainer => with_tdp(
            P::Primary,
            highest_surface(s),
            CONTAINER_CURVE,
            PrimaryContainer,
            Role::Primary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnPrimaryContainer => with_bg(P::Primary, PrimaryContainer, ON_SURFACE_CURVE),
        InversePrimary => with_bg(P::Primary, InverseSurface, ACCENT_CURVE),
        Secondary => with_tdp(
            P::Secondary,
            highest_surface(s),
            ACCENT_CURVE,
            SecondaryContainer,
            Role::Secondary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnSecondary => with_bg(P::Secondary, Secondary, ON_SURFACE_CURVE),
        SecondaryContainer => with_tdp(
            P::Secondary,
            highest_surface(s),
            CONTAINER_CURVE,
            SecondaryContainer,
            Role::Secondary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnSecondaryContainer => with_bg(P::Secondary, SecondaryContainer, ON_SURFACE_CURVE),
        Tertiary => with_tdp(
            P::Tertiary,
            highest_surface(s),
            ACCENT_CURVE,
            TertiaryContainer,
            Role::Tertiary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnTertiary => with_bg(P::Tertiary, Tertiary, ON_SURFACE_CURVE),
        TertiaryContainer => with_tdp(
            P::Tertiary,
            highest_surface(s),
            CONTAINER_CURVE,
            TertiaryContainer,
            Role::Tertiary,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnTertiaryContainer => with_bg(P::Tertiary, TertiaryContainer, ON_SURFACE_CURVE),
        Error => with_tdp(
            P::Error,
            highest_surface(s),
            ACCENT_CURVE,
            ErrorContainer,
            Role::Error,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnError => with_bg(P::Error, Error, ON_SURFACE_CURVE),
        ErrorContainer => with_tdp(
            P::Error,
            highest_surface(s),
            CONTAINER_CURVE,
            ErrorContainer,
            Role::Error,
            10.0,
            Polarity::Nearer,
            false,
        ),
        OnErrorContainer => with_bg(P::Error, ErrorContainer, ON_SURFACE_CURVE),
        PrimaryFixed => with_tdp(
            P::Primary,
            highest_surface(s),
            FIXED_CURVE,
            PrimaryFixed,
            PrimaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        PrimaryFixedDim => with_tdp(
            P::Primary,
            highest_surface(s),
            FIXED_CURVE,
            PrimaryFixed,
            PrimaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        OnPrimaryFixed => DynColor {
            palette: P::Primary,
            background: Some(PrimaryFixedDim),
            second_background: Some(PrimaryFixed),
            contrast_curve: Some(ContrastCurve::new(4.5, 7.0, 11.0, 21.0)),
            tone_delta_pair: None,
        },
        OnPrimaryFixedVariant => DynColor {
            palette: P::Primary,
            background: Some(PrimaryFixedDim),
            second_background: Some(PrimaryFixed),
            contrast_curve: Some(ContrastCurve::new(3.0, 4.5, 7.0, 11.0)),
            tone_delta_pair: None,
        },
        SecondaryFixed => with_tdp(
            P::Secondary,
            highest_surface(s),
            FIXED_CURVE,
            SecondaryFixed,
            SecondaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        SecondaryFixedDim => with_tdp(
            P::Secondary,
            highest_surface(s),
            FIXED_CURVE,
            SecondaryFixed,
            SecondaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        OnSecondaryFixed => DynColor {
            palette: P::Secondary,
            background: Some(SecondaryFixedDim),
            second_background: Some(SecondaryFixed),
            contrast_curve: Some(ContrastCurve::new(4.5, 7.0, 11.0, 21.0)),
            tone_delta_pair: None,
        },
        OnSecondaryFixedVariant => DynColor {
            palette: P::Secondary,
            background: Some(SecondaryFixedDim),
            second_background: Some(SecondaryFixed),
            contrast_curve: Some(ContrastCurve::new(3.0, 4.5, 7.0, 11.0)),
            tone_delta_pair: None,
        },
        TertiaryFixed => with_tdp(
            P::Tertiary,
            highest_surface(s),
            FIXED_CURVE,
            TertiaryFixed,
            TertiaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        TertiaryFixedDim => with_tdp(
            P::Tertiary,
            highest_surface(s),
            FIXED_CURVE,
            TertiaryFixed,
            TertiaryFixedDim,
            10.0,
            Polarity::Lighter,
            true,
        ),
        OnTertiaryFixed => DynColor {
            palette: P::Tertiary,
            background: Some(TertiaryFixedDim),
            second_background: Some(TertiaryFixed),
            contrast_curve: Some(ContrastCurve::new(4.5, 7.0, 11.0, 21.0)),
            tone_delta_pair: None,
        },
        OnTertiaryFixedVariant => DynColor {
            palette: P::Tertiary,
            background: Some(TertiaryFixedDim),
            second_background: Some(TertiaryFixed),
            contrast_curve: Some(ContrastCurve::new(3.0, 4.5, 7.0, 11.0)),
            tone_delta_pair: None,
        },
    }
}

fn tone_raw(role: Role, s: &DynamicScheme) -> f64 {
    use Role::*;
    let dark = s.is_dark;
    match role {
        PrimaryPaletteKeyColor => s.key_tone(PaletteKind::Primary),
        SecondaryPaletteKeyColor => s.key_tone(PaletteKind::Secondary),
        TertiaryPaletteKeyColor => s.key_tone(PaletteKind::Tertiary),
        NeutralPaletteKeyColor => s.key_tone(PaletteKind::Neutral),
        NeutralVariantPaletteKeyColor => s.key_tone(PaletteKind::NeutralVariant),
        Background => {
            if dark {
                6.0
            } else {
                98.0
            }
        }
        OnBackground => {
            if dark {
                90.0
            } else {
                10.0
            }
        }
        Surface => {
            if dark {
                6.0
            } else {
                98.0
            }
        }
        SurfaceDim => {
            if dark {
                6.0
            } else {
                ContrastCurve::new(87.0, 87.0, 80.0, 75.0).get(s.contrast_level)
            }
        }
        SurfaceBright => {
            if dark {
                ContrastCurve::new(24.0, 24.0, 29.0, 34.0).get(s.contrast_level)
            } else {
                98.0
            }
        }
        SurfaceContainerLowest => {
            if dark {
                ContrastCurve::new(4.0, 4.0, 2.0, 0.0).get(s.contrast_level)
            } else {
                100.0
            }
        }
        SurfaceContainerLow => {
            if dark {
                ContrastCurve::new(10.0, 10.0, 11.0, 12.0).get(s.contrast_level)
            } else {
                ContrastCurve::new(96.0, 96.0, 96.0, 95.0).get(s.contrast_level)
            }
        }
        SurfaceContainer => {
            if dark {
                ContrastCurve::new(12.0, 12.0, 16.0, 20.0).get(s.contrast_level)
            } else {
                ContrastCurve::new(94.0, 94.0, 92.0, 90.0).get(s.contrast_level)
            }
        }
        SurfaceContainerHigh => {
            if dark {
                ContrastCurve::new(17.0, 17.0, 21.0, 25.0).get(s.contrast_level)
            } else {
                ContrastCurve::new(92.0, 92.0, 88.0, 85.0).get(s.contrast_level)
            }
        }
        SurfaceContainerHighest => {
            if dark {
                ContrastCurve::new(22.0, 22.0, 26.0, 30.0).get(s.contrast_level)
            } else {
                ContrastCurve::new(90.0, 90.0, 84.0, 80.0).get(s.contrast_level)
            }
        }
        OnSurface => {
            if dark {
                90.0
            } else {
                10.0
            }
        }
        SurfaceVariant => {
            if dark {
                30.0
            } else {
                90.0
            }
        }
        OnSurfaceVariant => {
            if dark {
                80.0
            } else {
                30.0
            }
        }
        InverseSurface => {
            if dark {
                90.0
            } else {
                20.0
            }
        }
        InverseOnSurface => {
            if dark {
                20.0
            } else {
                95.0
            }
        }
        Outline => {
            if dark {
                60.0
            } else {
                50.0
            }
        }
        OutlineVariant => {
            if dark {
                30.0
            } else {
                80.0
            }
        }
        Shadow | Scrim => 0.0,
        SurfaceTint => {
            if dark {
                80.0
            } else {
                40.0
            }
        }
        Primary => {
            if s.is_monochrome() {
                100.0
            } else if dark {
                80.0
            } else {
                40.0
            }
        }
        OnPrimary => {
            if s.is_monochrome() {
                10.0
            } else if dark {
                20.0
            } else {
                100.0
            }
        }
        PrimaryContainer => {
            if s.is_fidelity() {
                s.source_hct.tone
            } else if s.is_monochrome() {
                85.0
            } else if dark {
                30.0
            } else {
                90.0
            }
        }
        OnPrimaryContainer => {
            if s.is_fidelity() {
                foreground_tone(tone_raw(PrimaryContainer, s), 4.5)
            } else if s.is_monochrome() {
                0.0
            } else if dark {
                90.0
            } else {
                10.0
            }
        }
        InversePrimary => {
            if dark {
                40.0
            } else {
                80.0
            }
        }
        Secondary => {
            if dark {
                80.0
            } else {
                40.0
            }
        }
        OnSecondary => {
            if s.is_monochrome() {
                10.0
            } else if dark {
                20.0
            } else {
                100.0
            }
        }
        SecondaryContainer => find_desired_tone_for_sec(s),
        OnSecondaryContainer => {
            if !s.is_fidelity() {
                if dark {
                    90.0
                } else {
                    10.0
                }
            } else {
                foreground_tone(tone_raw(SecondaryContainer, s), 4.5)
            }
        }
        Tertiary => {
            if s.is_monochrome() {
                if dark {
                    90.0
                } else {
                    25.0
                }
            } else if dark {
                80.0
            } else {
                40.0
            }
        }
        OnTertiary => {
            if s.is_monochrome() {
                if dark {
                    10.0
                } else {
                    90.0
                }
            } else if dark {
                20.0
            } else {
                100.0
            }
        }
        TertiaryContainer => tertiary_container_tone(s),
        OnTertiaryContainer => on_tertiary_container_tone(s),
        Error => {
            if dark {
                80.0
            } else {
                40.0
            }
        }
        OnError => {
            if dark {
                20.0
            } else {
                100.0
            }
        }
        ErrorContainer => {
            if dark {
                30.0
            } else {
                90.0
            }
        }
        OnErrorContainer => {
            if dark {
                90.0
            } else {
                10.0
            }
        }
        PrimaryFixed => {
            if s.is_monochrome() {
                40.0
            } else {
                90.0
            }
        }
        PrimaryFixedDim => {
            if s.is_monochrome() {
                30.0
            } else {
                80.0
            }
        }
        OnPrimaryFixed => {
            if s.is_monochrome() {
                100.0
            } else {
                10.0
            }
        }
        OnPrimaryFixedVariant => {
            if s.is_monochrome() {
                90.0
            } else {
                30.0
            }
        }
        SecondaryFixed => {
            if s.is_monochrome() {
                80.0
            } else {
                90.0
            }
        }
        SecondaryFixedDim => {
            if s.is_monochrome() {
                70.0
            } else {
                80.0
            }
        }
        OnSecondaryFixed => 10.0,
        OnSecondaryFixedVariant => {
            if s.is_monochrome() {
                25.0
            } else {
                30.0
            }
        }
        TertiaryFixed => {
            if s.is_monochrome() {
                40.0
            } else {
                90.0
            }
        }
        TertiaryFixedDim => {
            if s.is_monochrome() {
                30.0
            } else {
                80.0
            }
        }
        OnTertiaryFixed => {
            if s.is_monochrome() {
                100.0
            } else {
                10.0
            }
        }
        OnTertiaryFixedVariant => {
            if s.is_monochrome() {
                90.0
            } else {
                30.0
            }
        }
    }
}

fn find_desired_chroma_by_tone(hue: f64, chroma: f64, tone: f64, by_decreasing_tone: bool) -> f64 {
    let mut answer = tone;

    let mut closest_to_chroma = Hct::from_hct(hue, chroma, tone);
    if closest_to_chroma.chroma < chroma {
        let mut chroma_peak = closest_to_chroma.chroma;
        while closest_to_chroma.chroma < chroma {
            answer += if by_decreasing_tone { -1.0 } else { 1.0 };
            let potential_solution = Hct::from_hct(hue, chroma, answer);
            if chroma_peak > potential_solution.chroma {
                break;
            }
            if (potential_solution.chroma - chroma).abs() < 0.4 {
                break;
            }

            let potential_delta = (potential_solution.chroma - chroma).abs();
            let current_delta = (closest_to_chroma.chroma - chroma).abs();
            if potential_delta < current_delta {
                closest_to_chroma = potential_solution;
            }
            chroma_peak = chroma_peak.max(potential_solution.chroma);
        }
    }

    answer
}

fn find_desired_tone_for_sec(s: &DynamicScheme) -> f64 {
    let initial_tone = if s.is_dark { 30.0 } else { 90.0 };
    if s.is_monochrome() {
        return if s.is_dark { 30.0 } else { 85.0 };
    }
    if s.is_fidelity() {
        return initial_tone;
    }
    find_desired_chroma_by_tone(
        s.secondary_palette.hue,
        s.secondary_palette.chroma,
        initial_tone,
        !s.is_dark,
    )
}

fn tertiary_container_tone(s: &DynamicScheme) -> f64 {
    if !s.is_monochrome() {
        return if s.is_dark { 60.0 } else { 49.0 };
    }
    if !s.is_fidelity() {
        return if s.is_dark { 30.0 } else { 90.0 };
    }
    let proposed_hct = s.tertiary_palette.get_hct(s.source_hct.tone);
    DislikeAnalyzer::fix_if_disliked(proposed_hct).tone
}

fn on_tertiary_container_tone(s: &DynamicScheme) -> f64 {
    if !s.is_monochrome() {
        return if s.is_dark { 0.0 } else { 100.0 };
    }
    if !s.is_fidelity() {
        return if s.is_dark { 90.0 } else { 10.0 };
    }
    foreground_tone(tone_raw(Role::TertiaryContainer, s), 4.5)
}

fn tone_prefers_light_foreground(tone: f64) -> bool {
    crate::color::py_round(tone) < 60.0
}

fn foreground_tone(bg_tone: f64, ratio: f64) -> f64 {
    let lighter_tone = Contrast::lighter_unsafe(bg_tone, ratio);
    let darker_tone = Contrast::darker_unsafe(bg_tone, ratio);
    let lighter_ratio = Contrast::ratio_of_tones(lighter_tone, bg_tone);
    let darker_ratio = Contrast::ratio_of_tones(darker_tone, bg_tone);
    let prefer_lighter = tone_prefers_light_foreground(bg_tone);

    if prefer_lighter {
        let negligible_difference = (lighter_ratio - darker_ratio).abs() < 0.1
            && lighter_ratio < ratio
            && darker_ratio < ratio;
        if lighter_ratio >= ratio || lighter_ratio >= darker_ratio || negligible_difference {
            lighter_tone
        } else {
            darker_tone
        }
    } else if darker_ratio >= ratio || darker_ratio >= lighter_ratio {
        darker_tone
    } else {
        lighter_tone
    }
}

fn get_tone(role: Role, s: &DynamicScheme) -> f64 {
    let color = spec(role, s);
    let decreasing_contrast = s.contrast_level < 0.0;

    if let Some(tdp) = &color.tone_delta_pair {
        let bg = color.background.expect("tone delta has a background");
        let bg_tone = get_tone(bg, s);

        let a_is_nearer = match tdp.polarity {
            Polarity::Nearer => true,
            Polarity::Lighter => !s.is_dark,
            Polarity::Darker => s.is_dark,
        };
        let (nearer, farther) = if a_is_nearer {
            (tdp.role_a, tdp.role_b)
        } else {
            (tdp.role_b, tdp.role_a)
        };
        let am_nearer = role == nearer;
        let expansion_dir = if s.is_dark { 1.0 } else { -1.0 };

        let n_contrast = spec(nearer, s)
            .contrast_curve
            .unwrap()
            .get(s.contrast_level);
        let f_contrast = spec(farther, s)
            .contrast_curve
            .unwrap()
            .get(s.contrast_level);

        let n_initial_tone = tone_raw(nearer, s);
        let mut n_tone = if Contrast::ratio_of_tones(bg_tone, n_initial_tone) >= n_contrast {
            n_initial_tone
        } else {
            foreground_tone(bg_tone, n_contrast)
        };

        let f_initial_tone = tone_raw(farther, s);
        let mut f_tone = if Contrast::ratio_of_tones(bg_tone, f_initial_tone) >= f_contrast {
            f_initial_tone
        } else {
            foreground_tone(bg_tone, f_contrast)
        };

        if decreasing_contrast {
            n_tone = foreground_tone(bg_tone, n_contrast);
            f_tone = foreground_tone(bg_tone, f_contrast);
        }

        if (f_tone - n_tone) * expansion_dir >= tdp.delta {
            // already far enough apart
        } else {
            f_tone = if (f_tone - n_tone) * expansion_dir >= tdp.delta {
                clamp_double(0.0, 100.0, n_tone + tdp.delta * expansion_dir)
            } else {
                clamp_double(0.0, 100.0, f_tone - tdp.delta * expansion_dir)
            };
        }

        if (50.0..60.0).contains(&n_tone) {
            if expansion_dir > 0.0 {
                let new_f = f_tone.max(n_tone + tdp.delta * expansion_dir);
                n_tone = 60.0;
                f_tone = new_f;
            } else {
                let new_f = f_tone.min(n_tone + tdp.delta * expansion_dir);
                n_tone = 49.0;
                f_tone = new_f;
            }
        } else if (50.0..60.0).contains(&f_tone) {
            if tdp.stay_together {
                if expansion_dir > 0.0 {
                    let new_f = f_tone.max(n_tone + tdp.delta * expansion_dir);
                    n_tone = 60.0;
                    f_tone = new_f;
                } else {
                    let new_f = f_tone.min(n_tone + tdp.delta * expansion_dir);
                    n_tone = 49.0;
                    f_tone = new_f;
                }
            } else if expansion_dir > 0.0 {
                f_tone = 60.0;
            } else {
                f_tone = 49.0;
            }
        }

        return if am_nearer { n_tone } else { f_tone };
    }

    let answer = tone_raw(role, s);
    let Some(bg_role) = color.background else {
        return answer;
    };
    let bg_tone = get_tone(bg_role, s);
    let desired_ratio = color.contrast_curve.unwrap().get(s.contrast_level);

    let mut answer = if Contrast::ratio_of_tones(bg_tone, answer) >= desired_ratio {
        answer
    } else {
        foreground_tone(bg_tone, desired_ratio)
    };

    if decreasing_contrast {
        answer = foreground_tone(bg_tone, desired_ratio);
    }

    // Python's FromPaletteOptions defaults is_background to the (truthy) bool
    // class, so every role takes this branch, not only containers.
    if (50.0..60.0).contains(&answer) {
        answer = if Contrast::ratio_of_tones(49.0, bg_tone) >= desired_ratio {
            49.0
        } else {
            60.0
        };
    }

    if let Some(bg2_role) = color.second_background {
        let bg_tone2 = get_tone(bg2_role, s);
        let upper = bg_tone.max(bg_tone2);
        let lower = bg_tone.min(bg_tone2);

        if Contrast::ratio_of_tones(upper, answer) >= desired_ratio
            && Contrast::ratio_of_tones(lower, answer) >= desired_ratio
        {
            return answer;
        }

        let light_option = Contrast::lighter(upper, desired_ratio);
        let dark_option = Contrast::darker(lower, desired_ratio);

        let prefers_light =
            tone_prefers_light_foreground(bg_tone) || tone_prefers_light_foreground(bg_tone2);
        return if prefers_light && (light_option == -1.0 || dark_option == -1.0) {
            light_option
        } else {
            dark_option
        };
    }

    answer
}

pub fn get_argb(role: Role, s: &DynamicScheme) -> u32 {
    let tone = get_tone(role, s);
    s.tone(spec(role, s).palette, tone)
}

pub fn success_colors(dark: bool) -> [(&'static str, &'static str); 4] {
    if dark {
        [
            ("success", "#B5CCBA"),
            ("onSuccess", "#213528"),
            ("successContainer", "#374B3E"),
            ("onSuccessContainer", "#D1E9D6"),
        ]
    } else {
        [
            ("success", "#4F6354"),
            ("onSuccess", "#FFFFFF"),
            ("successContainer", "#D1E8D5"),
            ("onSuccessContainer", "#0C1F13"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_is_unique() {
        let mut seen = Vec::new();
        for (_, role) in ROLES {
            assert!(!seen.contains(&role), "duplicate role {role:?}");
            seen.push(role);
        }
        assert_eq!(seen.len(), 54);
    }

    #[test]
    fn neutral_scheme_key_tones_are_finite() {
        let hct = Hct::from_int(0xFF336699);
        let scheme = DynamicScheme::for_scheme("scheme-vibrant", hct, true, 0.0).unwrap();
        for (_, role) in ROLES {
            let argb = get_argb(role, &scheme);
            assert_eq!(argb >> 24, 255);
        }
    }
}
