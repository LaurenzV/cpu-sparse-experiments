// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization

pub(crate) mod compose;

use crate::execute::KernelExecutor;
use crate::paint::Paint;
use crate::util::ColorExt;
use crate::wide_tile::{Cmd, STRIP_HEIGHT, WIDE_TILE_WIDTH};
use peniko::Compose;
use std::marker::PhantomData;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TOTAL_STRIP_HEIGHT: usize = STRIP_HEIGHT * COLOR_COMPONENTS;
pub(crate) const SCRATCH_BUF_SIZE: usize = WIDE_TILE_WIDTH * STRIP_HEIGHT * COLOR_COMPONENTS;

pub(crate) type ScratchBuf = [u8; SCRATCH_BUF_SIZE];

pub struct Fine<'a, T: KernelExecutor> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) out_buf: &'a mut [u8],
    pub(crate) scratch: ScratchBuf,
    phantom_data: PhantomData<T>,
}

impl<'a, KE: KernelExecutor> Fine<'a, KE> {
    pub fn new(width: usize, height: usize, out_buf: &'a mut [u8]) -> Self {
        let scratch = [0; SCRATCH_BUF_SIZE];

        Self {
            width,
            height,
            out_buf,
            scratch,
            phantom_data: PhantomData::default(),
        }
    }

    #[inline(never)]
    pub fn clear(&mut self, premul_color: [u8; 4]) {
        if premul_color[0] == premul_color[1]
            && premul_color[1] == premul_color[2]
            && premul_color[2] == premul_color[3]
        {
            // All components are the same, so we can use memset instead.
            self.scratch.fill(premul_color[0])
        } else {
            for z in self.scratch.chunks_exact_mut(COLOR_COMPONENTS) {
                z.copy_from_slice(&premul_color);
            }
        }
    }

    #[inline(never)]
    pub(crate) fn pack(&mut self, x: usize, y: usize) {
        pack(self.out_buf, &self.scratch, self.width, self.height, x, y);
    }

    pub(crate) fn run_cmd(&mut self, cmd: &Cmd, alphas: &[u32], compose: Compose) {
        match cmd {
            Cmd::Fill(f) => {
                self.fill(f.x as usize, f.width as usize, &f.paint, compose);
            }
            Cmd::Strip(s) => {
                let aslice = &alphas[s.alpha_ix..];
                self.strip(s.x as usize, s.width as usize, aslice, &s.paint, compose);
            }
        }
    }

    #[inline(never)]
    pub fn fill(&mut self, x: usize, width: usize, paint: &Paint, mut compose: Compose) {
        match paint {
            Paint::Solid(c) => {
                let color = c.premultiply().to_rgba8_fast();

                // If color is completely opaque with SrcOver, it's the same as filling using Copy.
                if compose == Compose::SrcOver && color[3] == 255 {
                    compose = Compose::Copy
                }

                let target =
                    &mut self.scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width];

                KE::compose(target, &color, compose);
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }

    #[inline(never)]
    pub(crate) fn strip(
        &mut self,
        x: usize,
        width: usize,
        alphas: &[u32],
        paint: &Paint,
        compose: Compose,
    ) {
        debug_assert!(alphas.len() >= width);

        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8_fast();
                KE::strip_solid(&mut self.scratch, &color, x, width, alphas, compose);
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }
}

fn pack(out_buf: &mut [u8], scratch: &ScratchBuf, width: usize, height: usize, x: usize, y: usize) {
    let base_ix = (y * STRIP_HEIGHT * width + x * WIDE_TILE_WIDTH) * COLOR_COMPONENTS;

    // Make sure we don't process rows outside the range of the pixmap.
    let max_height = (height - y * STRIP_HEIGHT).min(STRIP_HEIGHT);

    for j in 0..max_height {
        let line_ix = base_ix + j * width * COLOR_COMPONENTS;

        // Make sure we don't process columns outside the range of the pixmap.
        let max_width = (width - x * WIDE_TILE_WIDTH).min(WIDE_TILE_WIDTH);
        let target_len = max_width * COLOR_COMPONENTS;
        // This helps the compiler to understand that any access to `dest` cannot
        // be out of bounds, and thus saves corresponding checks in the for loop.
        let dest = &mut out_buf[line_ix..][..target_len];

        for i in 0..max_width {
            let src = &scratch[(i * STRIP_HEIGHT + j) * COLOR_COMPONENTS..][..COLOR_COMPONENTS];
            dest[i * COLOR_COMPONENTS..][..COLOR_COMPONENTS]
                .copy_from_slice(&src[..COLOR_COMPONENTS]);
        }
    }
}

pub(crate) mod scalar {
    use crate::fine::compose::scalar::{
        clear, dest, dest_out, dest_over, plus, src_atop, src_copy, src_over, xor,
    };
    use crate::fine::{ScratchBuf, COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::util::scalar::div_255;
    use crate::wide_tile::STRIP_HEIGHT;
    use peniko::Compose;

    pub(crate) fn strip_solid(
        scratch: &mut ScratchBuf,
        cs: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
        compose: Compose,
    ) {
        // All the formulas in the comments are with premultiplied alpha for Cs and Cb.
        // `am` stands for `alpha mask` (i.e. opacity of the pixel due to anti-aliasing).
        match compose {
            // Cs * am
            Compose::Copy => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cs * am + Cb * (1 – αs * am)
            Compose::SrcOver => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cs * am * (1 – αb) + Cb
            Compose::DestOver => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cs * αb * am + Cb * (1 – αs * am)
            Compose::SrcAtop => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cs * am * (1 - αb) + Cb * (1 - αs * am)
            Compose::Xor => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cb * (1 - as * am)
            Compose::DestOut => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cs * am + Cb
            Compose::Plus => {
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
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
            // Cb
            Compose::Dest => {}
            // 0 (but actually Cb * (1 - am))
            Compose::Clear => {
                // Similar to `Copy`, we can't just set all values to 0, since not all pixels in
                // the strip are actually covered by the shape.
                for (cb, masks) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
                    .chunks_exact_mut(TOTAL_STRIP_HEIGHT)
                    .zip(alphas)
                {
                    for j in 0..STRIP_HEIGHT {
                        let inv_am = 255 - ((*masks >> (j * 8)) & 0xff) as u16;
                        let idx = j * COLOR_COMPONENTS;

                        for i in 0..COLOR_COMPONENTS {
                            cb[idx + i] = div_255(cb[idx + i] as u16 * inv_am) as u8;
                        }
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use std::arch::aarch64::*;

    use crate::fine::{ScratchBuf, COLOR_COMPONENTS, TOTAL_STRIP_HEIGHT};
    use crate::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};

    pub(crate) unsafe fn fill_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
    ) {
        let (color_buf, alpha) = {
            let mut buf = [0; TOTAL_STRIP_HEIGHT];

            for i in 0..STRIP_HEIGHT {
                buf[i * COLOR_COMPONENTS..((i + 1) * COLOR_COMPONENTS)].copy_from_slice(color);
            }

            (buf, buf[3])
        };

        let color_buf_simd = vld1q_u8(color_buf[0..].as_ptr());

        let mut strip_cols = scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
            .chunks_exact_mut(TOTAL_STRIP_HEIGHT);

        if alpha == 255 {
            for col in strip_cols {
                col.copy_from_slice(&color_buf);
            }
        } else {
            let inv_alpha = vdupq_n_u8(255 - alpha);

            for z in strip_cols {
                let z_vals = vld1q_u8(z.as_mut_ptr());
                let mut low = vmull_u8(vget_low_u8(z_vals), vget_low_u8(inv_alpha));
                let mut high = vmull_high_u8(z_vals, inv_alpha);
                high = vshrq_n_u16::<8>(high);
                low = vsraq_n_u16::<8>(low, low);
                high = vmlal_high_u8(high, z_vals, inv_alpha);
                let low = vaddhn_u16(low, vdupq_n_u16(1));
                let res = vaddq_u8(vaddhn_high_u16(low, high, vdupq_n_u16(1)), color_buf_simd);
                vst1q_u8(z.as_mut_ptr(), res);
            }
        }
    }

    pub(crate) unsafe fn strip_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
    ) {
        let color_alpha = vdupq_n_u16(color[3] as u16);

        let simd_color = {
            let color = u32::from_le_bytes(*color) as u64;
            let color = color | (color << 32);
            vmovl_u8(vcreate_u8(color))
        };

        let tff = vdupq_n_u16(255);

        for (z, a) in scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width]
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

                let im1 = vmulq_u16(vmovl_u8(vld1_u8(z.as_mut_ptr().add(index))), inv_alpha);
                let im2 = vmulq_u16(mask_alpha, simd_color);
                let im3 = vmovn_u16(div_255(vaddq_u16(im1, im2)));
                vst1_u8(z.as_mut_ptr().add(index), im3);
            }
        }
    }

    #[inline]
    unsafe fn div_255(val: uint16x8_t) -> uint16x8_t {
        let val_shifted = vshrq_n_u16::<8>(val);
        let one = vdupq_n_u16(1);
        let added = vaddq_u16(val, one);
        let added = vaddq_u16(added, val_shifted);

        vshrq_n_u16::<8>(added)
    }
}
