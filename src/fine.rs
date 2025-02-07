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
}

impl<'a> Fine<'a> {
    pub(crate) fn new(width: usize, height: usize, out_buf: &'a mut [u8]) -> Self {
        let scratch = [0; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4];
        Self {
            width,
            height,
            out_buf,
            scratch,
        }
    }

    pub(crate) fn clear_scalar(&mut self, premul_color: [u8; 4]) {
        for z in self.scratch.chunks_exact_mut(4) {
            z.copy_from_slice(&premul_color);
        }
    }

    pub(crate) fn pack_scalar(&mut self, x: usize, y: usize) {
        pack_scalar(
            &mut self.out_buf,
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
                self.fill_scalar(f.x as usize, f.width as usize, &f.paint);
            }
            Cmd::Strip(s) => {
                let aslice = &alphas[s.alpha_ix..];
                self.strip_scalar(s.x as usize, s.width as usize, aslice, &s.paint);
            }
        }
    }

    pub(crate) fn fill_scalar(&mut self, x: usize, width: usize, paint: &Paint) {
        match paint {
            Paint::Solid(c) => {
                let premul_color = c.premultiply().to_rgba8().to_u8_array();

                if premul_color[3] == 255 {
                    for z in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                        .chunks_exact_mut(4)
                    {
                        z.copy_from_slice(&premul_color);
                    }
                } else {
                    let inv_alpha = 255 - premul_color[3] as u16;
                    for z in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                        .chunks_exact_mut(4)
                    {
                        for i in 0..4 {
                            z[i] = div_255(z[i] as u16 * inv_alpha) as u8 + premul_color[i];
                        }
                    }
                }
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }

    pub(crate) fn strip_scalar(&mut self, x: usize, width: usize, alphas: &[u32], paint: &Paint) {
        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8().to_u8_array();

                debug_assert!(alphas.len() >= width);
                for (z, a) in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
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
}

#[inline(always)]
fn div_255(val: u16) -> u16 {
    // For some reason, doing this instead of / 255 makes strip_scalar 3x faster on ARM.
    // TODO: Measure behavior on x86
    (val + 1 + (val >> 8)) >> 8
}

pub(crate) fn pack_scalar(
    out_buf: &mut [u8],
    scratch: &[u8],
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

        for i in 0..max_width {
            let target_ix = line_ix + i * 4;

            out_buf[target_ix..][..4].copy_from_slice(&scratch[(i * STRIP_HEIGHT + j) * 4..][..4]);
        }
    }
}
