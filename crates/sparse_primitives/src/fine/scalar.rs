use crate::fine;
use crate::fine::COLOR_COMPONENTS;
use vello_common::execute::Scalar;
use vello_common::peniko;

impl fine::Compose for Scalar {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
        match compose {
            peniko::Compose::SrcOver => fill::src_over(target, cs),
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
            peniko::Compose::SrcOver => strip::src_over(target, cs, alphas),
            _ => unimplemented!(),
        }
    }
}

pub(crate) mod fill {
    // See https://www.w3.org/TR/compositing-1/#porterduffcompositingoperators for the
    // formulas.

    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;

    pub(crate) fn src_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let _as = cs[3] as u16;

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            for cb in cb.chunks_exact_mut(COLOR_COMPONENTS) {
                let _ab = cb[3] as u16;

                for i in 0..COLOR_COMPONENTS {
                    cb[i] = cs[i] + div_255(cb[i] as u16 * (255 - _as)) as u8;
                }
            }
        }
    }
}

pub(crate) mod strip {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;
    use vello_common::strip::STRIP_HEIGHT;

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
}
