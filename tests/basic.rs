use cpu_sparse::{CsRenderCtx, Pixmap};
use peniko::color::palette;
use peniko::kurbo::{BezPath, Circle, Rect, Shape, Stroke};
use std::io::BufWriter;
use std::path::PathBuf;
use oxipng::{InFile, OutFile};

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

        let mut out = vec![];
        let mut encoder = png::Encoder::new(&mut out, ctx.width as u32, ctx.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixmap.data()).unwrap();
        writer.finish().unwrap();

        let optimized = oxipng::optimize_from_memory(&out, &oxipng::Options::max_compression()).unwrap();
        std::fs::write(&path, optimized).unwrap();
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

#[test]
fn filled_triangle() {
    let mut ctx = get_ctx(100, 100, false);

    let path = {
        let mut path = BezPath::new();
        path.move_to((5.0, 5.0));
        path.line_to((95.0, 50.0));
        path.line_to((5.0, 95.0));
        path.close_path();

        path
    };

    ctx.fill(
        &path.into(),
        palette::css::LIME.into(),
    );

    save_pixmap(ctx, Some("filled_triangle"));
}

#[test]
fn stroked_triangle() {
    let mut ctx = get_ctx(100, 100, false);

    let path = {
        let mut path = BezPath::new();
        path.move_to((5.0, 5.0));
        path.line_to((95.0, 50.0));
        path.line_to((5.0, 95.0));
        path.close_path();

        path
    };

    let stroke = Stroke::new(3.0);

    ctx.stroke(
        &path.into(),
        &stroke,
        palette::css::LIME.into(),
    );

    save_pixmap(ctx, Some("stroked_triangle"));
}

#[test]
fn filled_circle() {
    let mut ctx = get_ctx(100, 100, false);

    let circle = Circle::new((50.0, 50.0), 45.0);

    ctx.fill(
        &circle.to_path(0.1).into(),
        palette::css::LIME.into(),
    );

    save_pixmap(ctx, Some("filled_circle"));
}

#[test]
fn stroked_circle() {
    let mut ctx = get_ctx(100, 100, false);

    let circle = Circle::new((50.0, 50.0), 45.0);

    let stroke = Stroke::new(3.0);

    ctx.stroke(
        &circle.to_path(0.1).into(),
        &stroke,
        palette::css::LIME.into(),
    );

    save_pixmap(ctx, Some("stroked_circle"));
}

