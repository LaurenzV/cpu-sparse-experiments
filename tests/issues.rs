use crate::util::{check_ref, get_ctx};
use cpu_sparse::FillRule;
use peniko::color::palette;
use peniko::kurbo::{BezPath, Circle, Stroke};

mod util;

#[test]
fn issue_2_incorrect_filling() {
    let mut p = BezPath::default();
    p.move_to((8.0, 0.0));
    p.line_to((16.0, 8.0));
    p.line_to((8.0, 16.0));
    p.line_to((0.0, 8.0));
    p.close_path();

    let mut ctx = get_ctx(16, 16, false);

    ctx.fill(&p.into(), FillRule::NonZero, palette::css::LIME.into());

    check_ref(&ctx, "issue_2_incorrect_filling");
}
