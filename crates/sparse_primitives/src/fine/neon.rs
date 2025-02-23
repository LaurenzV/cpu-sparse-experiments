use crate::execute::{Neon, Scalar};
use crate::fine;
use crate::fine::{scalar, COLOR_COMPONENTS};

impl fine::Compose for Neon {
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
}

mod strip {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::neon::div_255;
    use std::arch::aarch64::*;

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
}
