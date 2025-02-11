// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(feature = "simd"), forbid(unsafe_code))]

pub mod fine;
pub mod flatten;
pub mod paint;
pub mod pattern;
pub mod pixmap;
mod rect;
pub mod render;
pub mod strip;
pub mod svg;
pub mod tiling;
pub mod wide_tile;

#[derive(Clone, Copy)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl FillRule {
    pub(crate) fn active_fill(&self, winding: i32) -> bool {
        match self {
            FillRule::NonZero => winding != 0,
            FillRule::EvenOdd => winding % 2 != 0,
        }
    }
}

pub use pixmap::Pixmap;
pub use render::RenderContext;
