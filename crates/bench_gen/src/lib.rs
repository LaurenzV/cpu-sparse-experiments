use peniko::color::{AlphaColor, Rgba8, Srgb};
use peniko::kurbo::{Affine, BezPath, Point, Rect, Shape};
use rand::prelude::ThreadRng;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f64::consts::PI;

const SEED: [u8; 32] = [0; 32];

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
    FillPath(BezPath, AlphaColor<Srgb>),
    StrokePath(BezPath, AlphaColor<Srgb>),
}

pub struct RectAIterator {
    params: Params,
    rng: StdRng,
}

impl RectAIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for RectAIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let color = gen_color(&mut self.rng, 127);

        if self.params.stroke {
            Some(Command::StrokeRect(
                Rect::new(x, y, x + (size as f64), y + (size as f64)),
                color,
            ))
        }   else {
            Some(Command::FillRect(
                Rect::new(x, y, x + (size as f64), y + (size as f64)),
                color,
            ))
        }
    }
}

pub struct RectUIterator {
    params: Params,
    rng: StdRng,
}

impl RectUIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for RectUIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;

        let mut x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let mut y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let x_adjustment: f64 = self.rng.random();
        let y_adjustment: f64 = self.rng.random();

        x += x_adjustment;
        y += y_adjustment;

        let color = gen_color(&mut self.rng, 127);

        if self.params.stroke {
            Some(Command::StrokeRect(
                Rect::new(x, y, x + (size as f64), y + (size as f64)),
                color,
            ))
        }   else {
            Some(Command::FillRect(
                Rect::new(x, y, x + (size as f64), y + (size as f64)),
                color,
            ))
        }


    }
}

pub struct RectRotIterator {
    params: Params,
    angle: f64,
    rng: StdRng,
}

impl RectRotIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            angle: 0.0,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for RectRotIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let half_size = size as f64 / 2.0;

        let mut x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let mut y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let affine = Affine::rotate_about(
            self.angle * PI / 180.0,
            Point::new(x + half_size, y + half_size),
        );
        let color = gen_color(&mut self.rng, 127);
        let rect = Rect::new(x, y, x + (size as f64), y + (size as f64));

        self.angle += 0.01;

        if self.params.stroke {
            Some(Command::StrokePath(affine * rect.to_path(0.1), color))
        }   else {
            Some(Command::FillPath(affine * rect.to_path(0.1), color))
        }

    }
}

fn gen_color(rng: &mut StdRng, alpha: u8) -> AlphaColor<Srgb> {
    // Generate random color
    let r = rng.random_range(0..=255);
    let g = rng.random_range(0..=255);
    let b = rng.random_range(0..=255);

    AlphaColor::from_rgba8(r, g, b, alpha)
}
