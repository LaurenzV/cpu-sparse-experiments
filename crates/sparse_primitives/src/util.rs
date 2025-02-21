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

pub(crate) mod scalar {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::wide_tile::STRIP_HEIGHT;

    #[inline(always)]
    pub(crate) fn div_255(val: u16) -> u16 {
        (val + 1 + (val >> 8)) >> 8
    }

    #[inline(always)]
    pub(crate) fn splat_x4(val: &[u8; COLOR_COMPONENTS]) -> [u8; 4 * COLOR_COMPONENTS] {
        let mut buf = [0; 4 * COLOR_COMPONENTS];

        for i in 0..4 {
            buf[i * COLOR_COMPONENTS..((i + 1) * COLOR_COMPONENTS)].copy_from_slice(val);
        }

        buf
    }
}