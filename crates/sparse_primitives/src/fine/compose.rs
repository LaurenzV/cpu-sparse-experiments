use crate::execute::Scalar;
use crate::fine::{ScratchBuf, COLOR_COMPONENTS};

pub(crate) trait Compose {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose);
    fn compose_strip(
        target: &mut [u8],
        cs: &[u8; COLOR_COMPONENTS],
        alphas: &[u32],
        compose: peniko::Compose,
    );
}

pub(crate) mod scalar {
    use crate::execute::Scalar;
    use crate::fine::compose::Compose;
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};

    impl Compose for Scalar {
        fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            match compose {
                peniko::Compose::Copy => fill::src_copy(target, cs),
                peniko::Compose::SrcOver => fill::src_over(target, cs),
                peniko::Compose::DestOver => fill::dest_over(target, cs),
                peniko::Compose::SrcAtop => fill::src_atop(target, cs),
                peniko::Compose::DestOut => fill::dest_out(target, cs),
                peniko::Compose::Xor => fill::xor(target, cs),
                peniko::Compose::Plus => fill::plus(target, cs),
                peniko::Compose::Dest => fill::dest(target, cs),
                peniko::Compose::Clear => fill::clear(target, cs),
                _ => unimplemented!(),
            }
        }

        fn compose_strip(
            target: &mut [u8],
            cs: &[u8; COLOR_COMPONENTS],
            alphas: &[u32],
            compose: peniko::Compose,
        ) {
            match compose {
                peniko::Compose::Copy => strip::src_copy(target, cs, alphas),
                peniko::Compose::SrcOver => strip::src_over(target, cs, alphas),
                peniko::Compose::DestOver => strip::dest_over(target, cs, alphas),
                peniko::Compose::SrcAtop => strip::src_atop(target, cs, alphas),
                peniko::Compose::DestOut => strip::dest_out(target, cs, alphas),
                peniko::Compose::Xor => strip::xor(target, cs, alphas),
                peniko::Compose::Plus => strip::plus(target, cs, alphas),
                peniko::Compose::Dest => strip::dest(target, cs, alphas),
                peniko::Compose::Clear => strip::clear(target, cs, alphas),
                _ => unimplemented!(),
            }
        }
    }

    mod fill {
        // All the formulas in the comments are with premultiplied alpha for Cs and Cb.

        use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
        use crate::util::scalar::{div_255, splat_x4};

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

    mod strip {
        use crate::fine::{ScratchBuf, COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
        use crate::util::scalar::div_255;
        use crate::wide_tile::STRIP_HEIGHT;
        use peniko::Compose;

        // All the formulas in the comments are with premultiplied alpha for Cs and Cb.
        // `am` stands for `alpha mask` (i.e. opacity of the pixel due to anti-aliasing).

        /// Composite using `SrcCopy` (Cs * am).
        pub(crate) fn src_copy(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    // This one unfortunately needs a bit of a hacky solution. The problem
                    // is that strips are always of a size of 4, meaning that if we always
                    // copy Cs * am, we might override parts of the destination that are within
                    // the strip, but are actually not covered by the shape anymore. Because of
                    // this, we do source-over compositing for pixel with mask alpha < 255,
                    // while we do copy for all mask alphas = 255. It's a bit expensive, but not
                    // sure if there is a better way.
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let do_src_copy = (am == 255) as u8;
                    let do_src_over = 1 - do_src_copy;
                    let inv_as_am = 255 - div_255(am * cs[3] as u16);

                    for i in 0..COLOR_COMPONENTS {
                        let im1 = cb[j * 4 + i] as u16 * inv_as_am;
                        let im2 = cs[i] as u16 * am;
                        let src_over = div_255(im1 + im2) as u8;
                        let src_copy = div_255(cs[i] as u16 * am) as u8;
                        cb[j * 4 + i] = do_src_over * src_over + do_src_copy * src_copy;
                    }
                }
            }
        }

        /// Composite using `SrcOver` (Cs * am + Cb * (1 – αs * am)).
        pub(crate) fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let inv_as_am = 255 - div_255(am * cs[3] as u16);

                    for i in 0..COLOR_COMPONENTS {
                        let im1 = cb[j * 4 + i] as u16 * inv_as_am;
                        let im2 = cs[i] as u16 * am;
                        let im3 = div_255(im1 + im2);
                        cb[j * 4 + i] = im3 as u8;
                    }
                }
            }
        }

        /// Composite using `DestOver` (Cs * am * (1 – αb) + Cb).
        pub(crate) fn dest_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    for i in 0..COLOR_COMPONENTS {
                        let idx = j * COLOR_COMPONENTS;
                        let inv_ab = (255 - cb[idx + 3]) as u16;
                        let im1 = div_255(am * inv_ab);
                        let im2 = div_255(cs[i] as u16 * im1) as u8;
                        cb[idx + i] += im2;
                    }
                }
            }
        }

        /// Composite using `SrcAtop` (Cs * αb * am + Cb * (1 – αs * am)).
        pub(crate) fn src_atop(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let inv_as_am = 255 - div_255(cs[3] as u16 * am);

                    for i in 0..COLOR_COMPONENTS {
                        let idx = j * COLOR_COMPONENTS;
                        let ab = cb[idx + 3] as u16;
                        let im1 = div_255(cs[i] as u16 * div_255(ab * am)) as u8;
                        let im2 = div_255(cb[idx + i] as u16 * inv_as_am) as u8;
                        cb[idx + i] = im1 + im2;
                    }
                }
            }
        }

        /// Composite using `Xor` (Cs * am * (1 - αb) + Cb * (1 - αs * am)).
        pub(crate) fn xor(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let idx = j * 4;

                    for i in 0..COLOR_COMPONENTS {
                        let cs_am = div_255(cs[i] as u16 * am);
                        let inv_as_am = 255 - div_255(cs[3] as u16 * am);
                        let inv_ab = (255 - cb[idx + 3]) as u16;

                        let im1 = div_255(cs_am * inv_ab) as u8;
                        let im2 = div_255(cb[idx + i] as u16 * inv_as_am) as u8;
                        cb[idx + i] = im1 + im2;
                    }
                }
            }
        }

        /// Composite using `DestOut` (Cb * (1 - as * am)).
        pub(crate) fn dest_out(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let idx = j * 4;

                    for i in 0..COLOR_COMPONENTS {
                        let inv_as_am = 255 - div_255(cs[3] as u16 * am);
                        cb[idx + i] = div_255(cb[idx + i] as u16 * inv_as_am) as u8;
                    }
                }
            }
        }

        /// Composite using `Plus` (Cs * am + Cb).
        pub(crate) fn plus(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let am = ((*masks >> (j * 8)) & 0xff) as u16;
                    let idx = j * 4;

                    for i in 0..COLOR_COMPONENTS {
                        let cs_am = div_255(cs[i] as u16 * am) as u8;
                        cb[idx + i] = cs_am.saturating_add(cb[idx + i]);
                    }
                }
            }
        }

        /// Composite using `Dest` (Cb).
        pub(crate) fn dest(_: &mut [u8], _: &[u8; COLOR_COMPONENTS], _: &[u32]) {}

        /// Composite using `Clear` (0 (but actually Cb * (1 - am))).
        pub(crate) fn clear(target: &mut [u8], _: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
            // Similar to `Copy`, we can't just set all values to 0, since not all pixels in
            // the strip are actually covered by the shape.
            for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                for j in 0..STRIP_HEIGHT {
                    let inv_am = 255 - ((*masks >> (j * 8)) & 0xff) as u16;
                    let idx = j * COLOR_COMPONENTS;

                    for i in 0..COLOR_COMPONENTS {
                        cb[idx + i] = div_255(cb[idx + i] as u16 * inv_am) as u8;
                    }
                }
            }
        }
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use crate::execute::{Neon, Scalar};
    use crate::fine::compose::Compose;
    use crate::fine::COLOR_COMPONENTS;

    impl Compose for Neon {
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
            unsafe {
                match compose {
                    peniko::Compose::SrcOver => strip::src_over(target, cs, alphas),
                    _ => Scalar::compose_strip(target, cs, alphas, compose),
                }
            }
        }
    }

    mod fill {
        use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
        use crate::util::scalar::splat_x4;

        use std::arch::aarch64::*;

        pub(crate) unsafe fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
            let inv_as = vdupq_n_u8(255 - cs[3]);
            let cs = vld1q_u8(splat_x4(cs).as_ptr());

            for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
                let cb_vals = vld1q_u8(cb.as_ptr());
                let res_low = {
                    let im1 = vmull_u8(vget_low_u8(cb_vals), vget_low_u8(inv_as));
                    let im2 = vsraq_n_u16::<8>(im1, im1);
                    vaddhn_u16(im2, vdupq_n_u16(1))
                };

                let res_high = {
                    let im1 = vmull_high_u8(cb_vals, inv_as);
                    let im2 = vshrq_n_u16::<8>(im1);
                    vmlal_high_u8(im2, cb_vals, inv_as)
                };

                let res = vaddq_u8(vaddhn_high_u16(res_low, res_high, vdupq_n_u16(1)), cs);
                vst1q_u8(cb.as_mut_ptr(), res);
            }
        }
    }

    mod strip {
        use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
        use crate::util::neon::div_255;
        use std::arch::aarch64::*;

        pub(crate) unsafe fn src_over(
            target: &mut [u8],
            cs: &[u8; COLOR_COMPONENTS],
            alphas: &[u32],
        ) {
            let _as = vdupq_n_u16(cs[3] as u16);

            let cs = {
                let color = u32::from_le_bytes(*cs) as u64;
                let color = color | (color << 32);
                vmovl_u8(vcreate_u8(color))
            };

            for (cb, a) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                // TODO: Unroll
                for j in 0..2 {
                    let index = j * 8;

                    let am = {
                        let first = vdup_n_u16(((*a >> 2 * index) & 0xff) as u16);
                        let second = vdup_n_u16(((*a >> (2 * index + 8)) & 0xff) as u16);
                        vcombine_u16(first, second)
                    };

                    let inv_alpha = vsubq_u16(vdupq_n_u16(255), div_255(vmulq_u16(am, _as)));

                    let im1 = vmulq_u16(vmovl_u8(vld1_u8(cb.as_mut_ptr().add(index))), inv_alpha);
                    let im2 = vmulq_u16(am, cs);
                    let res = vmovn_u16(div_255(vaddq_u16(im1, im2)));
                    vst1_u8(cb.as_mut_ptr().add(index), res);
                }
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::execute::{Avx2, Scalar};
    use crate::fine::compose::Compose;
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};

    impl Compose for Avx2 {
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
        use std::arch::x86_64::{
            __m128i, _mm256_add_epi16, _mm256_cvtepu8_epi16, _mm256_extracti128_si256,
            _mm256_mullo_epi16, _mm256_set1_epi16, _mm_loadu_si128, _mm_packus_epi16,
            _mm_storeu_si128,
        };

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
}
