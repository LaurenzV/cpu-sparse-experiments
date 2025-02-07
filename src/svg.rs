use crate::render::Path;
use crate::{FillRule, Pixmap, RenderContext};
use peniko::color::AlphaColor;
use peniko::kurbo::{Affine, BezPath, Stroke};
use usvg::tiny_skia_path::PathSegment;
use usvg::{ImageKind, Node, Paint, PaintOrder};

pub struct SVGContext {
    transforms: Vec<Affine>,
}

impl SVGContext {
    pub fn new() -> Self {
        Self {
            transforms: vec![Affine::IDENTITY],
        }
    }

    pub fn new_with_scale(scale: f64) -> Self {
        Self {
            transforms: vec![Affine::scale(scale)],
        }
    }

    pub fn push_transform(&mut self, affine: &Affine) {
        let new = *self.transforms.last().unwrap() * *affine;
        self.transforms.push(new);
    }

    pub fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    pub fn get_transform(&self) -> Affine {
        *self.transforms.last().unwrap()
    }
}

pub fn render_tree(ctx: &mut RenderContext, sctx: &mut SVGContext, tree: &usvg::Tree) {
    render_group(ctx, sctx, tree.root());
}

fn render_group(ctx: &mut RenderContext, sctx: &mut SVGContext, group: &usvg::Group) {
    sctx.push_transform(&convert_transform(&group.transform()));

    for child in group.children() {
        match child {
            Node::Group(g) => {
                render_group(ctx, sctx, g);
            }
            Node::Path(p) => {
                render_path(ctx, sctx, p);
            }
            Node::Image(i) => render_image(ctx, sctx, i),
            Node::Text(_) => {}
        }
    }

    sctx.pop_transform();
}

fn render_image(ctx: &mut RenderContext, sctx: &mut SVGContext, image: &usvg::Image) {
    let pixmap = match image.kind() {
        ImageKind::JPEG(_) => unimplemented!(),
        ImageKind::PNG(i) => Pixmap::from_png(i).unwrap(),
        ImageKind::GIF(_) => unimplemented!(),
        ImageKind::WEBP(_) => unimplemented!(),
        ImageKind::SVG(_) => unimplemented!(),
    };
}

fn render_path(ctx: &mut RenderContext, sctx: &mut SVGContext, path: &usvg::Path) {
    if !path.is_visible() {
        return;
    }

    ctx.set_transform(sctx.get_transform());

    let fill = |ctx: &mut RenderContext, path: &usvg::Path| {
        if let Some(fill) = path.fill() {
            let color = match fill.paint() {
                Paint::Color(c) => {
                    AlphaColor::from_rgba8(c.red, c.green, c.blue, fill.opacity().to_u8())
                }
                _ => return,
            };

            ctx.fill_path(
                &convert_path_data(path),
                convert_fill_rule(fill.rule()),
                color.into(),
            );
        }
    };

    let stroke = |ctx: &mut RenderContext, path: &usvg::Path| {
        if let Some(stroke) = path.stroke() {
            let color = match stroke.paint() {
                Paint::Color(c) => {
                    AlphaColor::from_rgba8(c.red, c.green, c.blue, stroke.opacity().to_u8())
                }
                _ => return,
            };

            let stroke = Stroke::new(stroke.width().get() as f64);

            ctx.stroke_path(&convert_path_data(path), &stroke, color.into());
        }
    };

    if path.paint_order() == PaintOrder::FillAndStroke {
        fill(ctx, path);
        stroke(ctx, path);
    } else {
        stroke(ctx, path);
        fill(ctx, path);
    }
}

fn convert_fill_rule(fill_rule: usvg::FillRule) -> FillRule {
    match fill_rule {
        usvg::FillRule::NonZero => FillRule::NonZero,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

fn convert_transform(transform: &usvg::Transform) -> Affine {
    Affine::new([
        transform.sx as f64,
        transform.ky as f64,
        transform.kx as f64,
        transform.sy as f64,
        transform.tx as f64,
        transform.ty as f64,
    ])
}

fn convert_path_data(path: &usvg::Path) -> Path {
    let mut bez_path = BezPath::new();

    for e in path.data().segments() {
        match e {
            PathSegment::MoveTo(p) => {
                bez_path.move_to((p.x, p.y));
            }
            PathSegment::LineTo(p) => {
                bez_path.line_to((p.x, p.y));
            }
            PathSegment::QuadTo(p1, p2) => {
                bez_path.quad_to((p1.x, p1.y), (p2.x, p2.y));
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                bez_path.curve_to((p1.x, p1.y), (p2.x, p2.y), (p3.x, p3.y));
            }
            PathSegment::Close => {
                bez_path.close_path();
            }
        }
    }

    bez_path.into()
}
