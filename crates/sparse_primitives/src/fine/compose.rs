use crate::fine::COLOR_COMPONENTS;

pub(crate) mod scalar {
    use crate::fine::{COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::{div_255, splat_x4};

    // All the formulas in the comments are with premultiplied alpha for Cs and Cb.

    /// Composite using `SrcOver`
    /// Cs + Cb * (1 – αs)
    pub(crate) fn src_over(
        target: &mut [u8],
        color: &[u8; COLOR_COMPONENTS],
    ) {
        let alpha = color[3];
        let cs = splat_x4(color);

        let inv_as = (255 - alpha) as u16;
        let dest = target.chunks_exact_mut(TOTAL_STRIP_HEIGHT);

        for cb in dest {
            for i in 0..TOTAL_STRIP_HEIGHT {
                cb[i] = cs[i] + div_255(cb[i] as u16 * inv_as) as u8;
            }
        }
    }
}