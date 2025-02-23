use crate::execute::{Avx2, Scalar};
use crate::fine;
use crate::fine::COLOR_COMPONENTS;

impl fine::Compose for Avx2 {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
        unsafe {
            match compose {
                peniko::Compose::SrcOver => fill::src_over(target, cs),
                _ => Scalar::compose_fill(target, cs, compose),
            }
        }
    }

    fn compose_strip(
        target: &mut [u8],
        cs: &[u8; COLOR_COMPONENTS],
        alphas: &[u32],
        compose: peniko::Compose,
    ) {
        Scalar::compose_strip(target, cs, alphas, compose);
    }
}

mod fill {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::avx2::{div_255, splat_x8};
    use std::arch::x86_64::*;

    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        // TODO: This code can be improved by processing TOTAL_STRIP_HEIGHT * 2
        // elements at the time according to preliminary benchmarks

        let stride = TOTAL_STRIP_HEIGHT * 2;
        let remainder = target.len() % stride;
        let (head, tail) = target.split_at_mut(target.len() - remainder);
        let inv_as = _mm256_set1_epi16(255 - cs[3] as i16);
        let cs = splat_x8(cs);

        for cb in head.chunks_exact_mut(stride) {
            let z_vals = _mm256_loadu_si256(cb.as_ptr() as *const __m256i);
            let lo = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<0>(z_vals));
            let hi = _mm256_cvtepu8_epi16(_mm256_extracti128_si256::<1>(z_vals));

            let mulled_lo = _mm256_mullo_epi16(lo, inv_as);
            let mulled_hi = _mm256_mullo_epi16(hi, inv_as);
            let dived_lo = div_255(mulled_lo);
            let dived_hi = div_255(mulled_hi);
            let added_lo = _mm256_add_epi16(cs, dived_lo);
            let added_hi = _mm256_add_epi16(cs, dived_hi);
            let packed = _mm256_packus_epi16(added_lo, added_hi);
            let switched = _mm256_permute4x64_epi64::<0b11011000>(packed);
            _mm256_storeu_si256(cb.as_mut_ptr() as *mut __m256i, switched);
        }

        // Remainder in case length is not a multiple of stride. This has been measured to be a bit
        // slower, but it's only used for the last strip at most. In the future, we should just
        // fallback to SSE instead of adding this special case.
        for cb in tail.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
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
