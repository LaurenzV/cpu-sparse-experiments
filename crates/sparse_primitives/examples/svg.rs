// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::BufWriter;
use std::str::FromStr;

use peniko::color::{palette, AlphaColor, Srgb};
use peniko::kurbo::{Affine, BezPath, Point, Shape, Size, Stroke, Vec2};
use peniko::{BrushRef, Color};
use roxmltree::Document;
use sparse_primitives::render::Path;
use sparse_primitives::svg::{render_tree, SVGContext};
use sparse_primitives::{Pixmap, RenderContext};
use usvg::tiny_skia_path::PathSegment;
use usvg::{Node, Paint};

pub fn main() {
    let scale = 1.0;
    let svg = std::fs::read_to_string("../../svgs/coat_of_arms.svg").expect("error reading file");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let width = (tree.size().width() * scale).ceil() as usize;
    let height = (tree.size().height() * scale).ceil() as usize;

    let mut ctx = RenderContext::new(width, height);

    let mut sctx = SVGContext::new_with_scale(scale as f64);
    let mut pixmap = Pixmap::new(width, height);

    let num_iters = 200;

    // Hacky code for crude measurements; change this to arg parsing
    let start = std::time::Instant::now();
    for _ in 0..num_iters {
        ctx.reset();
        render_tree(&mut ctx, &mut sctx, &tree);
        ctx.render_to_pixmap(&mut pixmap);
    }

    let end = start.elapsed();
    println!("{:?}ms", end.as_millis() as f32 / num_iters as f32);

    pixmap.unpremultiply();

    let file = std::fs::File::create("cpu_sparse.png").unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixmap.data()).unwrap();
}
