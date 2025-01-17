use tiny_skia::{Color, FillRule, Pixmap};
use usvg::{Node, Paint, PaintOrder, Transform};

fn main() {
    let scale = 10.0 / 9.0;
    let svg = std::fs::read_to_string("svgs/gs.svg").expect("error reading file");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let width = (tree.size().width() * scale).ceil() as usize;
    let height = (tree.size().height() * scale).ceil() as usize;

    let mut sctx = SVGContext::new_with_scale(scale as f64);

    let num_iters = 200;
    let mut pix = None;

    // Hacky code for crude measurements; change this to arg parsing
    let start = std::time::Instant::now();
    for _ in 0..num_iters {
        let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();
        render_tree(&mut pixmap.as_mut(), &mut sctx, &tree);

        pix = Some(pixmap);
    }

    let end = start.elapsed();
    println!("{:?}ms", end.as_millis() as f32 / num_iters as f32);

    pix.unwrap().save_png("tiny_skia.png").unwrap();
}

pub struct SVGContext {
    transforms: Vec<Transform>,
}

impl SVGContext {
    pub fn new() -> Self {
        Self {
            transforms: vec![Transform::identity()],
        }
    }

    pub fn new_with_scale(scale: f64) -> Self {
        Self {
            transforms: vec![Transform::from_scale(scale as f32, scale as f32)],
        }
    }

    pub fn push_transform(&mut self, affine: &Transform) {
        let new = self.transforms.last().unwrap().pre_concat(*affine);
        self.transforms.push(new);
    }

    pub fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    pub fn get_transform(&self) -> Transform {
        *self.transforms.last().unwrap()
    }
}

pub fn render_tree(pixmap: &mut tiny_skia::PixmapMut, sctx: &mut SVGContext, tree: &usvg::Tree) {
    render_group(pixmap, sctx, tree.root());
}

fn render_group(pixmap: &mut tiny_skia::PixmapMut, sctx: &mut SVGContext, group: &usvg::Group) {
    sctx.push_transform(&group.transform());

    for child in group.children() {
        match child {
            Node::Group(g) => {
                render_group(pixmap, sctx, g);
            }
            Node::Path(p) => {
                render_path(pixmap, sctx, p);
            }
            Node::Image(_) => {}
            Node::Text(_) => {}
        }
    }

    sctx.pop_transform();
}

fn render_path(pixmap: &mut tiny_skia::PixmapMut, sctx: &mut SVGContext, path: &usvg::Path) {
    if !path.is_visible() {
        return;
    }

    let fill = |pixmap: &mut tiny_skia::PixmapMut, path: &usvg::Path| {
        if let Some(fill) = path.fill() {
            let color = match fill.paint() {
                Paint::Color(c) => {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(c.red, c.green, c.blue, fill.opacity().to_u8());

                    paint
                }
                _ => return,
            };

            pixmap.fill_path(
                path.data(),
                &color,
                convert_fill_rule(fill.rule()),
                sctx.get_transform(),
                None,
            );
        }
    };

    let stroke = |pixmap: &mut tiny_skia::PixmapMut, path: &usvg::Path| {
        if let Some(stroke) = path.stroke() {
            let color = match stroke.paint() {
                Paint::Color(c) => {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(c.red, c.green, c.blue, stroke.opacity().to_u8());

                    paint
                }
                _ => return,
            };

            let stroke = tiny_skia::Stroke {
                width: stroke.width().get(),
                miter_limit: 0.0,
                line_cap: Default::default(),
                line_join: Default::default(),
                dash: None,
            };

            pixmap.stroke_path(path.data(), &color, &stroke, sctx.get_transform(), None);
        }
    };

    if path.paint_order() == PaintOrder::FillAndStroke {
        fill(pixmap, path);
        stroke(pixmap, path);
    } else {
        stroke(pixmap, path);
        fill(pixmap, path);
    }
}

fn convert_fill_rule(fill_rule: usvg::FillRule) -> FillRule {
    match fill_rule {
        usvg::FillRule::NonZero => FillRule::Winding,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}
