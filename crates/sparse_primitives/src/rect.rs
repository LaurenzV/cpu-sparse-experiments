//! Accelerators for filling and stroking rectangles more efficiently.
//!
//! When filling/stroking simple rectangles, we can make quite a few optimizations that are
//! not necessary for arbitrary paths. For example, alpha calculation becomes much easier,
//! which means that we don't need to go through the steps "make tiles", "sort tiles" and "create
//! strips", but can instead straight away generate appropriate strip and fill commands for the
//! corresponding wide tiles, based on the coordinates of the rectangle.

use crate::execute::KernelExecutor;
use crate::render::{InnerContext, DEFAULT_TOLERANCE};
use vello_common::kurbo::{Rect, Shape};

impl<KE: KernelExecutor> InnerContext<KE> {
    pub(crate) fn fill_rect(&mut self, rect: &Rect) {
        self.fill_path(&rect.to_path(DEFAULT_TOLERANCE).into());
    }

    pub(crate) fn stroke_rect(&mut self, rect: &Rect) {
        self.stroke_path(&rect.to_path(DEFAULT_TOLERANCE).into());
    }
}
