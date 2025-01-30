// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod fine;
pub mod flatten;
pub mod pixmap;
pub mod render;
pub mod strip;
pub mod svg;
pub mod tiling;
pub mod wide_tile;
mod pattern;

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
pub use render::CsRenderCtx;
