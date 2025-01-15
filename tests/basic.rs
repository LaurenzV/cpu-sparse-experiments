use cpu_sparse::{CsRenderCtx, Pixmap};
use peniko::color::palette;
use peniko::kurbo::{BezPath, Rect, Shape};
use std::io::BufWriter;
use std::path::PathBuf;

const RECT_TOLERANCE: f32 = 0.1;

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

fn get_ctx(width: usize, height: usize, transparent: bool) -> CsRenderCtx {
    let mut ctx = CsRenderCtx::new(width, height);
    if !transparent {
        let path = Rect::new(0.0, 0.0, width as f64, height as f64).to_path(0.1);

        ctx.fill(&path.into(), palette::css::WHITE.into());
    }

    ctx
}

#[test]
fn empty_1x1() {
    let ctx = get_ctx(1, 1, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_5x1() {
    let ctx = get_ctx(5, 1, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_1x5() {
    let ctx = get_ctx(1, 5, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_3x10() {
    let ctx = get_ctx(3, 10, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_23x45() {
    let ctx = get_ctx(23, 45, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_50x50() {
    let ctx = get_ctx(50, 50, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_463x450() {
    let ctx = get_ctx(463, 450, true);
    save_pixmap(ctx, None);
}

#[test]
fn empty_1134x1376() {
    let ctx = get_ctx(1134, 1376, true);
    save_pixmap(ctx, None);
}

#[test]
fn full_cover_1() {
    let mut ctx = get_ctx(8, 8, true);
    ctx.fill(
        &Rect::new(0.0, 0.0, 8.0, 8.0).to_path(0.1).into(),
        palette::css::BEIGE.into(),
    );

    save_pixmap(ctx, Some("full_cover_1"))
}
