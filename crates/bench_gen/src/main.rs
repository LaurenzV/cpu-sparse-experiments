use std::fmt::format;
use svg::Document;
use rand::Rng;
use rand::rngs::ThreadRng;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 600;
const RECT_SIZES: [(u32, u32); 3] = [(8, 8), (64, 64), (256, 256)];
const NUM_RENDER_CALLS: usize = 1000;

fn main() {
    fill_aligned_rect();
    fill_rotated_rect();
}

fn gen_color(rng: &mut ThreadRng) -> String {
    // Generate random color
    let r = rng.gen_range(0..=255);
    let g = rng.gen_range(0..=255);
    let b = rng.gen_range(0..=255);
    format!("rgb({},{},{})", r, g, b)
}

fn fill_aligned_rect() {
    let mut rng = rand::thread_rng();

    for (width, height) in RECT_SIZES.iter() {
        let mut document = Document::new().set("viewBox", (0, 0, WIDTH, HEIGHT));

        for _ in 0..NUM_RENDER_CALLS {
            // Generate random position aligned to integers
            let x = rng.gen_range(0..=(WIDTH - width)) as u32;
            let y = rng.gen_range(0..=(HEIGHT - height)) as u32;

            let color = gen_color(&mut rng);

            document = document.add(
                svg::node::element::Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", *width)
                    .set("height", *height)
                    .set("fill", color)
                    .set("fill-opacity", 0.5)
            );
        }

        svg::save(format!("fill_rect_a_{}x{}.svg", width, height), &document).unwrap();
    }
}

fn fill_rotated_rect() {
    let mut rng = rand::thread_rng();

    for (width, height) in RECT_SIZES.iter() {
        let mut document = Document::new().set("viewBox", (0, 0, WIDTH, HEIGHT));

        let mut angle = 0.0;

        for _ in 0..NUM_RENDER_CALLS {
            // Generate random position aligned to integers
            let x = rng.gen_range(0..=(WIDTH - width)) as u32;
            let y = rng.gen_range(0..=(HEIGHT - height)) as u32;

            let color = gen_color(&mut rng);

            document = document.add(
                svg::node::element::Rectangle::new()
                    .set("x", x)
                    .set("y", y)
                    .set("width", *width)
                    .set("height", *height)
                    .set("fill", color)
                    .set("transform", format!("rotate({}, {}, {})", angle, x + *width / 2, y + *height / 2))
                    .set("fill-opacity", 0.5)
            );

            angle += 0.05;
        }

        svg::save(format!("fill_rect_rotated_{}x{}.svg", width, height), &document).unwrap();
    }
}