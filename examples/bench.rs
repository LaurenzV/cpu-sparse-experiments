use bench_gen::{Command, Params, PolyIterator, RectIterator, RectType};
use cpu_sparse::{FillRule, Pixmap, RenderContext};
use peniko::kurbo::{Cap, Join, Stroke};
use std::io::BufWriter;
use std::time::Instant;

const WIDTH: usize = 512;
const HEIGHT: usize = 600;
const RENDER_CALLS: u32 = 1000;
const STROKE_WIDTH: f64 = 2.0;

fn main() {
    let mut ctx = RenderContext::new(WIDTH, HEIGHT);

    for size in [8, 16, 32, 64, 128, 256].repeat(1) {
        ctx.reset();

        let params = Params {
            width: WIDTH,
            height: HEIGHT,
            alpha: 127,
            stroke: false,
            size,
        };

        let commands = PolyIterator::new(params, 40, false)
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
        write_pixmap(&mut pixmap, size);
    }
}

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
            ctx.fill_rect(&r, (*c).into());
        }
        Command::StrokeRect(r, c) => {
            let stroke = stroke();
            ctx.stroke_rect(&r, &stroke, (*c).into());
        }
        Command::FillPath(p, c) => {
            ctx.fill_path(&p.clone().into(), FillRule::NonZero, (*c).into());
        }
        Command::StrokePath(p, c) => {
            let stroke = stroke();
            ctx.stroke_path(&p.clone().into(), &stroke, (*c).into());
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
