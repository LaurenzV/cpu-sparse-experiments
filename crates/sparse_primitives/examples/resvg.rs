use tiny_skia::Pixmap;
use usvg::Transform;

fn main() {
    let scale = 1.0;
    let svg = std::fs::read_to_string("../../svgs/coat_of_arms.svg").expect("error reading file");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let width = (tree.size().width() * scale).ceil() as usize;
    let height = (tree.size().height() * scale).ceil() as usize;

    let num_iters = 100;
    let mut pix = None;

    // Hacky code for crude measurements; change this to arg parsing
    let start = std::time::Instant::now();
    for _ in 0..num_iters {
        let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();
        resvg::render(&tree, Transform::from_scale(1.0, 1.0), &mut pixmap.as_mut());

        pix = Some(pixmap);
    }

    let end = start.elapsed();
    println!("{:?}ms", end.as_millis() as f32 / num_iters as f32);

    pix.unwrap().save_png("tiny_skia.png").unwrap();
}
