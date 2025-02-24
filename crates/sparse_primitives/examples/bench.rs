use bench_gen::{Command, Params, RectIterator, RectType};
use peniko::kurbo::{Cap, Join, Stroke};
use peniko::{BlendMode, Compose, Mix};
use sparse_primitives::{FillRule, Pixmap, RenderContext};
use std::io::BufWriter;
use std::time::Instant;

const WIDTH: usize = 512;
const HEIGHT: usize = 600;
const RENDER_CALLS: u32 = 1000;
const STROKE_WIDTH: f64 = 2.0;

fn main() {
    let mut ctx = RenderContext::new(WIDTH, HEIGHT);

    for size in [256].repeat(200) {
        ctx.reset();

        let params = Params {
            width: WIDTH,
            height: HEIGHT,
            stroke: false,
            size,
        };

        let commands = RectIterator::new(params, RectType::Unaligned)
            .take(RENDER_CALLS as usize)
            .collect::<Vec<_>>();

        let start = Instant::now();
        let mut pixmap = Pixmap::new(WIDTH, HEIGHT);

        for cmd in &commands {
            run_cmd(&mut ctx, cmd);
        }

        ctx.render_to_pixmap(&mut pixmap);

        let elapsed = start.elapsed();

        println!("Runtime for {}x{}: {:?}", size, size, elapsed);
        // write_pixmap(&mut pixmap, size);
    }
}

#[allow(dead_code)]
fn write_pixmap(pixmap: &mut Pixmap, size: usize) {
    pixmap.unpremultiply();
    let file = std::fs::File::create(format!("out-{}x{}.png", size, size)).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixmap.data()).unwrap();
}

fn run_cmd(ctx: &mut RenderContext, cmd: &Command) {
    match cmd {
        Command::FillRect(r, c) => {
            ctx.set_paint((*c).into());
            ctx.fill_rect(r);
        }
        Command::StrokeRect(r, c) => {
            let stroke = stroke();

            ctx.set_paint((*c).into());
            ctx.set_stroke(stroke);
            ctx.stroke_rect(r);
        }
        Command::FillPath(p, c, nz) => {
            let fill_rule = if *nz {
                FillRule::NonZero
            } else {
                FillRule::EvenOdd
            };

            ctx.set_fill_rule(fill_rule);
            ctx.set_paint((*c).into());

            ctx.fill_path(&p.clone().into());
        }
        Command::StrokePath(p, c) => {
            let stroke = stroke();

            ctx.set_stroke(stroke);
            ctx.set_paint((*c).into());
            ctx.stroke_path(&p.clone().into());
        }
    }
}

fn stroke() -> Stroke {
    Stroke {
        width: STROKE_WIDTH,
        join: Join::Miter,
        start_cap: Cap::Square,
        end_cap: Cap::Square,
        ..Default::default()
    }
}
