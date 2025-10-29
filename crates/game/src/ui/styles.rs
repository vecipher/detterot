use bevy::prelude::Color;

/// Okabe-Ito colour palette expressed as RGBA tuples. Each tuple is
/// colour-blind-safe and converts to an `srgb` colour via [`Color::rgba`].
pub const OKABE_ITO_SKY_BLUE: (f32, f32, f32, f32) = (0.337, 0.705, 0.913, 1.0);
pub const OKABE_ITO_ORANGE: (f32, f32, f32, f32) = (0.902, 0.623, 0.0, 1.0);
pub const OKABE_ITO_BLUISH_GREEN: (f32, f32, f32, f32) = (0.0, 0.62, 0.451, 1.0);
pub const OKABE_ITO_VERMILLION: (f32, f32, f32, f32) = (0.835, 0.337, 0.004, 1.0);
pub const OKABE_ITO_REDDISH_PURPLE: (f32, f32, f32, f32) = (0.8, 0.475, 0.655, 1.0);
pub const OKABE_ITO_YELLOW: (f32, f32, f32, f32) = (0.941, 0.894, 0.259, 1.0);
pub const OKABE_ITO_LIGHT_GREY: (f32, f32, f32, f32) = (0.729, 0.729, 0.729, 1.0);
pub const SURFACE_DEEP_SLATE: (f32, f32, f32, f32) = (0.082, 0.094, 0.117, 1.0);
pub const SURFACE_SMOKE: (f32, f32, f32, f32) = (0.231, 0.258, 0.294, 1.0);
pub const TEXT_PRIMARY: (f32, f32, f32, f32) = (0.898, 0.902, 0.917, 1.0);
pub const TEXT_MUTED: (f32, f32, f32, f32) = (0.584, 0.6, 0.62, 1.0);

/// Converts a palette tuple into a [`Color`] in the sRGB colour space.
#[inline]
pub fn color(tuple: (f32, f32, f32, f32)) -> Color {
    Color::srgba(tuple.0, tuple.1, tuple.2, tuple.3)
}

/// Convenience constructors for frequently used palette entries.
#[inline]
pub fn positive() -> Color {
    color(OKABE_ITO_BLUISH_GREEN)
}

#[inline]
pub fn negative() -> Color {
    color(OKABE_ITO_VERMILLION)
}

#[inline]
pub fn neutral() -> Color {
    color(OKABE_ITO_LIGHT_GREY)
}

#[inline]
pub fn accent() -> Color {
    color(OKABE_ITO_ORANGE)
}

#[inline]
pub fn background() -> Color {
    color(SURFACE_DEEP_SLATE)
}

#[inline]
pub fn surface() -> Color {
    color(SURFACE_SMOKE)
}

#[inline]
pub fn text_primary() -> Color {
    color(TEXT_PRIMARY)
}

#[inline]
pub fn text_muted() -> Color {
    color(TEXT_MUTED)
}
