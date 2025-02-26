use crate::color::{PremulColor, Srgb};

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
    #[cfg(feature = "simd")]
    use crate::fine::COLOR_COMPONENTS;

    #[inline(always)]
    pub(crate) const fn div_255(val: u16) -> u16 {
        (val + 1 + (val >> 8)) >> 8
    }

    #[inline(always)]
    #[cfg(feature = "simd")]
    pub(crate) fn splat_x4(val: &[u8; COLOR_COMPONENTS]) -> [u8; 4 * COLOR_COMPONENTS] {
        let mut buf = [0; 4 * COLOR_COMPONENTS];

        for i in 0..4 {
            buf[i * COLOR_COMPONENTS..((i + 1) * COLOR_COMPONENTS)].copy_from_slice(val);
        }

        buf
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::fine::COLOR_COMPONENTS;
    use crate::util::scalar::splat_x4;
    use std::arch::x86_64::{
        __m128i, __m256i, _mm256_add_epi16, _mm256_cvtepu8_epi16, _mm256_set1_epi16,
        _mm256_srli_epi16, _mm_loadu_si128,
    };

    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn div_255(val: __m256i) -> __m256i {
        _mm256_srli_epi16::<8>(_mm256_add_epi16(
            _mm256_add_epi16(val, _mm256_set1_epi16(1)),
            _mm256_srli_epi16::<8>(val),
        ))
    }

    /// Splat from 4x u8 to 16x u16.
    ///
    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn splat_x8(val: &[u8; COLOR_COMPONENTS]) -> __m256i {
        // TODO: Do this using only SIMD?
        let cs = splat_x4(val);

        _mm256_cvtepu8_epi16(_mm_loadu_si128(cs.as_ptr() as *const __m128i))
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use std::arch::aarch64::*;

    /// SAFETY: The CPU needs to support the target feature `neon`.
    #[inline]
    pub(crate) unsafe fn div_255(val: uint16x8_t) -> uint16x8_t {
        let val_shifted = vshrq_n_u16::<8>(val);
        let one = vdupq_n_u16(1);
        let added = vaddq_u16(val, one);
        let added = vaddq_u16(added, val_shifted);

        vshrq_n_u16::<8>(added)
    }

    /// SAFETY: The CPU needs to support the target feature `neon`.
    pub(crate) unsafe fn inv(val: uint8x8_t) -> uint8x8_t {
        vsub_u8(vdup_n_u8(255), val)
    }
}
