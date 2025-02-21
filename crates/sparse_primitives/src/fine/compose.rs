use crate::execute::Scalar;
use crate::fine::COLOR_COMPONENTS;

pub(crate) trait Compose {
    fn compose(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose);
}

pub(crate) mod scalar {
    use crate::execute::Scalar;
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::fine::compose::Compose;
    use crate::util::scalar::{div_255, splat_x4};

    impl Compose for Scalar {
        fn compose(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
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
        let alpha = cs[3];
        let cs = splat_x4(cs);

        let inv_as = (255 - alpha) as u16;
        let dest = target.chunks_exact_mut(TOTAL_STRIP_HEIGHT);

        for cb in dest {
            for i in 0..TOTAL_STRIP_HEIGHT {
                cb[i] = cs[i] + div_255(cb[i] as u16 * inv_as) as u8;
            }
        }
    }

    /// Composite using `SrcCopy` (Cs).
    pub(crate) fn src_copy(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let dest = target.chunks_exact_mut(TOTAL_STRIP_HEIGHT);
        let cs = splat_x4(cs);

        for cb in dest {
            cb.copy_from_slice(&cs);
        }
    }

    /// Composite using `DestOver` (Cs * (1 – αb) + Cb).
    pub(crate) fn dest_over(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let dest = target.chunks_exact_mut(4);

        for cb in dest {
            let inv_ab = (255 - cb[3]) as u16;

            for i in 0..COLOR_COMPONENTS {
                cb[i] = div_255(cs[i] as u16 * inv_ab) as u8 + cb[i];
            }
        }
    }

    /// Composite using `SrcAtop` (Cs * αb + Cb * (1 – αs)).
    pub(crate) fn src_atop(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let dest = target.chunks_exact_mut(4);

        for cb in dest {
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
        let dest = target.chunks_exact_mut(4);
        let inv_as = 255 - cs[3] as u16;

        for cb in dest {
            for i in 0..COLOR_COMPONENTS {
                cb[i] = div_255(cb[i] as u16 * inv_as) as u8;
            }
        }
    }

    /// Composite using `Xor` (Cs * (1 - αb) + Cb * (1 - αs)).
    pub(crate) fn xor(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let dest = target.chunks_exact_mut(4);
        let inv_as = 255 - cs[3] as u16;

        for cb in dest {
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
        let dest = target.chunks_exact_mut(TOTAL_STRIP_HEIGHT);
        let cs = splat_x4(cs);

        for cb in dest {
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
    use crate::fine::COLOR_COMPONENTS;
    use crate::fine::compose::Compose;

    impl Compose for Neon {
        fn compose(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            Scalar::compose(target, cs, compose);
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::execute::{Avx2, Scalar};
    use crate::fine::COLOR_COMPONENTS;
    use crate::fine::compose::Compose;

    impl Compose for Avx2 {
        fn compose(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
            Scalar::compose(target, cs, compose);
        }
    }
}
