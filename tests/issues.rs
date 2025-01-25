use crate::util::{check_ref, get_ctx};
use cpu_sparse::FillRule;
use peniko::color::palette;
use peniko::kurbo::{BezPath, Circle, Stroke};

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
    p.move_to((128.0, 128.0));
    p.line_to((160.0, 128.0));
    p.line_to((160.0, 160.0));
    p.line_to((128.0, 160.0));
    p.close_path();

    let mut ctx = get_ctx(64, 64, false);

    ctx.set_transform(Affine::translate((-112.0, -112.0)));
    ctx.fill(&p.into(), FillRule::NonZero, palette::css::LIME.into());

    check_ref(&ctx, "issue_2_incorrect_filling_2");
}

