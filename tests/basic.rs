use crate::util::{check_ref, get_ctx, render_pixmap};
use cpu_sparse::{FillRule, Pixmap, RenderContext};
use oxipng::{InFile, OutFile};
use peniko::color::palette::css::{BLUE, GREEN, LIME, MAROON, REBECCA_PURPLE, RED};
use peniko::color::{palette, AlphaColor};
use peniko::kurbo::{Affine, BezPath, Circle, Join, Point, Rect, Shape, Stroke};
use std::f64::consts::PI;

mod util;

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
    ctx.fill_path(
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

    ctx.fill_path(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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
    ctx.stroke_path(&path.into(), &stroke, palette::css::LIME.into());

    check_ref(&ctx, "stroked_triangle");
}

#[test]
fn filled_circle() {
    let mut ctx = get_ctx(100, 100, false);
    let circle = Circle::new((50.0, 50.0), 45.0);
    ctx.fill_path(
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
    ctx.fill_path(
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
        ctx.fill_path(
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

    ctx.stroke_path(
        &circle.to_path(0.1).into(),
        &stroke,
        palette::css::LIME.into(),
    );

    check_ref(&ctx, "stroked_circle");
}

fn star_path() -> BezPath {
    let mut path = BezPath::new();
    path.move_to((50.0, 10.0));
    path.line_to((75.0, 90.0));
    path.line_to((10.0, 40.0));
    path.line_to((90.0, 40.0));
    path.line_to((25.0, 90.0));
    path.line_to((50.0, 10.0));

    path
}

#[test]
fn filling_nonzero_rule() {
    let mut ctx = get_ctx(100, 100, false);
    let star = star_path();

    ctx.fill_path(&star.into(), FillRule::NonZero, MAROON.into());

    check_ref(&ctx, "filling_nonzero_rule");
}

#[test]
fn filling_evenodd_rule() {
    let mut ctx = get_ctx(100, 100, false);
    let star = star_path();

    ctx.fill_path(&star.into(), FillRule::EvenOdd, MAROON.into());

    check_ref(&ctx, "filling_evenodd_rule");
}

#[test]
fn filled_aligned_rect() {
    let mut ctx = get_ctx(30, 20, false);
    let rect = Rect::new(1.0, 1.0, 29.0, 19.0);
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_aligned_rect");
}

#[test]
fn stroked_unaligned_rect() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 25.0, 25.0);
    let stroke = Stroke {
        width: 1.0,
        join: Join::Miter,
        ..Default::default()
    };
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_unaligned_rect");
}

#[test]
fn stroked_aligned_rect() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 25.0, 25.0);
    let stroke = miter_stroke_2();
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_aligned_rect");
}

#[test]
fn overflowing_stroked_rect() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(12.5, 12.5, 17.5, 17.5);
    let stroke = Stroke {
        width: 5.0,
        join: Join::Miter,
        ..Default::default()
    };
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "overflowing_stroked_rect");
}

#[test]
fn round_stroked_rect() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 25.0, 25.0);
    let stroke = Stroke::new(3.0);
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "round_stroked_rect");
}

#[test]
fn bevel_stroked_rect() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 25.0, 25.0);
    let stroke = Stroke {
        width: 3.0,
        join: Join::Bevel,
        ..Default::default()
    };
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "bevel_stroked_rect");
}

#[test]
fn filled_unaligned_rect() {
    let mut ctx = get_ctx(30, 20, false);
    let rect = Rect::new(1.5, 1.5, 28.5, 18.5);
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_unaligned_rect");
}

#[test]
fn filled_transformed_rect_1() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    ctx.transform(Affine::translate((10.0, 10.0)));
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_transformed_rect_1");
}

#[test]
fn filled_transformed_rect_2() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 10.0, 10.0);
    ctx.transform(Affine::scale(2.0));
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_transformed_rect_2");
}

#[test]
fn filled_transformed_rect_3() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    ctx.transform(Affine::new([2.0, 0.0, 0.0, 2.0, 5.0, 5.0]));
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_transformed_rect_3");
}

#[test]
fn filled_transformed_rect_4() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
    ctx.transform(Affine::rotate_about(
        45.0 * PI / 180.0,
        Point::new(15.0, 15.0),
    ));
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "filled_transformed_rect_4");
}

#[test]
fn stroked_transformed_rect_1() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let stroke = miter_stroke_2();
    ctx.transform(Affine::translate((10.0, 10.0)));
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_transformed_rect_1");
}

#[test]
fn stroked_transformed_rect_2() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(5.0, 5.0, 10.0, 10.0);
    let stroke = miter_stroke_2();
    ctx.transform(Affine::scale(2.0));
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_transformed_rect_2");
}

#[test]
fn stroked_transformed_rect_3() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let stroke = miter_stroke_2();
    ctx.transform(Affine::new([2.0, 0.0, 0.0, 2.0, 5.0, 5.0]));
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_transformed_rect_3");
}

#[test]
fn stroked_transformed_rect_4() {
    let mut ctx = get_ctx(30, 30, false);
    let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
    let stroke = miter_stroke_2();
    ctx.transform(Affine::rotate_about(
        45.0 * PI / 180.0,
        Point::new(15.0, 15.0),
    ));
    ctx.stroke_rect(&rect, &stroke, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "stroked_transformed_rect_4");
}

#[test]
fn strip_inscribed_rect() {
    let mut ctx = get_ctx(30, 20, false);
    let rect = Rect::new(1.5, 9.5, 28.5, 11.5);
    ctx.fill_rect(&rect, REBECCA_PURPLE.with_alpha(0.5).into());

    check_ref(&ctx, "strip_inscribed_rect");
}

fn miter_stroke_2() -> Stroke {
    Stroke {
        width: 2.0,
        join: Join::Miter,
        ..Default::default()
    }
}
