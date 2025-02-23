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
            peniko::Compose::PlusLighter => fill::plus_lighter(target, cs),
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
            peniko::Compose::DestAtop => strip::dest_atop(target, cs, alphas),
            peniko::Compose::Xor => strip::xor(target, cs, alphas),
            peniko::Compose::Plus => strip::plus(target, cs, alphas),
            peniko::Compose::PlusLighter => strip::plus_lighter(target, cs, alphas),
        }
    }
}

pub(crate) mod fill {
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

    pub(crate) fn clear(target: &mut [u8], _: &[u8; COLOR_COMPONENTS]) {
        target.fill(0);
    }

    pub(crate) fn dest(_: &mut [u8], _: &[u8; COLOR_COMPONENTS]) {}

    pub(crate) fn copy(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS]) {
        for cb in target.chunks_exact_mut(COLOR_COMPONENTS) {
            cb.copy_from_slice(cs);
        }
    }

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

    pub(crate) fn plus_lighter(_: &mut [u8], _: &[u8; COLOR_COMPONENTS]) {
        unimplemented!()
    }
}

pub(crate) mod strip {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;
    use crate::wide_tile::STRIP_HEIGHT;

    macro_rules! compose_strip {
        (
            name: $n:ident,
            fa: $fa:expr,
            fb: $fb:expr
        ) => {
            pub(crate) fn $n(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], alphas: &[u32]) {
                let _as = cs[3] as u16;

                for (cb, masks) in target.chunks_exact_mut(TOTAL_STRIP_HEIGHT).zip(alphas) {
                    for (j, cb) in cb.chunks_exact_mut(COLOR_COMPONENTS).enumerate() {
                        let am = ((*masks >> (j * 8)) & 0xff) as u16;

                        for i in 0..COLOR_COMPONENTS {
                            let _ab = cb[3] as u16;

                            let res = div_255(cs[i] as u16 * $fa(_as, _ab))
                                + div_255(cb[i] as u16 * $fb(_as, _ab));
                            cb[i] =
                                div_255(res * am) as u8 + div_255(cb[i] as u16 * (255 - am)) as u8;
                        }
                    }
                }
            }
        };
    }

    // Since this is the most common operation, we use a custom implementation which should
    // be more efficient than the macro above.
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

    pub(crate) fn dest(_: &mut [u8], _: &[u8; COLOR_COMPONENTS], _: &[u32]) {}

    compose_strip!(
        name: clear,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| 0
    );

    compose_strip!(
        name: copy,
        fa: |_as, _ab| 255,
        fb: |_as, _ab| 0
    );

    compose_strip!(
        name: dest_over,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 255
    );

    compose_strip!(
        name: src_in,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| 0
    );

    compose_strip!(
        name: dest_in,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| _as
    );

    compose_strip!(
        name: src_out,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 0
    );

    compose_strip!(
        name: dest_out,
        fa: |_as, _ab| 0,
        fb: |_as, _ab| 255 - _as
    );

    compose_strip!(
        name: src_atop,
        fa: |_as, _ab| _ab,
        fb: |_as, _ab| 255 - _as
    );

    compose_strip!(
        name: dest_atop,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab|  _as
    );

    compose_strip!(
        name: xor,
        fa: |_as, _ab| 255 - _ab,
        fb: |_as, _ab| 255 - _as
    );

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

    pub(crate) fn plus_lighter(_: &mut [u8], _: &[u8; COLOR_COMPONENTS], _: &[u32]) {
        unimplemented!()
    }
}
