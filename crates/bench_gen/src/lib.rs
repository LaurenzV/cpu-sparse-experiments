use peniko::color::{AlphaColor, Srgb};
use peniko::kurbo::{Affine, BezPath, Point, Rect, RoundedRectRadii, Shape};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f64::consts::PI;
use std::path::PathBuf;

const SEED: [u8; 32] = [0; 32];

#[derive(Clone, Copy)]
pub enum RectType {
    Aligned,
    Unaligned,
    Rotated,
    RoundUnaligned,
    RoundRotated,
}

impl RectType {
    pub fn is_rotated(&self) -> bool {
        matches!(self, RectType::Rotated | RectType::RoundRotated)
    }

    pub fn is_unaligned(&self) -> bool {
        matches!(self, RectType::Unaligned | RectType::RoundUnaligned)
    }

    pub fn is_rounded(&self) -> bool {
        matches!(self, RectType::RoundRotated | RectType::RoundUnaligned)
    }
}

#[derive(Copy, Clone)]
pub struct Params {
    pub width: usize,
    pub height: usize,
    pub stroke: bool,
    pub size: usize,
}

#[derive(Clone)]
pub enum Command {
    FillRect(Rect, AlphaColor<Srgb>),
    StrokeRect(Rect, AlphaColor<Srgb>),
    FillPath(BezPath, AlphaColor<Srgb>, bool),
    StrokePath(BezPath, AlphaColor<Srgb>),
}

pub struct RectIterator {
    params: Params,
    angle: f64,
    rect_type: RectType,
    color_iter: ColorIter,
    rng: StdRng,
}

impl RectIterator {
    pub fn new(params: Params, rect_type: RectType) -> Self {
        Self {
            params,
            angle: 0.0,
            rect_type,
            color_iter: ColorIter::new(false),
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for RectIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let radius = RoundedRectRadii::from_single_radius(self.rng.random_range(4.0..=40.0));
        let mut x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let mut y = self.rng.random_range(0..=(self.params.height - size)) as f64;
        let half_size = size as f64 / 2.0;

        if self.rect_type.is_unaligned() {
            let x_adjustment: f64 = self.rng.random();
            let y_adjustment: f64 = self.rng.random();

            x += x_adjustment;
            y += y_adjustment;
        }

        let color = self.color_iter.next().unwrap();
        let rect = Rect::new(x, y, x + (size as f64), y + (size as f64));

        if self.rect_type.is_rotated() {
            let affine = Affine::rotate_about(
                self.angle * PI / 180.0,
                Point::new(x + half_size, y + half_size),
            );

            self.angle += 0.01;

            if self.rect_type.is_rounded() {
                if self.params.stroke {
                    Some(Command::StrokePath(
                        affine * rect.to_rounded_rect(radius).to_path(0.1),
                        color,
                    ))
                } else {
                    Some(Command::FillPath(
                        affine * rect.to_rounded_rect(radius).to_path(0.1),
                        color,
                        true,
                    ))
                }
            } else {
                if self.params.stroke {
                    Some(Command::StrokePath(affine * rect.to_path(0.1), color))
                } else {
                    Some(Command::FillPath(affine * rect.to_path(0.1), color, true))
                }
            }
        } else {
            if self.rect_type.is_rounded() {
                if self.params.stroke {
                    Some(Command::StrokePath(
                        rect.to_rounded_rect(radius).to_path(0.1),
                        color,
                    ))
                } else {
                    Some(Command::FillPath(
                        rect.to_rounded_rect(radius).to_path(0.1),
                        color,
                        true,
                    ))
                }
            } else {
                if self.params.stroke {
                    Some(Command::StrokeRect(rect, color))
                } else {
                    Some(Command::FillRect(rect, color))
                }
            }
        }
    }
}

pub struct PolyIterator {
    params: Params,
    nz: bool,
    num_vertices: usize,
    color_iter: ColorIter,
    rng: StdRng,
}

impl PolyIterator {
    pub fn new(params: Params, num_vertices: usize, nz: bool) -> Self {
        Self {
            params,
            nz,
            num_vertices,
            color_iter: ColorIter::new(false),
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for PolyIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;

        let x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let mut path = BezPath::new();
        let mut move_to = false;

        for _ in 0..self.num_vertices {
            let xd = self.rng.random_range(0..=size) as f64;
            let yd = self.rng.random_range(0..=size) as f64;

            let point = Point::new(x + xd, y + yd);

            if !move_to {
                path.move_to(point);
                move_to = true;
            } else {
                path.line_to(point);
            }
        }

        let color = self.color_iter.next().unwrap();

        if self.params.stroke {
            Some(Command::StrokePath(path.into(), color))
        } else {
            Some(Command::FillPath(path.into(), color, self.nz))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ShapeKind {
    Butterfly,
    Dragon,
    Fish,
    World,
}

impl ShapeKind {
    fn name(&self) -> &'static str {
        match self {
            ShapeKind::Butterfly => "butterfly",
            ShapeKind::Dragon => "dragon",
            ShapeKind::Fish => "fish",
            ShapeKind::World => "world"
        }
    }
}

pub struct ShapeIterator {
    params: Params,
    color_iter: ColorIter,
    shape_path: BezPath,
    rng: StdRng,
}

impl ShapeIterator {
    pub fn new(params: Params, kind: ShapeKind) -> Self {
        let shape_path = load_shape_path(kind, params.size as f32 / 8.0);

        Self {
            params,
            shape_path,
            color_iter: ColorIter::new(false),
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for ShapeIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let transformed_path = Affine::translate((x, y)) * self.shape_path.clone();
        let color = self.color_iter.next().unwrap();

        if self.params.stroke {
            Some(Command::StrokePath(transformed_path, color))
        } else {
            Some(Command::FillPath(transformed_path, color, true))
        }
    }
}

fn load_shape_path(shape: ShapeKind, scale: f32) -> BezPath {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("assets/{}.txt", shape.name()));
    let bez_path = BezPath::from_svg(std::str::from_utf8(&std::fs::read(path).unwrap()).unwrap()).unwrap();
    let transform = Affine::scale(scale as f64);

    transform * bez_path
}

pub struct ColorIter {
    opaque: bool,
    rng: StdRng,
}

impl ColorIter {
    pub fn new(opaque: bool) -> Self {
        Self {
            opaque,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for ColorIter {
    type Item = AlphaColor<Srgb>;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.rng.random_range(0..=255);
        let g = self.rng.random_range(0..=255);
        let b = self.rng.random_range(0..=255);
        let a = if self.opaque {
            255
        } else {
            self.rng.random_range(0..255)
        };

        Some(AlphaColor::from_rgba8(r, g, b, a))
    }
}
