use crate::color::{AlphaColor, PremulColor, Srgb};

pub(crate) trait ColorExt {
    /// Using the already-existing `to_rgba8` is slow on x86 because it involves rounding, so
    /// we use a fast method with just + 0.5.
    fn to_rgba8_fast(&self) -> [u8; 4];
}

impl ColorExt for PremulColor<Srgb> {
    fn to_rgba8_fast(&self) -> [u8; 4] {
        [
            (self.components[0] * 255.0 + 0.5) as u8,
            (self.components[1] * 255.0 + 0.5) as u8,
            (self.components[2] * 255.0 + 0.5) as u8,
            (self.components[3] * 255.0 + 0.5) as u8,
        ]
    }
}
