//! Methods for drawing rectangles more efficiently
//!
//! When drawing simple rectangles, we can make quite a few optimizations that are
//! not necessary for arbitrary paths. For example, alpha calculation becomes much easier,
//! which means that we don't need to go through the steps "make tiles", "sort tiles" and "create
//! strips", but can instead straight away generate appropriate strip and fill commands for the
//! corresponding wide tiles, based on the coordinates of the rectangle.

use crate::paint::Paint;
use crate::tiling::FlatLine;
use crate::{FillRule, RenderContext};
use peniko::kurbo::Rect;

impl RenderContext {
    fn strip_rect(&mut self, rect: &Rect) {
        // Don't try to draw empty rects
        if rect.x0 >= rect.x1 || rect.y0 >= rect.y1 {
            return;
        }
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
