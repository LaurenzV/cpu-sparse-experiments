//! Accelerators for filling and stroking rectangles more efficiently.
//!
//! When filling/stroking simple rectangles, we can make quite a few optimizations that are
//! not necessary for arbitrary paths. For example, alpha calculation becomes much easier,
//! which means that we don't need to go through the steps "make tiles", "sort tiles" and "create
//! strips", but can instead straight away generate appropriate strip and fill commands for the
//! corresponding wide tiles, based on the coordinates of the rectangle.

use crate::strip::Strip;
use crate::tiling::FlatLine;
use crate::wide_tile::STRIP_HEIGHT;
use crate::RenderContext;
use peniko::kurbo::Rect;

impl RenderContext {
    /// When filling a non-skewed rectangle, there are quite a few simplifying assumptions
    /// we can make, and thus we can avoid the expensive tiling + strip generation stage, and
    /// instead run a customized, much more efficient strip generation algorithm.
    ///
    /// The basic idea is very simple: First, we generate strips that cover the top-horizontal
    /// part of the rectangle, then we generate strips for the vertical left and right
    /// line segments of the rectangle, and finally strips that cover the bottom-horizontal
    /// part of the rectangle.
    pub(crate) fn strip_filled_rect(&mut self, rect: &Rect) {
        self.strip_buf.clear();

        // Don't try to draw empty rects
        if rect.x0 >= rect.x1 || rect.y0 >= rect.y1 {
            return;
        }

        let (x0, x1, y0, y1) = (
            rect.x0 as f32,
            rect.x1 as f32,
            rect.y0 as f32,
            rect.y1 as f32,
        );

        let top_strip_index = y0 as u32 / STRIP_HEIGHT as u32;
        let top_strip_y = top_strip_index * STRIP_HEIGHT as u32;

        let bottom_strip_index = y1 as u32 / STRIP_HEIGHT as u32;
        let bottom_strip_y = bottom_strip_index * STRIP_HEIGHT as u32;

        let x0_floored = x0.floor();
        let x1_floored = x1.floor();

        let x_start = x0_floored as u32;
        // Inclusive, i.e. the pixel at column `x_end` is the very right border (possibly only anti-aliased)
        // of the rectangle, which should still be stripped.
        let x_end = x1_floored as u32;

        // Calculate the vertical/horizontal coverage of a pixel, using a start
        // and end point whose area in-between should be considered covered.
        let pixel_coverage = |pixel_pos: u32, start: f32, end: f32| {
            let pixel_pos = pixel_pos as f32;
            let end = (end - pixel_pos).clamp(0.0, 1.0);
            let start = (start - pixel_pos).clamp(0.0, 1.0);

            end - start
        };

        // Calculate the alpha coverages of the top/bottom borders of the rectangle.
        let horizontal_alphas = |strip_y: u32| {
            let mut buf = [0.0f32; STRIP_HEIGHT];

            // For each row in the strip, calculate how much it is covered by y0/y1.
            for i in 0..STRIP_HEIGHT {
                buf[i] = pixel_coverage(strip_y + i as u32, y0, y1);
            }

            buf
        };

        let left_alpha = pixel_coverage(x_start, x0, x1);
        let right_alpha = pixel_coverage(x_end, x0, x1);

        // Calculate the alpha coverage of a strip using an alpha mask. For example, if we
        // want to calculate the coverage of the very first column of the top line in the
        // rect (which might start at the subpixel offset .5), then we need to multiply
        // all its alpha values by 0.5 to account for anti-aliasing.
        let alpha = |alphas: &[f32; 4], alpha_mask: f32| {
            let mut packed_alphas = 0;

            for i in 0..STRIP_HEIGHT {
                let u8_alpha = ((alphas[i] * alpha_mask) * 255.0 + 0.5) as u32;
                packed_alphas += u8_alpha << (i * 8);
            }

            packed_alphas
        };

        // Create a strip for the top/bottom edge of the rectangle.
        let horizontal_strip = |alpha_buf: &mut Vec<u32>,
                                strip_buf: &mut Vec<Strip>,
                                alphas: &[f32; 4],
                                strip_y: u32| {
            // Strip the first column, which might have an additional alpha mask due to non-integer
            // alignment of x0.
            let mut col = alpha_buf.len() as u32;
            alpha_buf.push(alpha(&alphas, left_alpha));

            // If the rect covers more than one pixel horizontally, fill all the remaining ones
            // except for the last one with the same opacity as in `alphas`.
            // If the rect is within one pixel horizontally, then right_alpha == left_alpha, and thus
            // the alpha we pushed above is enough.
            if x_end - x_start > 1 {
                for _ in (x_start + 1)..x_end {
                    alpha_buf.push(alpha(&alphas, 1.0));
                }

                // Fill the last, right column, which might also need an additional alpha mask
                // due to non-integer alignment of x1.
                alpha_buf.push(alpha(&alphas, right_alpha));
            }

            // Push the actual strip.
            strip_buf.push(Strip {
                x: x0_floored as i32,
                y: strip_y as i32,
                col,
                winding: 0,
            });
        };

        let top_alphas = horizontal_alphas(top_strip_y);
        horizontal_strip(
            &mut self.alphas,
            &mut self.strip_buf,
            &top_alphas,
            top_strip_y,
        );

        // If rect covers more than one strip vertically, we need to strip the vertical line
        // segments of the rectangle, and finally the bottom horizontal line segment.
        if top_strip_index != bottom_strip_index {
            let alphas = [1.0, 1.0, 1.0, 1.0];

            for i in (top_strip_index + 1)..bottom_strip_index {
                // Left side (and right side if rect is only one pixel wide).
                let mut col = self.alphas.len() as u32;
                self.alphas.push(alpha(&alphas, left_alpha));

                self.strip_buf.push(Strip {
                    x: x0_floored as i32,
                    y: (i * STRIP_HEIGHT as u32) as i32,
                    col,
                    winding: 0,
                });

                if x_end > x_start {
                    // Right side.
                    col = self.alphas.len() as u32;
                    self.alphas.push(alpha(&alphas, right_alpha));

                    self.strip_buf.push(Strip {
                        x: x1_floored as i32,
                        y: (i * STRIP_HEIGHT as u32) as i32,
                        col,
                        winding: 1,
                    });
                }
            }

            // Strip the bottom part of the rectangle.
            let bottom_alphas = horizontal_alphas(bottom_strip_y);
            horizontal_strip(
                &mut self.alphas,
                &mut self.strip_buf,
                &bottom_alphas,
                bottom_strip_y,
            );
        }

        // Push sentinel strip.
        self.strip_buf.push(Strip {
            x: 65524,
            y: 65532,
            col: self.alphas.len() as u32,
            winding: 0,
        })
    }
}

/// Check if a sequence of flat lines can be reduced to a rectangle.
pub(crate) fn lines_to_rect(line_buf: &[FlatLine], width: usize, height: usize) -> Option<Rect> {
    if line_buf.len() != 4 {
        return None;
    }

    let mut horizontal = line_buf[0].p0.x != line_buf[0].p1.x;
    let mut is_rect = true;

    let mut x_min = width as f32;
    let mut y_min = height as f32;
    let mut x_max = 0.0f32;
    let mut y_max = 0.0f32;

    for i in 0..4 {
        if horizontal {
            is_rect &= line_buf[i].p0.y == line_buf[i].p1.y;
        } else {
            is_rect &= line_buf[i].p0.x == line_buf[i].p1.x;
        }

        x_min = x_min.min(line_buf[i].p0.x.min(line_buf[i].p1.x));
        y_min = y_min.min(line_buf[i].p0.y.min(line_buf[i].p1.y));
        x_max = x_max.max(line_buf[i].p0.x.max(line_buf[i].p1.x));
        y_max = y_max.max(line_buf[i].p0.y.max(line_buf[i].p1.y));

        horizontal = !horizontal;
    }

    if is_rect {
        Some(Rect::new(
            x_min.max(0.0) as f64,
            y_min.max(0.0) as f64,
            x_max.min(width as f32) as f64,
            y_max.min(height as f32) as f64,
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::lines_to_rect;
    use crate::tiling::{FlatLine, Point};
    use peniko::kurbo::Rect;

    #[test]
    fn lines_to_rect_1() {
        let lines = [
            FlatLine::new(Point::new(-10.0, -10.0), Point::new(10.0, -10.0)),
            FlatLine::new(Point::new(10.0, -10.0), Point::new(10.0, 10.0)),
            FlatLine::new(Point::new(10.0, 10.0), Point::new(-10.0, 10.0)),
            FlatLine::new(Point::new(-10.0, 10.0), Point::new(-10.0, -10.0)),
        ];

        assert_eq!(
            lines_to_rect(&lines, 200, 200).unwrap(),
            Rect::new(0.0, 0.0, 10.0, 10.0)
        )
    }

    #[test]
    fn lines_to_rect_2() {
        let lines = [
            FlatLine::new(Point::new(10.0, -10.0), Point::new(-10.0, -10.0)),
            FlatLine::new(Point::new(-10.0, -10.0), Point::new(-10.0, 10.0)),
            FlatLine::new(Point::new(-10.0, 10.0), Point::new(10.0, 10.0)),
            FlatLine::new(Point::new(10.0, 10.0), Point::new(10.0, -10.0)),
        ];

        assert_eq!(
            lines_to_rect(&lines, 200, 200).unwrap(),
            Rect::new(0.0, 0.0, 10.0, 10.0)
        )
    }
}
