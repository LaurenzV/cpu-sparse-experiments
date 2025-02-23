use crate::execute::{Neon, Scalar};
use crate::fine;
use crate::fine::{scalar, COLOR_COMPONENTS};

impl fine::Compose for Neon {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
        unsafe {
            match compose {
                peniko::Compose::SrcOver => fill::src_over(target, cs),
                peniko::Compose::SrcOut => fill::src_out(target, cs),
                peniko::Compose::DestOver => fill::dest_over(target, cs),
                peniko::Compose::SrcIn => fill::src_in(target, cs),
                peniko::Compose::DestIn => fill::dest_in(target, cs),
                peniko::Compose::DestOut => fill::dest_out(target, cs),
                peniko::Compose::SrcAtop => fill::src_atop(target, cs),
                peniko::Compose::DestAtop => fill::dest_atop(target, cs),
                peniko::Compose::Xor => fill::xor(target, cs),

                // For those, we just fall back to scalar, either because no improvement is possible
                // or we just haven't implemented it yet.
                peniko::Compose::Clear => scalar::fill::clear(target, cs),
                peniko::Compose::Copy => scalar::fill::copy(target, cs),
                peniko::Compose::Plus => scalar::fill::plus(target, cs),
                peniko::Compose::Dest => scalar::fill::dest(target, cs),
                peniko::Compose::PlusLighter => scalar::fill::plus_lighter(target, cs),
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
                peniko::Compose::SrcOut => strip::src_out(target, cs, alphas),
                peniko::Compose::DestOver => strip::dest_over(target, cs, alphas),
                peniko::Compose::SrcIn => strip::src_in(target, cs, alphas),
                peniko::Compose::DestIn => strip::dest_in(target, cs, alphas),
                peniko::Compose::DestOut => strip::dest_out(target, cs, alphas),
                peniko::Compose::SrcAtop => strip::src_atop(target, cs, alphas),
                peniko::Compose::DestAtop => strip::dest_atop(target, cs, alphas),
                peniko::Compose::Xor => strip::xor(target, cs, alphas),

                // For those, we just fall back to scalar, either because no improvement is possible
                // or we just haven't implemented it yet.
                peniko::Compose::Clear => scalar::strip::clear(target, cs, alphas),
                peniko::Compose::Copy => scalar::strip::copy(target, cs, alphas),
                peniko::Compose::Plus => scalar::strip::plus(target, cs, alphas),
                peniko::Compose::Dest => scalar::strip::dest(target, cs, alphas),
                peniko::Compose::PlusLighter => scalar::strip::plus_lighter(target, cs, alphas),
            }
        }
    }
}

mod fill {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::{splat_x2, splat_x4};

    use crate::util::neon::{div_255, inv};
    use std::arch::aarch64::*;

    macro_rules! compose_fill {
        (
            name: $n:ident,
            fa: $fa:expr,
            fb: $fb:expr
        ) => {
            /// SAFETY: The CPU needs to support the target feature `neon`.
            pub(crate) unsafe fn $n(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
                let _cs = vld1_u8(splat_x2(cs).as_ptr());
                let _as = vdup_n_u8(cs[3]);

                for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
                    for i in 0..2 {
                        let idx = i * 8;
                        let _ab = {
                            let v0 = vdup_n_u8(cb[idx + 3]);
                            let v1 = vdup_n_u8(cb[idx + 7]);

                            vext_u8::<4>(v0, v1)
                        };
                        let _cb = vld1_u8(cb.as_ptr().add(idx));

                        let im_1 = div_255(vmull_u8(_cs, $fa(_as, _ab)));
                        let im_2 = div_255(vmull_u8(_cb, $fb(_as, _ab)));
                        let res = vmovn_u16(vaddq_u16(im_1, im_2));

                        vst1_u8(cb.as_mut_ptr().add(idx), res);
                    }
                }
            }
        };
    }

    /// SAFETY: The CPU needs to support the target feature `neon`.
    pub(crate) unsafe fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let inv_as = vdupq_n_u8(255 - cs[3]);
        let cs = vld1q_u8(splat_x4(cs).as_ptr());

        let ones = vdupq_n_u16(1);

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            let cb_vals = vld1q_u8(cb.as_ptr());

            let high_im1 = vmull_high_u8(cb_vals, inv_as);
            let low_1 = vget_low_u8(cb_vals);
            let low_2 = vget_low_u8(inv_as);
            let low_im1 = vmull_u8(low_1, low_2);
            let high_im2 = vshrq_n_u16::<8>(high_im1);
            let low_im2 = vsraq_n_u16::<8>(low_im1, low_im1);
            let res_high = vmlal_high_u8(high_im2, cb_vals, inv_as);
            let res_low = vaddhn_u16(low_im2, ones);

            let im4 = vaddhn_high_u16(res_low, res_high, ones);
            let res = vaddq_u8(im4, cs);

            vst1q_u8(cb.as_mut_ptr(), res);
        }
    }

    compose_fill!(
        name: src_out,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| vdup_n_u8(0)
    );

    compose_fill!(
        name: dest_over,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| vdup_n_u8(255)
    );

    compose_fill!(
        name: src_in,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| vdup_n_u8(0)
    );

    compose_fill!(
        name: dest_in,
        fa: |_as, _ab| vdup_n_u8(0),
        fb: |_as, _ab| _as
    );

    compose_fill!(
        name: dest_out,
        fa: |_as, _ab| vdup_n_u8(0),
        fb: |_as, _ab| inv(_as)
    );

    compose_fill!(
        name: src_atop,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| inv(_as)
    );

    compose_fill!(
        name: dest_atop,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| _as
    );

    compose_fill!(
        name: xor,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| inv(_as)
    );
}

mod strip {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::neon::{div_255, inv};
    use crate::util::scalar::splat_x2;
    use std::arch::aarch64::*;

    // Note that this doesn't produce optimally-performing kernels yet, but for now
    // having one macro is much easier than duplicating code.
    // Speedup in most cases is only around 1.5x.
    macro_rules! compose_strip {
        (
            name: $n:ident,
            fa: $fa:expr,
            fb: $fb:expr
        ) => {
            /// SAFETY: The CPU needs to support the target feature `neon`.
            pub(crate) unsafe fn $n(
                target: &mut [u8],
                cs: &[u8; COLOR_COMPONENTS],
                alphas: &[u32],
            ) {
                let _cs = vld1_u8(splat_x2(cs).as_ptr());
                let _as = vdup_n_u8(cs[3]);

                for (cb, a) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                    for i in 0..2 {
                        let idx = i * 8;

                        let am = {
                            let first = vdup_n_u16(((*a >> 2 * idx) & 0xff) as u16);
                            let second = vdup_n_u16(((*a >> (2 * idx + 8)) & 0xff) as u16);
                            vcombine_u16(first, second)
                        };
                        let inv_am = vsubq_u16(vdupq_n_u16(255), am);

                        let _ab = {
                            let v0 = vdup_n_u8(cb[idx + 3]);
                            let v1 = vdup_n_u8(cb[idx + 7]);

                            vext_u8::<4>(v0, v1)
                        };
                        let _cb = vld1_u8(cb.as_ptr().add(idx));

                        let im_1 = div_255(vmull_u8(_cs, $fa(_as, _ab)));
                        let im_2 = div_255(vmull_u8(_cb, $fb(_as, _ab)));
                        let res = vaddq_u16(im_1, im_2);
                        let final_res = vaddq_u16(
                            div_255(vmulq_u16(am, res)),
                            div_255(vmulq_u16(inv_am, vmovl_u8(_cb))),
                        );

                        vst1_u8(cb.as_mut_ptr().add(idx), vmovn_u16(final_res));
                    }
                }
            }
        };
    }

    // Since this is the default and most common operation, we do include this as a custom-written
    // kernel.
    /// SAFETY: The CPU needs to support the target feature `neon`.
    pub(crate) unsafe fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
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

    compose_strip!(
        name: src_out,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| vdup_n_u8(0)
    );

    compose_strip!(
        name: dest_over,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| vdup_n_u8(255)
    );

    compose_strip!(
        name: src_in,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| vdup_n_u8(0)
    );

    compose_strip!(
        name: dest_in,
        fa: |_as, _ab| vdup_n_u8(0),
        fb: |_as, _ab| _as
    );

    compose_strip!(
        name: dest_out,
        fa: |_as, _ab| vdup_n_u8(0),
        fb: |_as, _ab| inv(_as)
    );

    compose_strip!(
        name: src_atop,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| inv(_as)
    );

    compose_strip!(
        name: dest_atop,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| _as
    );

    compose_strip!(
        name: xor,
        fa: |_as, _ab| inv(_ab),
        fb: |_as, _ab| inv(_as)
    );
}
