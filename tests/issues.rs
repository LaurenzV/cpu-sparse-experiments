use crate::util::{check_ref, get_ctx};
use cpu_sparse::FillRule;
use peniko::color::palette;
use peniko::kurbo::{Affine, BezPath, Circle, Stroke};

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

    ctx.fill(&p.into(), FillRule::NonZero, palette::css::LIME.into());

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

    ctx.fill(&p.into(), FillRule::NonZero, palette::css::LIME.into());

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

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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
    path.move_to((16.000002, 9.));
    path.close_path();

    let mut ctx = get_ctx(64, 64, false);

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

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
    path.move_to((16.000427, 9.));
    path.close_path();

    let mut ctx = get_ctx(32, 32, false);

    ctx.fill(&path.into(), FillRule::NonZero, palette::css::LIME.into());

    check_ref(&ctx, "issue_2_incorrect_filling_8");
}


