//! Methods for drawing rectangles more efficiently
//!
//! When drawing simple rectangles, we can make quite a few optimizations that are
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
    pub(crate) fn strip_rect(&mut self, rect: &Rect) {
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
        let y0_floored = y0.floor();
        let x1_ceiled = x1.ceil();
        let x1_floored = x1.floor();
        let y1_ceiled = y1.ceil();

        let x_start = x0_floored as u32;
        // Inclusive, i.e. the pixel at column `x_end` is the right border of the rectangle,
        // which should be stripped.
        let x_end = x1_floored as u32;

        // Calculate the alpha coverages of the top/bottom borders of the rectangle.
        let horizontal_alphas = |strip_y: u32| {
            let mut buf = [0.0f32; STRIP_HEIGHT];

            let height_start = y0 - strip_y as f32;
            let height_end = y1 - strip_y as f32;

            for i in 0..STRIP_HEIGHT {
                let fi = i as f32;
                let upper_coverage = 1.0 - (height_start - fi).clamp(0.0, 1.0);
                let bottom_coverage = (height_end - fi).clamp(0.0, 1.0);

                buf[i] = upper_coverage * bottom_coverage;
            }

            buf
        };

        let vertical_alpha = |right: bool| -> f32 {
            // The reason we need to calculate the coverage from the left and
            if right {
                let start = (x0 - x1_floored).max(0.0);
                let end = x1 - x1_floored;

                end - start
            } else {
                let end = (x1 - x0_floored).min(1.0);
                let start = x0 - x0_floored;

                end - start
            }
        };

        let left_alpha = vertical_alpha(false);
        let right_alpha = vertical_alpha(true);

        let alpha = |alphas: &[f32; 4], coverage: f32| {
            let mut packed_alphas = 0;

            for i in 0..STRIP_HEIGHT {
                let u8_alpha = ((alphas[i] * coverage) * 255.0 + 0.5) as u32;
                packed_alphas += u8_alpha << (i * 8);
            }

            packed_alphas
        };

        // Create a strip for the top/bottom edge of the rectangle.
        let horizontal_strip = |alpha_buf: &mut Vec<u32>,
                                strip_buf: &mut Vec<Strip>,
                                alphas: &[f32; 4],
                                strip_y: u32| {
            // Let's first start by stripping the horizontal line of the rectangle.
            // Strip the first column, which might have an additional alpha due to non-integer
            // alignment of x0.
            let mut col = alpha_buf.len() as u32;
            alpha_buf.push(alpha(&alphas, left_alpha));

            // If the rect covers more than one pixel horizontally, fill all the remaining ones with
            // the same opacity as in `alphas`, and deal with the final column similarly to the
            // first one.
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
                x: x0_floored as u32,
                y: strip_y,
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

        // If rect covers more than one strip, we need to strip the vertical line segments
        // of the rectangle, and finally the bottom horizontal line segment.
        if top_strip_index != bottom_strip_index {
            let alphas = [1.0, 1.0, 1.0, 1.0];

            for i in (top_strip_index + 1)..bottom_strip_index {
                // Left side.
                let mut col = self.alphas.len() as u32;
                self.alphas.push(alpha(&alphas, left_alpha));

                self.strip_buf.push(Strip {
                    x: x0_floored as u32,
                    y: i * STRIP_HEIGHT as u32,
                    col,
                    winding: 0,
                });

                // Right side.
                col = self.alphas.len() as u32;
                self.alphas.push(alpha(&alphas, vertical_alpha(true)));

                self.strip_buf.push(Strip {
                    x: x1_floored as u32,
                    y: i * STRIP_HEIGHT as u32,
                    col,
                    winding: 1,
                });
            }

            let bottom_alphas = horizontal_alphas(bottom_strip_y);
            horizontal_strip(
                &mut self.alphas,
                &mut self.strip_buf,
                &bottom_alphas,
                bottom_strip_y,
            );
        }

        // Push sentinel strip
        self.strip_buf.push(Strip {
            x: 65524,
            y: 65532,
            col: self.alphas.len() as u32,
            winding: 0,
        })
    }
}

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
