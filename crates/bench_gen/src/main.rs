use std::fmt::format;
use svg::Document;
use rand::Rng;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 600;
const RECT_SIZES: [(u32, u32); 3] = [(8, 8), (64, 64), (256, 256)];
const NUM_RENDER_CALLS: usize = 1000;

fn main() {
    gen_aligned_rects();
}

fn gen_aligned_rects() {
    let mut rng = rand::thread_rng();

    for (width, height) in RECT_SIZES.iter() {
        let mut document = Document::new().set("viewBox", (0, 0, WIDTH, HEIGHT));

        for _ in 0..NUM_RENDER_CALLS {
            // Generate random position aligned to integers
            let x = rng.gen_range(0..=(WIDTH - width)) as u32;
            let y = rng.gen_range(0..=(HEIGHT - height)) as u32;

            // Generate random color
            let r = rng.gen_range(0..=255);
            let g = rng.gen_range(0..=255);
            let b = rng.gen_range(0..=255);
            let color = format!("rgb({},{},{})", r, g, b);

            // Add rectangle to document
            document = document.add(
                svg::node::element::Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", *width)
                    .set("height", *height)
                    .set("fill", color)
                    .set("opacity", 1.0)
            );
        }

        svg::save(format!("rect_a_{}x{}.svg", width, height), &document).unwrap();
    }
}