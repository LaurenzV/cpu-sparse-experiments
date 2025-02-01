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

    pub(crate) fn clear_scalar(&mut self, color: [u8; 4]) {
        for z in self.scratch.chunks_exact_mut(4) {
            z.copy_from_slice(&color);
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
                let color = c.to_rgba8().to_u8_array();

                if color[3] == 255 {
                    for z in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                        .chunks_exact_mut(4)
                    {
                        z.copy_from_slice(&color);
                    }
                } else {
                    let one_minus_alpha = 255 - color[3];
                    for z in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                        .chunks_exact_mut(4)
                    {
                        for i in 0..4 {
                            //z[i] = color[i] + one_minus_alpha * z[i];
                            // Note: the mul_add will perform poorly on x86_64 default cpu target
                            // Probably right thing to do is craft a #cfg that detects fma, fcma, etc.
                            // What we really want is fmuladdf32 from intrinsics!
                            z[i] = mul_255(z[i], one_minus_alpha).saturating_add(color[i]);
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
                let color = s.to_rgba8().to_u8_array();

                debug_assert!(alphas.len() >= width);
                for (z, a) in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
                    .chunks_exact_mut(16)
                    .zip(alphas)
                {
                    for j in 0..4 {
                        let mask_alpha = ((*a >> (j * 8)) & 0xff) as u8;
                        let one_minus_alpha = 255 - mul_255(mask_alpha, color[3]);
                        for i in 0..4 {
                            z[j * 4 + i] = mul_255(z[j * 4 + i], one_minus_alpha).saturating_add(mul_255(mask_alpha, color[i]));
                        }
                    }
                }
            }
            Paint::Pattern(_) => unimplemented!(),
        }
    }
}

#[inline(always)]
fn mul_255(val1: u8, val2: u8) -> u8 {
    ((val1 as u16 * val2 as u16) / 255) as u8
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

    for j in 0..STRIP_HEIGHT {
        let line_ix = base_ix + j * width * 4;

        // Continue if the current row is outside the range of the pixmap.
        if y * STRIP_HEIGHT + j >= height {
            break;
        }

        for i in 0..WIDE_TILE_WIDTH {
            // Abort if the current column is outside the range of the pixmap.
            if x * WIDE_TILE_WIDTH + i >= width {
                break;
            }

            let target_ix = line_ix + i * 4;

            out_buf[target_ix..][..4].copy_from_slice(&scratch[(i * STRIP_HEIGHT + j) * 4..][..4]);
        }
    }
}
