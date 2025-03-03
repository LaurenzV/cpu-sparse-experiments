use crate::util::{check_ref, get_ctx, render_pixmap};
use peniko::kurbo::{BezPath, Stroke};
use peniko::Fill;
use sparse_primitives::color::palette::css::{DARK_BLUE, LIME};

mod util;

#[test]
fn issue_2_incorrect_filling_1() {
    let mut p = BezPath::default();
    p.move_to((4.0, 0.0));
    p.line_to((8.0, 4.0));
    p.line_to((4.0, 8.0));
    p.line_to((0.0, 4.0));
    p.close_path();

    let mut ctx = get_ctx(8, 8, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&p.into());

    check_ref(&ctx, "issue_2_incorrect_filling_1");
}

#[test]
fn issue_2_incorrect_filling_2() {
    let mut p = BezPath::default();
    p.move_to((16.0, 16.0));
    p.line_to((48.0, 16.0));
    p.line_to((48.0, 48.0));
    p.line_to((16.0, 48.0));
    p.close_path();

    let mut ctx = get_ctx(64, 64, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&p.into());

    check_ref(&ctx, "issue_2_incorrect_filling_2");
}

#[test]
fn issue_2_incorrect_filling_3() {
    let mut path = BezPath::new();
    path.move_to((4.00001, 1e-45));
    path.line_to((8.00001, 4.00001));
    path.line_to((4.00001, 8.00001));
    path.line_to((1e-45, 4.00001));
    path.close_path();

    let mut ctx = get_ctx(9, 9, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_3");
}

#[test]
fn issue_2_incorrect_filling_4() {
    let mut path = BezPath::new();
    path.move_to((16.000002, 8.));
    path.line_to((20.000002, 8.));
    path.line_to((24.000002, 8.));
    path.line_to((28.000002, 8.));
    path.line_to((32.000002, 8.));
    path.line_to((32.000002, 9.));
    path.line_to((28.000002, 9.));
    path.line_to((24.000002, 9.));
    path.line_to((20.000002, 9.));
    path.line_to((16.000002, 9.));
    path.close_path();

    let mut ctx = get_ctx(64, 64, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_4");
}

#[test]
fn issue_2_incorrect_filling_5() {
    let mut path = BezPath::new();
    path.move_to((16., 8.));
    path.line_to((16., 9.));
    path.line_to((32., 9.));
    path.line_to((32., 8.));
    path.close_path();

    let mut ctx = get_ctx(32, 32, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_5");
}

#[test]
fn issue_2_incorrect_filling_6() {
    let mut path = BezPath::new();
    path.move_to((16., 8.));
    path.line_to((31.999998, 8.));
    path.line_to((31.999998, 9.));
    path.line_to((16., 9.));
    path.close_path();

    let mut ctx = get_ctx(32, 32, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_6");
}

#[test]
fn issue_2_incorrect_filling_7() {
    let mut path = BezPath::new();
    path.move_to((32.000002, 9.));
    path.line_to((28., 9.));
    path.line_to((28., 8.));
    path.line_to((32.000002, 8.));
    path.close_path();

    let mut ctx = get_ctx(32, 32, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_7");
}

#[test]
fn issue_2_incorrect_filling_8() {
    let mut path = BezPath::new();
    path.move_to((16.000427, 8.));
    path.line_to((20.000427, 8.));
    path.line_to((24.000427, 8.));
    path.line_to((28.000427, 8.));
    path.line_to((32.000427, 8.));
    path.line_to((32.000427, 9.));
    path.line_to((28.000427, 9.));
    path.line_to((24.000427, 9.));
    path.line_to((20.000427, 9.));
    path.line_to((16.000427, 9.));
    path.close_path();

    let mut ctx = get_ctx(32, 32, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_2_incorrect_filling_8");
}

#[test]
fn issue_11_out_of_bound_strip() {
    let mut path = BezPath::new();
    path.move_to((258.0, 254.0));
    path.line_to((265.0, 254.0));
    let stroke = Stroke::new(1.0);

    let mut ctx = get_ctx(256, 256, true);

    ctx.set_paint(DARK_BLUE.into());
    ctx.set_stroke(stroke);
    // Just make sure we don't panic.
    ctx.stroke_path(&path.into());
}

#[test]
fn issue_12_filling_unclosed_path_1() {
    let mut path = BezPath::new();
    path.move_to((75.0, 25.0));
    path.line_to((25.0, 25.0));
    path.line_to((25.0, 75.0));
    let mut ctx = get_ctx(100, 100, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_12_filling_unclosed_path_1");
}

#[test]
fn issue_12_filling_unclosed_path_2() {
    let mut path = BezPath::new();
    path.move_to((50.0, 0.0));
    path.line_to((0.0, 0.0));
    path.line_to((0.0, 50.0));

    path.move_to((50.0, 50.0));
    path.line_to((100.0, 50.0));
    path.line_to((100.0, 100.0));
    path.line_to((50.0, 100.0));
    path.close_path();

    let mut ctx = get_ctx(100, 100, false);

    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_12_filling_unclosed_path_2");
}

#[test]
fn issue_28_triangle_exceeding_viewport_1() {
    let mut path = BezPath::new();
    path.move_to((5.0, 0.0));
    path.line_to((12.0, 7.99));
    path.line_to((-4.0, 7.99));
    path.close_path();

    let mut ctx = get_ctx(15, 8, false);

    ctx.set_fill_rule(Fill::EvenOdd);
    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_28_triangle_exceeding_viewport_1");
}

#[test]
fn issue_28_triangle_exceeding_viewport_2() {
    let mut path = BezPath::new();
    path.move_to((4.0, 0.0));
    path.line_to((11.0, 7.99));
    path.line_to((-5.0, 7.99));
    path.close_path();

    let mut ctx = get_ctx(15, 8, false);

    ctx.set_fill_rule(Fill::EvenOdd);
    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_28_triangle_exceeding_viewport_2");
}

#[test]
fn issue_30_shape_at_wide_tile_boundary() {
    let mut path = BezPath::new();
    path.move_to((248.0, 0.0));
    path.line_to((257.0, 0.0));
    path.line_to((257.0, 2.0));
    path.line_to((248.0, 2.0));
    path.close_path();

    let mut ctx = get_ctx(256, 4, false);

    ctx.set_fill_rule(Fill::EvenOdd);
    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    // Just make sure we don't panic.
    render_pixmap(&ctx);
}

#[test]
fn issue_eo_filling_missing_anti_aliasing() {
    let mut path = BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((50.0, 50.0));
    path.line_to((0.0, 50.0));
    path.line_to((50.0, 0.0));
    path.close_path();

    let mut ctx = get_ctx(50, 50, false);

    ctx.set_fill_rule(Fill::EvenOdd);
    ctx.set_paint(LIME.into());
    ctx.fill_path(&path.into());

    check_ref(&ctx, "issue_eo_filling_missing_anti_aliasing");
}
