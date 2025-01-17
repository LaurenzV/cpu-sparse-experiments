use crate::fine::{Fine, STRIP_HEIGHT_F32};
use std::arch::aarch64::*;

// Just a sanity check, the below implementations assume this size so there are
// no out of bounds accesses!
const _: () = assert!(STRIP_HEIGHT_F32 == 16);

impl<'a> Fine<'a> {
    pub unsafe fn fill_simd(&mut self, x: usize, width: usize, color: [f32; 4]) {
        let v_color = vld1q_f32(color.as_ptr());
        let alpha = color[3];
        if alpha == 1.0 {
            let v_color_4 = float32x4x4_t(v_color, v_color, v_color, v_color);
            for i in x..x + width {
                vst1q_f32_x4(self.scratch.as_mut_ptr().add(i * 16), v_color_4);
            }
        } else {
            let one_minus_alpha = vdupq_n_f32(1.0 - alpha);

            for z in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                .chunks_exact_mut(16)
            {
                let mut v = vld1q_f32_x4(z.as_ptr());
                v.0 = vfmaq_f32(v_color, v.0, one_minus_alpha);
                v.1 = vfmaq_f32(v_color, v.1, one_minus_alpha);
                v.2 = vfmaq_f32(v_color, v.2, one_minus_alpha);
                v.3 = vfmaq_f32(v_color, v.3, one_minus_alpha);
                vst1q_f32_x4(z.as_mut_ptr(), v);
            }
        }
    }

    pub unsafe fn strip_simd(&mut self, x: usize, width: usize, alphas: &[u32], color: [f32; 4]) {
        debug_assert!(alphas.len() >= width);
        let v_color = vmulq_f32(vld1q_f32(color.as_ptr()), vdupq_n_f32(1.0 / 255.0));
        for i in 0..width {
            let a = *alphas.get_unchecked(i);
            // all this zipping compiles to tbl, we should probably just write that
            let a1 = vreinterpret_u8_u32(vdup_n_u32(a));
            let a2 = vreinterpret_u16_u8(vzip1_u8(a1, vdup_n_u8(0)));
            let a3 = vcombine_u16(a2, vdup_n_u16(0));
            let a4 = vreinterpretq_u32_u16(vzip1q_u16(a3, vdupq_n_u16(0)));
            let alpha = vcvtq_f32_u32(a4);
            let ix = (x + i) * 16;
            let mut v = vld1q_f32_x4(self.scratch.as_ptr().add(ix));
            let one_minus_alpha = vfmsq_laneq_f32(vdupq_n_f32(1.0), alpha, v_color, 3);
            v.0 = vfmaq_laneq_f32(vmulq_laneq_f32(v_color, alpha, 0), v.0, one_minus_alpha, 0);
            v.1 = vfmaq_laneq_f32(vmulq_laneq_f32(v_color, alpha, 1), v.1, one_minus_alpha, 1);
            v.2 = vfmaq_laneq_f32(vmulq_laneq_f32(v_color, alpha, 2), v.2, one_minus_alpha, 2);
            v.3 = vfmaq_laneq_f32(vmulq_laneq_f32(v_color, alpha, 3), v.3, one_minus_alpha, 3);
            vst1q_f32_x4(self.scratch.as_mut_ptr().add(ix), v);
        }
    }
}
