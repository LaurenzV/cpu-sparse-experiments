// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SIMD speedups

use crate::{
    fine::Fine,
    strip::{Strip, Tile},
};

pub fn render_strips(tiles: &[Tile], strip_buf: &mut Vec<Strip>, alpha_buf: &mut Vec<u32>) {
    crate::strip::render_strips_scalar(tiles, strip_buf, alpha_buf);
}

// This block is the fallback, no SIMD
impl<'a> Fine<'a> {
    pub(crate) fn pack(&mut self, x: usize, y: usize) {
        self.pack_scalar(x, y);
    }

    pub(crate) fn clear(&mut self, color: [f32; 4]) {
        self.clear_scalar(color);
    }

    pub(crate) fn fill(&mut self, x: usize, y: usize, color: [f32; 4]) {
        self.fill_scalar(x, y, color);
    }

    pub(crate) fn strip(&mut self, x: usize, width: usize, alphas: &[u32], color: [f32; 4]) {
        self.strip_scalar(x, width, alphas, color);
    }
}
