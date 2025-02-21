use crate::execute::Scalar;
use crate::fine;
use crate::fine::COLOR_COMPONENTS;

impl fine::Compose for Scalar {
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
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;
    use crate::wide_tile::STRIP_HEIGHT;

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
