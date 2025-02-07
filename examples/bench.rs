use bench_gen::{Command, FillRectAIterator, FillRectRotIterator, FillRectUIterator, Params};
use cpu_sparse::{FillRule, Pixmap, RenderContext};
use std::io::BufWriter;
use std::time::Instant;

const WIDTH: usize = 512;
const HEIGHT: usize = 600;
const RENDER_CALLS: u32 = 1000;

fn main() {
    let mut ctx = RenderContext::new(WIDTH, HEIGHT);

    for size in [8, 16, 32, 64, 128, 256] {
        ctx.reset();

        let params = Params {
            width: 512,
            height: 600,
            size,
        };

        let commands = FillRectRotIterator::new(params)
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
        Command::FillPath(p, c) => {
            ctx.fill_path(&p.clone().into(), FillRule::NonZero, (*c).into());
        }
    }
}
