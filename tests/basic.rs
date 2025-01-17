use crate::util::{check_ref, render_pixmap};
use cpu_sparse::{CsRenderCtx, FillRule, Pixmap};
use oxipng::{InFile, OutFile};
use peniko::color::palette::css::{BLUE, GREEN, LIME, REBECCA_PURPLE, RED};
use peniko::color::{palette, AlphaColor};
use peniko::kurbo::{BezPath, Circle, Rect, Shape, Stroke};
use std::io::BufWriter;
use std::path::PathBuf;

mod util;

const RECT_TOLERANCE: f32 = 0.1;

fn get_ctx(width: usize, height: usize, transparent: bool) -> CsRenderCtx {
    let mut ctx = CsRenderCtx::new(width, height);
    if !transparent {
        let path = Rect::new(0.0, 0.0, width as f64, height as f64).to_path(0.1);

        ctx.fill(&path.into(), FillRule::NonZero, palette::css::WHITE.into());
    }

    ctx
}

#[test]
fn empty_1x1() {
    let ctx = get_ctx(1, 1, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_5x1() {
    let ctx = get_ctx(5, 1, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_1x5() {
    let ctx = get_ctx(1, 5, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_3x10() {
    let ctx = get_ctx(3, 10, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_23x45() {
    let ctx = get_ctx(23, 45, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_50x50() {
    let ctx = get_ctx(50, 50, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_463x450() {
    let ctx = get_ctx(463, 450, true);
    render_pixmap(&ctx);
}

#[test]
fn empty_1134x1376() {
    let ctx = get_ctx(1134, 1376, true);
    render_pixmap(&ctx);
}

#[test]
fn full_cover_1() {
    let mut ctx = get_ctx(8, 8, true);
    ctx.fill(
        &Rect::new(0.0, 0.0, 8.0, 8.0).to_path(0.1).into(),
        FillRule::NonZero,
        palette::css::BEIGE.into(),
    );

    check_ref(&ctx, "full_cover_1")
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

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

    check_ref(&ctx, "filled_triangle");
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
    ctx.stroke(&path.into(), &stroke, palette::css::LIME.into());

    check_ref(&ctx, "stroked_triangle");
}

#[test]
fn filled_circle() {
    let mut ctx = get_ctx(100, 100, false);
    let circle = Circle::new((50.0, 50.0), 45.0);
    ctx.fill(
        &circle.to_path(0.1).into(),
        FillRule::NonZero,
        palette::css::LIME.into(),
    );

    check_ref(&ctx, "filled_circle");
}

#[test]
fn filled_circle_with_opacity() {
    let mut ctx = get_ctx(100, 100, false);
    let circle = Circle::new((50.0, 50.0), 45.0);
    ctx.fill(
        &circle.to_path(0.1).into(),
        FillRule::NonZero,
        REBECCA_PURPLE.with_alpha(0.5).into(),
    );

    check_ref(&ctx, "filled_circle_with_opacity");
}

#[test]
fn filled_overlapping_circles() {
    let mut ctx = get_ctx(100, 100, false);

    for e in [(35.0, 35.0, RED), (65.0, 35.0, GREEN), (50.0, 65.0, BLUE)] {
        let circle = Circle::new((e.0, e.1), 30.0);
        ctx.fill(
            &circle.to_path(0.1).into(),
            FillRule::NonZero,
            e.2.with_alpha(0.5).into(),
        );
    }

    check_ref(&ctx, "filled_overlapping_circles");
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

    check_ref(&ctx, "stroked_circle");
}
