use cpu_sparse::{CsRenderCtx, Pixmap};
use std::io::BufWriter;
use std::path::PathBuf;

fn save_pixmap(ctx: CsRenderCtx, name: Option<&str>) {
    let mut pixmap = Pixmap::new(ctx.width, ctx.height);
    ctx.render_to_pixmap(&mut pixmap);
    pixmap.unpremultiply();

    if let Some(name) = name {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("refs")
            .join(name)
            .with_extension("png");

        let file = std::fs::File::create(&path).unwrap();
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, ctx.width as u32, ctx.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixmap.data()).unwrap();
    }
}

#[test]
fn empty_1x1() {
    let ctx = CsRenderCtx::new(1, 1);
    save_pixmap(ctx, None);
}

#[test]
fn empty_5x1() {
    let ctx = CsRenderCtx::new(5, 1);
    save_pixmap(ctx, None);
}

#[test]
fn empty_1x5() {
    let ctx = CsRenderCtx::new(1, 5);
    save_pixmap(ctx, None);
}

#[test]
fn empty_3x10() {
    let ctx = CsRenderCtx::new(3, 10);
    save_pixmap(ctx, None);
}

#[test]
fn empty_23x45() {
    let ctx = CsRenderCtx::new(23, 45);
    save_pixmap(ctx, None);
}

#[test]
fn empty_50x50() {
    let ctx = CsRenderCtx::new(50, 50);
    save_pixmap(ctx, None);
}

#[test]
fn empty_463x450() {
    let ctx = CsRenderCtx::new(463, 450);
    save_pixmap(ctx, None);
}

#[test]
fn empty_1134x1376() {
    let ctx = CsRenderCtx::new(1134, 1376);
    save_pixmap(ctx, None);
}
