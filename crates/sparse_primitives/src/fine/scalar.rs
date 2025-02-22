use crate::execute::Scalar;
use crate::fine;
use crate::fine::COLOR_COMPONENTS;

impl fine::Compose for Scalar {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose) {
        match compose {
            peniko::Compose::Clear => fill::clear(target, cs),
            peniko::Compose::Copy => fill::copy(target, cs),
            peniko::Compose::Dest => fill::dest(target, cs),
            peniko::Compose::SrcOver => fill::src_over(target, cs),
            peniko::Compose::DestOver => fill::dest_over(target, cs),
            peniko::Compose::SrcIn => fill::src_in(target, cs),
            peniko::Compose::DestIn => fill::dest_in(target, cs),
            peniko::Compose::SrcOut => fill::src_out(target, cs),
            peniko::Compose::DestOut => fill::dest_out(target, cs),
            peniko::Compose::SrcAtop => fill::src_atop(target, cs),
            peniko::Compose::DestAtop => fill::dest_atop(target, cs),
            peniko::Compose::Xor => fill::xor(target, cs),
            peniko::Compose::Plus => fill::plus(target, cs),
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
            peniko::Compose::Clear => strip::clear(target, cs, alphas),
            peniko::Compose::Copy => strip::copy(target, cs, alphas),
            peniko::Compose::Dest => strip::dest(target, cs, alphas),
            peniko::Compose::SrcOver => strip::src_over(target, cs, alphas),
            peniko::Compose::DestOver => strip::dest_over(target, cs, alphas),
            peniko::Compose::SrcIn => strip::src_in(target, cs, alphas),
            peniko::Compose::DestIn => strip::dest_in(target, cs, alphas),
            peniko::Compose::SrcOut => strip::src_out(target, cs, alphas),
            peniko::Compose::DestOut => strip::dest_out(target, cs, alphas),
            peniko::Compose::SrcAtop => strip::src_atop(target, cs, alphas),
            // peniko::Compose::DestAtop => strip::dest_atop(target, cs, alphas),
            peniko::Compose::Xor => strip::xor(target, cs, alphas),
            peniko::Compose::Plus => strip::plus(target, cs, alphas),
            _ => unimplemented!(),
        }
    }
}

mod fill {
    // See https://www.w3.org/TR/compositing-1/#porterduffcompositingoperators for the
    // formulas.

    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::{div_255, splat_x4};

    macro_rules! compose_fill {
        (
            name: $n:ident,
            fa: $fa:expr,
            fb: $fb:expr
        ) => {
            pub(crate) fn $n(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
                let _as = cs[3] as u16;

                for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
                    for cb in cb.chunks_exact_mut(COLOR_COMPONENTS) {
                        let _ab = cb[3] as u16;

                        for i in 0..COLOR_COMPONENTS {
                            cb[i] = div_255(cs[i] as u16 * $fa(_as, _ab)) as u8
                                + div_255(cb[i] as u16 * $fb(_as, _ab)) as u8;
                        }
                    }
                }
            }
        };
    }

    compose_fill!(
        name: clear,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| 0
    );

    compose_fill!(
        name: copy,
        fa: |_as, _ab| 255,
        fb: |_as, _ab| 0
    );

    compose_fill!(
        name: dest,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| 255
    );

    compose_fill!(
        name: src_over,
        fa: |_as, _ab| 255,
        fb: |_as, _ab| 255 - _as
    );

    compose_fill!(
        name: dest_over,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 255
    );

    compose_fill!(
        name: src_in,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| 0
    );

    compose_fill!(
        name: dest_in,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| _as
    );

    compose_fill!(
        name: src_out,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 0
    );

    compose_fill!(
        name: dest_out,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| 255 - _as
    );

    compose_fill!(
        name: src_atop,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| 255 - _as
    );

    compose_fill!(
        name: dest_atop,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab|  _as
    );

    compose_fill!(
        name: xor,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 255 - _as
    );

    /// Composite using `Plus` (Fa = 255, Fb = 255).
    // We can't use the macro here because the operation might overflow, and
    // using `saturating_add` in the macro comes at the expense of bad auto-vectorization.
    pub(crate) fn plus(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        let cs = splat_x4(cs);

        for cb in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT) {
            for i in 0..TOTAL_STRIP_HEIGHT {
                cb[i] = cs[i].saturating_add(cb[i]);
            }
        }
    }
}

mod strip {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;
    use crate::wide_tile::STRIP_HEIGHT;

    // All the formulas in the comments are with premultiplied alpha for Cs and Cb.
    // `am` stands for `alpha mask` (i.e. opacity of the pixel due to anti-aliasing).
    //
    // I am not exactly sure how the `am` should be incorporated in some formulas, so it's
    // possible there are mistakes. I used tiny-skia output as the main point of reference.

    /// Composite using `SrcCopy` (Cs * am) + (1 - am) * Cb.
    pub(crate) fn copy(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
        for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
            for j in 0..STRIP_HEIGHT {
                let am = ((*masks >> (j * 8)) & 0xff) as u16;
                let inv_am = 255 - am;
                let base_idx = j * COLOR_COMPONENTS;

                for i in 0..COLOR_COMPONENTS {
                    let idx = base_idx + i;
                    let im1 = div_255(cs[i] as u16 * am) as u8;
                    let im2 = div_255(inv_am * cb[idx] as u16) as u8;
                    cb[idx] = im1 + im2;
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

    /// Composite using `SrcIn` (Cs * ab * am) + (1 - am) * Cb.
    pub(crate) fn src_in(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
        for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
            for j in 0..STRIP_HEIGHT {
                let am = ((*masks >> (j * 8)) & 0xff) as u16;
                let inv_am = 255 - am;
                let base_idx = j * COLOR_COMPONENTS;

                for i in 0..COLOR_COMPONENTS {
                    let idx = base_idx + i;
                    let ab = cb[base_idx + 3] as u16;

                    let src_in = {
                        let im1 = div_255(am * ab);
                        div_255(cs[i] as u16 * im1) as u8
                    };

                    cb[idx] = src_in + div_255(inv_am * cb[idx] as u16) as u8;
                }
            }
        }
    }

    /// Composite using `DestIn` (Cb * as * am)  + (1 - am) * Cb.
    pub(crate) fn dest_in(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
        for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
            for j in 0..STRIP_HEIGHT {
                let am = ((*masks >> (j * 8)) & 0xff) as u16;
                let inv_am = 255 - am;
                let as_am = div_255(am * cs[3] as u16);
                let base_idx = j * COLOR_COMPONENTS;

                for i in 0..COLOR_COMPONENTS {
                    let idx = base_idx + i;
                    let dest_in = div_255(cb[idx] as u16 * as_am) as u8;

                    cb[idx] = dest_in + div_255(inv_am * cb[idx] as u16) as u8;
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

    /// Composite using `SrcOut` Cs * (1 – αb) * am + (1 - am) * Cb.
    pub(crate) fn src_out(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
        for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
            for j in 0..STRIP_HEIGHT {
                let base_idx = j * COLOR_COMPONENTS;
                let am = ((*masks >> (j * 8)) & 0xff) as u8;
                let inv_am = 255 - am as u16;

                for i in 0..COLOR_COMPONENTS {
                    let idx = base_idx + i;
                    let inv_ab = 255 - cb[base_idx + 3] as u16;
                    let src_out = div_255(cs[i] as u16 * div_255(inv_ab * am as u16)) as u8;

                    cb[idx] = src_out + div_255(inv_am * cb[idx] as u16) as u8;
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
