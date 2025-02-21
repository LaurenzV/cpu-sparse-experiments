// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg_attr(not(feature = "simd"), forbid(unsafe_code))]

pub mod execute;
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

pub use peniko::*;

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

enum InnerContextType {
    Scalar(InnerContext<Scalar>),
    #[cfg(all(feature = "simd", target_arch = "aarch64"))]
    Neon(InnerContext<execute::Neon>),
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    Avx2(InnerContext<execute::Avx2>),
}

macro_rules! dispatch_mut {
    (
        func: $scalar:ident($($args:tt)*),
        $self:expr
    ) => {
        match &mut $self.0 {
            InnerContextType::Scalar(s) => s.$scalar($($args)*),
            #[cfg(all(feature = "simd", target_arch = "aarch64"))]
            InnerContextType::Neon(n) => n.$scalar($($args)*),
            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            InnerContextType::Avx2(n) => n.$scalar($($args)*)
        }
    };
}

macro_rules! dispatch {
    (
        func: $scalar:ident($($args:tt)*),
        $self:expr
    ) => {
        match &$self.0 {
            InnerContextType::Scalar(s) => s.$scalar($($args)*),
             #[cfg(all(feature = "simd", target_arch = "aarch64"))]
            InnerContextType::Neon(n) => n.$scalar($($args)*),
            #[cfg(all(feature = "simd", target_arch = "x86_64"))]
            InnerContextType::Avx2(n) => n.$scalar($($args)*),
        }
    };
}

pub struct RenderContext(InnerContextType);

impl RenderContext {
    /// Create a new render context.
    pub fn new(width: usize, height: usize) -> Self {
        let inner = select_inner_context(width, height, ExecutionMode::default());

        Self(inner)
    }

    /// Create a new render context with a specific execution mode.
    ///
    /// Panics when attempting to choose an execution mode not supported by the
    /// current CPU.
    pub fn new_with_execution_mode(
        width: usize,
        height: usize,
        execution_mode: ExecutionMode,
    ) -> Self {
        let inner = select_inner_context(width, height, execution_mode);

        Self(inner)
    }

    /// Fill a rectangle.
    pub fn fill_rect(&mut self, rect: &Rect) {
        dispatch_mut!(func: fill_rect(rect), self);
    }

    /// Stroke a rectangle.
    pub fn stroke_rect(&mut self, rect: &Rect) {
        dispatch_mut!(func: stroke_rect(rect), self);
    }

    /// Fill a path.
    pub fn fill_path(&mut self, path: &BezPath) {
        dispatch_mut!(func: fill_path(path), self);
    }

    /// Stroke a path.
    pub fn stroke_path(&mut self, path: &BezPath) {
        dispatch_mut!(func: stroke_path(path), self)
    }

    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        dispatch_mut!(func: set_blend_mode(blend_mode), self)
    }

    /// Set the stroking properties for stroking operations.
    pub fn set_stroke(&mut self, stroke: Stroke) {
        dispatch_mut!(func: set_stroke(stroke), self)
    }

    /// Set the paint for filling and stroking operations.
    pub fn set_paint(&mut self, paint: Paint) {
        dispatch_mut!(func: set_paint(paint), self)
    }

    /// Set the fill rule for filling operations.
    pub fn set_fill_rule(&mut self, fill_rule: FillRule) {
        dispatch_mut!(func: set_fill_rule(fill_rule), self)
    }

    /// Pre-concatenate a transform to the current transformation matrix.
    pub fn pre_concat_transform(&mut self, transform: Affine) {
        dispatch_mut!(func: pre_concat_transform(transform), self)
    }

    /// Post-concatenate a new transform to the current transformation matrix.
    pub fn post_concat_transform(&mut self, transform: Affine) {
        dispatch_mut!(func: post_concat_transform(transform), self)
    }

    /// Set the current transformation matrix.
    pub fn set_transform(&mut self, transform: Affine) {
        dispatch_mut!(func: set_transform(transform), self)
    }

    /// Reset the current transformation matrix to the identity matrix.
    pub fn reset_transform(&mut self) {
        dispatch_mut!(func: reset_transform(), self)
    }

    /// Get the current transformation matrix.
    pub fn current_transform(&self) -> Affine {
        dispatch!(func: current_transform(), self)
    }

    /// Reset the current render context.
    pub fn reset(&mut self) {
        dispatch_mut!(func: reset(), self)
    }

    /// Render the current render context into a pixmap.
    pub fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        dispatch!(func: render_to_pixmap(pixmap), self)
    }

    /// Get the width of the render context.
    pub fn width(&self) -> usize {
        dispatch!(func: width(), self)
    }

    /// Get the height of the render context.
    pub fn height(&self) -> usize {
        dispatch!(func: height(), self)
    }

    /// Get the wide tiles of the render context.
    pub fn wide_tiles(&self) -> &[WideTile] {
        dispatch!(func: wide_tiles(), self)
    }

    /// Get the alpha values of the render context.
    pub fn alphas(&self) -> &[u32] {
        dispatch!(func: alphas(), self)
    }

    /// Get the line buffer of the render context.
    pub fn line_buf(&self) -> &[FlatLine] {
        dispatch!(func: line_buf(), self)
    }

    /// Get the tiles of the render context.
    pub fn tiles(&self) -> &Tiles {
        dispatch!(func: tiles(), self)
    }

    /// Get the strip buffer of the render context.
    pub fn strip_buf(&self) -> &[Strip] {
        dispatch!(func: strip_buf(), self)
    }
}

macro_rules! avx2 {
    ($e:expr) => {
        // We also require FMA for AVX2 support, but from what I can tell in practice AVX2 support seems
        // to imply FMA support?
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        if std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma")
        {
            return $e;
        }
    };
}

macro_rules! neon {
    ($e:expr) => {
        #[cfg(all(target_arch = "aarch64", feature = "simd"))]
        if std::arch::is_aarch64_feature_detected!("neon") {
            return $e;
        }
    };
}

/// NOTE: BE CAREFUL WHEN CHANGING THIS METHOD! We need to make sure to only choose an inner type
/// when the target CPU actually supports it. Unsafe code relies on the correctness of this method!
fn select_inner_context(
    width: usize,
    height: usize,
    execution_mode: ExecutionMode,
) -> InnerContextType {
    match execution_mode {
        ExecutionMode::Scalar => InnerContextType::Scalar(InnerContext::new(width, height)),
        #[cfg(feature = "simd")]
        ExecutionMode::Auto => {
            neon!(InnerContextType::Neon(InnerContext::new(width, height)));
            avx2!(InnerContextType::Avx2(InnerContext::new(width, height)));

            // Fallback.
            InnerContextType::Scalar(InnerContext::new(width, height))
        }
        #[cfg(all(target_arch = "aarch64", feature = "simd"))]
        ExecutionMode::Neon => {
            neon!(InnerContextType::Neon(InnerContext::new(width, height)));

            panic!(
                "attempted to force execution mode NEON, but CPU doesn't support NEON instructions"
            );
        }
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        ExecutionMode::Avx2 => {
            avx2!(InnerContextType::Avx2(InnerContext::new(width, height)));

            panic!(
                "attempted to force execution mode AVX2, but CPU doesn't support AVX2 instructions"
            );
        }
    }
}

use crate::execute::{ExecutionMode, Scalar};
use crate::kurbo::{Affine, BezPath, Rect, Stroke};
use crate::paint::Paint;
use crate::render::InnerContext;
use crate::strip::Strip;
use crate::tiling::{FlatLine, Tiles};
use crate::wide_tile::WideTile;
pub use pixmap::Pixmap;
