// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use cpu_sparse::{CsRenderCtx, Pixmap};
use peniko::color::palette;
use peniko::kurbo::{BezPath, Stroke};
use std::io::BufWriter;
use std::time::Instant;

const WIDTH: usize = 64;
const HEIGHT: usize = 64;

pub fn main() {
    let mut ctx = CsRenderCtx::new(WIDTH, HEIGHT);
    let mut path = BezPath::new();
    path.move_to((2.5, 2.5));
    path.line_to((45.0, 15.0));
    path.line_to((7.5, 45.0));
    // path.move_to((2.5, 2.5));
    // path.line_to((20.0, 2.5));
    // path.line_to((20.0, 7.5));
    // path.line_to((7.5, 10.0));
    path.close_path();
    let piet_path = path.into();
    ctx.fill(&piet_path, palette::css::DARK_BLUE.into());
    // let stroke = Stroke::new(1.0);
    // ctx.stroke(&piet_path, &stroke, palette::css::DARK_BLUE.into());

    if let Some(filename) = std::env::args().nth(1) {
        let mut pixmap = Pixmap::new(WIDTH, HEIGHT);
        ctx.render_to_pixmap(&mut pixmap);
        pixmap.unpremultiply();
        let file = std::fs::File::create(filename).unwrap();
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixmap.data()).unwrap();

        ctx.debug_dump();
    } else {
        ctx.debug_dump();
    }
}
