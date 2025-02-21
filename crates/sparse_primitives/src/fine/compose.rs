use crate::execute::Scalar;
use crate::fine::COLOR_COMPONENTS;

pub(crate) trait Compose {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose);
}

pub(crate) mod scalar {
    use crate::execute::Scalar;
    use crate::fine::compose::Compose;
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::{div_255, splat_x4};

    impl Compose for Scalar {
        fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            match compose {
                peniko::Compose::Copy => src_copy(target, cs),
                peniko::Compose::SrcOver => src_over(target, cs),
                peniko::Compose::DestOver => dest_over(target, cs),
                peniko::Compose::SrcAtop => src_atop(target, cs),
                peniko::Compose::DestOut => dest_out(target, cs),
                peniko::Compose::Xor => xor(target, cs),
                peniko::Compose::Plus => plus(target, cs),
                peniko::Compose::Dest => dest(target, cs),
                peniko::Compose::Clear => clear(target, cs),
                _ => unimplemented!(),
            }
        }
    }

    // All the formulas in the comments are with premultiplied alpha for Cs and Cb.

    /// Composite using `SrcOver` (Cs + Cb * (1 – αs)).
    pub(crate) fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let _as = cs[3];
        let cs = splat_x4(cs);

        let inv_as = (255 - _as) as u16;

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            for i in 0..TOTAL_STRIP_HEIGHT {
                cb[i] = cs[i] + div_255(cb[i] as u16 * inv_as) as u8;
            }
        }
    }

    /// Composite using `SrcCopy` (Cs).
    pub(crate) fn src_copy(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let cs = splat_x4(cs);

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            cb.copy_from_slice(&cs);
        }
    }

    /// Composite using `DestOver` (Cs * (1 – αb) + Cb).
    pub(crate) fn dest_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        for cb in target.chunks_exact_mut(4) {
            let inv_ab = (255 - cb[3]) as u16;

            for i in 0..COLOR_COMPONENTS {
                cb[i] = div_255(cs[i] as u16 * inv_ab) as u8 + cb[i];
            }
        }
    }

    /// Composite using `SrcAtop` (Cs * αb + Cb * (1 – αs)).
    pub(crate) fn src_atop(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        for cb in target.chunks_exact_mut(4) {
            let inv_as = (255 - cs[3]) as u16;

            for i in 0..COLOR_COMPONENTS {
                let ab = cb[3] as u16;
                let im1 = div_255(cs[i] as u16 * ab) as u8;
                let im2 = div_255(cb[i] as u16 * inv_as) as u8;

                cb[i] = im1 + im2;
            }
        }
    }

    /// Composite using `DestOut` (Cb * (1 - as)).
    pub(crate) fn dest_out(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let inv_as = 255 - cs[3] as u16;

        for cb in target.chunks_exact_mut(4) {
            for i in 0..COLOR_COMPONENTS {
                cb[i] = div_255(cb[i] as u16 * inv_as) as u8;
            }
        }
    }

    /// Composite using `Xor` (Cs * (1 - αb) + Cb * (1 - αs)).
    pub(crate) fn xor(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let inv_as = 255 - cs[3] as u16;

        for cb in target.chunks_exact_mut(4) {
            for i in 0..COLOR_COMPONENTS {
                let inv_ab = 255 - cb[3] as u16;
                let im1 = div_255(cs[i] as u16 * inv_ab) as u8;
                let im2 = div_255(cb[i] as u16 * inv_as) as u8;

                cb[i] = im1 + im2;
            }
        }
    }

    /// Composite using `Plus` (Cs + Cb).
    pub(crate) fn plus(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let cs = splat_x4(cs);

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            for i in 0..TOTAL_STRIP_HEIGHT {
                cb[i] = cs[i].saturating_add(cb[i]);
            }
        }
    }

    /// Composite using `Dest` (Cb).
    pub(crate) fn dest(_: &mut [u8], _: &[u8; COLOR_COMPONENTS]) {}

    /// Composite using `Clear` (0).
    pub(crate) fn clear(target: &mut [u8], _: &[u8; COLOR_COMPONENTS]) {
        target.fill(0);
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use crate::execute::{Neon, Scalar};
    use crate::fine::compose::Compose;
    use crate::fine::COLOR_COMPONENTS;

    impl Compose for Neon {
        fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            Scalar::compose_fill(target, cs, compose);
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::execute::{Avx2, Scalar};
    use crate::fine::compose::Compose;
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::avx2::{div_255, splat_x8};
    use std::arch::x86_64::*;

    impl Compose for Avx2 {
        fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            unsafe {
                match compose {
                    peniko::Compose::SrcOver => src_over(target, cs),
                    _ => Scalar::compose_fill(target, cs, compose),
                }
            }
        }
    }

    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        // TODO: This code can be improved by processing TOTAL_STRIP_HEIGHT * 2
        // elements at the time according to preliminary benchmarks

        let inv_as = _mm256_set1_epi16(255 - cs[3] as i16);
        let cs = splat_x8(cs);

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            let cb_vals = _mm256_cvtepu8_epi16(_mm_loadu_si128(cb.as_ptr() as *const __m128i));
            let im1 = _mm256_mullo_epi16(cb_vals, inv_as);
            let im2 = div_255(im1);
            let im3 = _mm256_add_epi16(cs, im2);
            let im4 = _mm_packus_epi16(
                _mm256_extracti128_si256::<0>(im3),
                _mm256_extracti128_si256::<1>(im3),
            );
            _mm_storeu_si128(cb.as_mut_ptr() as *mut __m128i, im4);
        }
    }
}
