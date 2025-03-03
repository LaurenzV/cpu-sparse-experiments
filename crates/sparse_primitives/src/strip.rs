// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_common::execute::{Avx2, Scalar};
use vello_common::peniko::Fill;
use vello_common::strip::Strip;
use vello_common::tile::Tiles;

pub trait Render {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: Fill,
    );
}

impl Render for Scalar {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: Fill,
    ) {
        vello_common::strip::render(tiles, strip_buf, alpha_buf, fill_rule);
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl Render for Avx2 {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: Fill,
    ) {
        vello_common::strip::render(tiles, strip_buf, alpha_buf, fill_rule);
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
impl Render for crate::execute::Neon {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: Fill,
    ) {
        unsafe {
            neon::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
        }
    }
}
