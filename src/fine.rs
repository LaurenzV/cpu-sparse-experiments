// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization

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
    pub(crate) scratch: [f32; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4],
}

impl<'a> Fine<'a> {
    pub(crate) fn new(width: usize, height: usize, out_buf: &'a mut [u8]) -> Self {
        let scratch = [0.0; WIDE_TILE_WIDTH * STRIP_HEIGHT * 4];
        Self {
            width,
            height,
            out_buf,
            scratch,
        }
    }

    pub(crate) fn clear_scalar(&mut self, color: [f32; 4]) {
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
                self.fill_scalar(f.x as usize, f.width as usize, f.color.components);
            }
            Cmd::Strip(s) => {
                let aslice = &alphas[s.alpha_ix..];
                self.strip_scalar(s.x as usize, s.width as usize, aslice, s.color.components);
            }
        }
    }

    pub(crate) fn fill_scalar(&mut self, x: usize, width: usize, color: [f32; 4]) {
        if color[3] == 1.0 {
            for z in
                self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width].chunks_exact_mut(4)
            {
                z.copy_from_slice(&color);
            }
        } else {
            let one_minus_alpha = 1.0 - color[3];
            for z in
                self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width].chunks_exact_mut(4)
            {
                for i in 0..4 {
                    //z[i] = color[i] + one_minus_alpha * z[i];
                    // Note: the mul_add will perform poorly on x86_64 default cpu target
                    // Probably right thing to do is craft a #cfg that detects fma, fcma, etc.
                    // What we really want is fmuladdf32 from intrinsics!
                    z[i] = z[i].mul_add(one_minus_alpha, color[i]);
                }
            }
        }
    }

    pub(crate) fn strip_scalar(&mut self, x: usize, width: usize, alphas: &[u32], color: [f32; 4]) {
        debug_assert!(alphas.len() >= width);
        let cs = color.map(|x| x * (1.0 / 255.0));
        for (z, a) in self.scratch[x * STRIP_HEIGHT_F32..][..STRIP_HEIGHT_F32 * width]
            .chunks_exact_mut(16)
            .zip(alphas)
        {
            for j in 0..4 {
                let mask_alpha = ((*a >> (j * 8)) & 0xff) as f32;
                let one_minus_alpha = 1.0 - mask_alpha * cs[3];
                for i in 0..4 {
                    z[j * 4 + i] = z[j * 4 + i].mul_add(one_minus_alpha, mask_alpha * cs[i]);
                }
            }
        }
    }
}

pub(crate) fn pack_scalar(
    out_buf: &mut [u8],
    scratch: &[f32],
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

            let mut rgba_f32 = [0.0; 4];
            rgba_f32.copy_from_slice(&scratch[(i * STRIP_HEIGHT + j) * 4..][..4]);
            let rgba_u8 = rgba_f32.map(|x| ((x * 255.0) + 0.5) as u8);
            out_buf[target_ix..][..4].copy_from_slice(&rgba_u8);
        }
    }
}
