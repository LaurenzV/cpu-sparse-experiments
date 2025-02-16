// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization

use crate::paint::Paint;
use crate::wide_tile::{Cmd, STRIP_HEIGHT, WIDE_TILE_WIDTH};

pub(crate) const STRIP_HEIGHT_F32: usize = STRIP_HEIGHT * 4;

pub(crate) struct Fine<'a> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    // rgba pixels
    pub(crate) out_buf: &'a mut [u8],
    // f32 RGBA pixels
    // That said, if we use u8, then this is basically a block of
    // untyped memory.
    pub(crate) scratch: [u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
    #[cfg(feature = "simd")]
    use_simd: bool,
}

impl<'a> Fine<'a> {
    pub(crate) fn new(
        width: usize,
        height: usize,
        out_buf: &'a mut [u8],
        #[cfg(feature = "simd")] use_simd: bool,
    ) -> Self {
        let scratch = [0; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4];
        Self {
            width,
            height,
            out_buf,
            scratch,
            #[cfg(feature = "simd")]
            use_simd,
        }
    }

    #[inline(never)]
    pub(crate) fn clear(&mut self, premul_color: [u8; 4]) {
        if premul_color[0] == premul_color[1]
            && premul_color[1] == premul_color[2]
            && premul_color[2] == premul_color[3]
        {
            // All components are the same, so we can use memset instead.
            self.scratch.fill(premul_color[0])
        } else {
            for z in self.scratch.chunks_exact_mut(4) {
                z.copy_from_slice(&premul_color);
            }
        }
    }

    #[inline(never)]
    pub(crate) fn pack(&mut self, x: usize, y: usize) {
        pack(
            self.out_buf,
            &self.scratch,
            self.width,
            self.height,
            x,
            y,
        );
    }

    pub(crate) fn run_cmd(&mut self, cmd: &Cmd, alphas: &[u32]) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(f.x as usize, f.width as usize, &f.paint);
            }
            Cmd::Strip(s) => {
                let aslice = &alphas[s.alpha_ix..];
                self.strip(s.x as usize, s.width as usize, aslice, &s.paint);
            }
        }
    }

    #[inline(never)]
    pub(crate) fn fill(&mut self, x: usize, width: usize, paint: &Paint) {
        #[cfg(feature = "simd")]
        if self.use_simd {
            #[cfg(target_arch = "aarch64")]
            if std::arch::is_aarch64_feature_detected!("neon") {
                // SAFETY: We ensured that the `neon` target feature is available.
                return unsafe { neon::fill_simd(&mut self.scratch, x, width, paint) };
            }
        }

        fill_scalar(&mut self.scratch, x, width, paint);
    }

    #[inline(never)]
    pub(crate) fn strip(&mut self, x: usize, width: usize, alphas: &[u32], paint: &Paint) {
        #[cfg(feature = "simd")]
        if self.use_simd {
            #[cfg(target_arch = "aarch64")]
            if std::arch::is_aarch64_feature_detected!("neon") {
                // SAFETY: We ensured that the `neon` target feature is available.
                return unsafe { neon::strip_simd(&mut self.scratch, x, width, alphas, paint) };
            }
        }

        strip_scalar(&mut self.scratch, x, width, alphas, paint);
    }
}

#[inline(always)]
fn div_255(val: u16) -> u16 {
    // For some reason, doing this instead of / 255 makes strip_scalar 3x faster on ARM.
    // TODO: Measure behavior on x86
    (val + 1 + (val >> 8)) >> 8
}

fn fill_scalar(
    scratch: &mut [u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
    x: usize,
    width: usize,
    paint: &Paint,
) {
    match paint {
        Paint::Solid(c) => {
            let (color_buf, alpha) = {
                let mut buf = [0; STRIP_HEIGHT_F32];
                let premul_color = c.premultiply().to_rgba8().to_u8_array();

                for i in 0..STRIP_HEIGHT {
                    buf[i * 4..((i + 1) * 4)].copy_from_slice(&premul_color);
                }

                (buf, buf[3])
            };

            let colors = scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                .chunks_exact_mut(STRIP_HEIGHT_F32);

            if alpha == 255 {
                for z in colors {
                    z.copy_from_slice(&color_buf);
                }
            } else {
                let inv_alpha = 255 - alpha as u16;
                for z in colors {
                    for i in 0..STRIP_HEIGHT_F32 {
                        z[i] = div_255(z[i] as u16 * inv_alpha) as u8 + color_buf[i];
                    }
                }
            }
        }
        Paint::Pattern(_) => unimplemented!(),
    }
}

fn strip_scalar(
    scratch: &mut [u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
    x: usize,
    width: usize,
    alphas: &[u32],
    paint: &Paint,
) {
    match paint {
        Paint::Solid(s) => {
            let color = s.premultiply().to_rgba8().to_u8_array();

            debug_assert!(alphas.len() >= width);
            for (z, a) in scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                .chunks_exact_mut(16)
                .zip(alphas)
            {
                for j in 0..4 {
                    let mask_alpha = ((*a >> (j * 8)) & 0xff) as u16;
                    let inv_alpha = 255 - (mask_alpha * color[3] as u16) / 255;
                    for i in 0..4 {
                        let im1 = z[j * 4 + i] as u16 * inv_alpha;
                        let im2 = mask_alpha * color[i] as u16;
                        let im3 = div_255(im1 + im2);
                        z[j * 4 + i] = im3 as u8;
                    }
                }
            }
        }
        Paint::Pattern(_) => unimplemented!(),
    }
}

fn pack(
    out_buf: &mut [u8],
    scratch: &[u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) {
    let base_ix = (y * STRIP_HEIGHT * width + x * WIDE_TILE_WIDTH) * 4;

    // Make sure we don't process rows outside the range of the pixmap.
    let max_height = (height - y * STRIP_HEIGHT).min(STRIP_HEIGHT);

    for j in 0..max_height {
        let line_ix = base_ix + j * width * 4;

        // Make sure we don't process columns outside the range of the pixmap.
        let max_width = (width - x * WIDE_TILE_WIDTH).min(WIDE_TILE_WIDTH);
        let target_len = max_width * 4;
        // This helps the compiler to understand that any access to `dest` cannot
        // be out of bounds, and thus saves corresponding checks in the for loop.
        let dest = &mut out_buf[line_ix..][..target_len];

        for i in 0..max_width {
            let src = &scratch[(i * STRIP_HEIGHT + j) * 4..][..4];
            dest[i * 4..][..4].copy_from_slice(&src[..4]);
        }
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
mod neon {
    use std::arch::aarch64::*;

    use crate::fine::STRIP_HEIGHT_F32;
    use crate::paint::Paint;
    use crate::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};

    /// SAFETY: Caller must ensure target feature `neon` is available.
    // Note: This method currently seems to be slower than the scalar version.
    pub(super) unsafe fn fill_simd(
        scratch: &mut [u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
        x: usize,
        width: usize,
        paint: &Paint,
    ) {
        use std::arch::aarch64::*;

        match paint {
            Paint::Solid(c) => {
                let (color_buf, alpha) = {
                    let mut buf = [0; STRIP_HEIGHT_F32];
                    let premul_color = c.premultiply().to_rgba8().to_u8_array();

                    for i in 0..STRIP_HEIGHT {
                        buf[i * 4..((i + 1) * 4)].copy_from_slice(&premul_color);
                    }

                    (buf, buf[3])
                };

                let color_buf_simd = vld1_u8(color_buf[0..].as_ptr());

                let mut strip_cols = scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                    .chunks_exact_mut(STRIP_HEIGHT_F32);

                if alpha == 255 {
                    for col in strip_cols {
                        col.copy_from_slice(&color_buf);
                    }
                } else {
                    let inv_alpha = vdupq_n_u16(255 - alpha as u16);

                    for z in strip_cols {
                        for i in 0..2 {
                            let index = i * 8;
                            let z_vals = vmovl_u8(vld1_u8(z.as_mut_ptr().add(index)));
                            let im_1 = vmulq_u16(z_vals, inv_alpha);
                            let im_2 = div_255(im_1);
                            let im_3 = vmovn_u16(im_2);
                            let im_4 = vadd_u8(im_3, color_buf_simd);
                            vst1_u8(z.as_mut_ptr().add(index), im_4);
                        }
                    }
                }
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }

    /// SAFETY: Caller must ensure target feature `neon` is available.
    #[inline(never)]
    pub(super) unsafe fn strip_simd(
        scratch: &mut [u8; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
        x: usize,
        width: usize,
        alphas: &[u32],
        paint: &Paint,
    ) {
        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8().to_u8_array();
                let color_alpha = vdupq_n_u16(color[3] as u16);
                let simd_color = {
                    let color = u32::from_le_bytes(color) as u64;
                    let color = color | (color << 32);
                    vmovl_u8(vcreate_u8(color))
                };

                let tff = vdupq_n_u16(255);

                debug_assert!(alphas.len() >= width);

                for (z, a) in scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                    .chunks_exact_mut(16)
                    .zip(alphas)
                {
                    for j in 0..2 {
                        let index = j * 8;

                        let mask_alpha = {
                            let first_mask = vdup_n_u16(((*a >> 2 * index) & 0xff) as u16);
                            let second_mask = vdup_n_u16(((*a >> (2 * index + 8)) & 0xff) as u16);
                            vcombine_u16(first_mask, second_mask)
                        };

                        let inv_alpha = {
                            let im1 = vmulq_u16(mask_alpha, color_alpha);
                            let im2 = div_255(im1);
                            vsubq_u16(tff, im2)
                        };

                        let im1 =
                            vmulq_u16(vmovl_u8(vld1_u8(z.as_mut_ptr().add(index))), inv_alpha);
                        let im2 = vmulq_u16(mask_alpha, simd_color);
                        let im3 = vmovn_u16(div_255(vaddq_u16(im1, im2)));
                        vst1_u8(z.as_mut_ptr().add(index), im3);
                    }
                }
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }

    unsafe fn div_255(val: uint16x8_t) -> uint16x8_t {
        let val_shifted = vshrq_n_u16::<8>(val);
        let one = vdupq_n_u16(1);
        let added = vaddq_u16(val, one);
        let added = vaddq_u16(added, val_shifted);

        vshrq_n_u16::<8>(added)
    }
}
