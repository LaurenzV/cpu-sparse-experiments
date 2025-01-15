use std::io::BufWriter;
use std::path::PathBuf;
use cpu_sparse::{CsRenderCtx, Pixmap};

fn save_pixmap(ctx: CsRenderCtx, name: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("refs")
        .join(name)
        .with_extension("png");

    let mut pixmap = Pixmap::new(ctx.width, ctx.height);
    ctx.render_to_pixmap(&mut pixmap);
    pixmap.unpremultiply();

    let file = std::fs::File::create(&path).unwrap();
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, ctx.width as u32, ctx.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixmap.data()).unwrap();
}

#[test]
fn empty() {
    let ctx = CsRenderCtx::new(50, 50);
    save_pixmap(ctx, "empty");
}