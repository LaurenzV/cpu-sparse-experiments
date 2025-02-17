// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(feature = "simd"), forbid(unsafe_code))]

mod dispatcher;
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

#[derive(Copy, Clone, Debug)]
/// The execution mode used for the rendering process.
pub enum ExecutionMode {
    /// Only use scalar execution. This is recommended if you want to have
    /// consistent results across different platforms and want to avoid unsafe code,
    /// and is the only option if you disabled the `simd` feature. Performance will be
    /// worse, though.
    Scalar,
    /// Select the best execution mode according to what is available on the host system.
    /// This is the recommended option for highest performance.
    #[cfg(feature = "simd")]
    Auto,
    /// Force the usage of neon SIMD instructions. This will lead to panics in case
    /// the CPU doesn't support neon.
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    Neon,
}

pub use pixmap::Pixmap;
pub use render::RenderContext;
