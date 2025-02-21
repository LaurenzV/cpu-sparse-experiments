// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fine rasterization

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2;
#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon;
pub(crate) mod scalar;

use crate::execute::KernelExecutor;
use crate::paint::Paint;
use crate::util::ColorExt;
use crate::wide_tile::{Cmd, STRIP_HEIGHT, WIDE_TILE_WIDTH};
use std::marker::PhantomData;

pub(crate) const COLOR_COMPONENTS: usize = 4;
pub(crate) const TOTAL_STRIP_HEIGHT: usize = STRIP_HEIGHT * COLOR_COMPONENTS;
pub(crate) const SCRATCH_BUF_SIZE: usize = WIDE_TILE_WIDTH * STRIP_HEIGHT * COLOR_COMPONENTS;

pub(crate) type ScratchBuf = [u8; SCRATCH_BUF_SIZE];

pub trait Compose {
    fn compose_fill(target: &mut [u8], cs: &[u8; COLOR_COMPONENTS], compose: peniko::Compose);
    fn compose_strip(
        target: &mut [u8],
        cs: &[u8; COLOR_COMPONENTS],
        alphas: &[u32],
        compose: peniko::Compose,
    );
}

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

    pub(crate) fn run_cmd(&mut self, cmd: &Cmd, alphas: &[u32], compose: peniko::Compose) {
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
    pub fn fill(&mut self, x: usize, width: usize, paint: &Paint, mut compose: peniko::Compose) {
        match paint {
            Paint::Solid(c) => {
                let color = c.premultiply().to_rgba8_fast();

                // If color is completely opaque with SrcOver, it's the same as filling using Copy.
                if compose == peniko::Compose::SrcOver && color[3] == 255 {
                    compose = peniko::Compose::Copy
                }

                let target =
                    &mut self.scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width];

                KE::compose_fill(target, &color, compose);
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
        compose: peniko::Compose,
    ) {
        debug_assert!(alphas.len() >= width);

        match paint {
            Paint::Solid(s) => {
                let color = s.premultiply().to_rgba8_fast();

                let target =
                    &mut self.scratch[x * TOTAL_STRIP_HEIGHT..][..TOTAL_STRIP_HEIGHT * width];

                KE::compose_strip(target, &color, alphas, compose);
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
