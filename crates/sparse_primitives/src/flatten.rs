// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Utilities for flattening

use flatten::stroke::LoweredPath;
use peniko::kurbo::{self, Affine, BezPath, Line, Stroke};

use crate::tiling::{FlatLine, Point};

/// The flattening tolerance
const TOL: f64 = 0.25;

pub fn fill(path: &BezPath, affine: Affine, line_buf: &mut Vec<FlatLine>) {
    line_buf.clear();
    let mut start = kurbo::Point::default();
    let mut p0 = kurbo::Point::default();
    let iter = path.iter().map(|el| affine * el);

    let mut closed = false;

    kurbo::flatten(iter, TOL, |el| match el {
        kurbo::PathEl::MoveTo(p) => {
            if !closed && p0 != start {
                close_path(start, p0, line_buf);
            }

            closed = false;
            start = p;
            p0 = p;
        }
        kurbo::PathEl::LineTo(p) => {
            let pt0 = Point::new(p0.x as f32, p0.y as f32);
            let pt1 = Point::new(p.x as f32, p.y as f32);
            line_buf.push(FlatLine::new(pt0, pt1));
            p0 = p;
        }
        kurbo::PathEl::QuadTo(_, _) => unreachable!(),
        kurbo::PathEl::CurveTo(_, _, _) => unreachable!(),
        kurbo::PathEl::ClosePath => {
            closed = true;

            close_path(start, p0, line_buf);
        }
    });

    if !closed {
        close_path(start, p0, line_buf);
    }
}

pub fn stroke(path: &BezPath, style: &Stroke, affine: Affine, line_buf: &mut Vec<FlatLine>) {
    line_buf.clear();

    // TODO: Temporary hack to ensure that strokes are scaled properly by the transform.
    let tolerance = TOL / affine.as_coeffs()[0].abs().max(affine.as_coeffs()[3].abs());

    let lines: LoweredPath<Line> = flatten::stroke::stroke_undashed(path.iter(), style, tolerance);
    for line in &lines.path {
        let scaled_p0 = affine * line.p0;
        let scaled_p1 = affine * line.p1;
        let p0 = Point::new(scaled_p0.x as f32, scaled_p0.y as f32);
        let p1 = Point::new(scaled_p1.x as f32, scaled_p1.y as f32);
        line_buf.push(FlatLine::new(p0, p1));
    }
}

fn close_path(start: kurbo::Point, p0: kurbo::Point, line_buf: &mut Vec<FlatLine>) {
    let pt0 = Point::new(p0.x as f32, p0.y as f32);
    let pt1 = Point::new(start.x as f32, start.y as f32);

    if pt0 != pt1 {
        line_buf.push(FlatLine::new(pt0, pt1));
    }
}
