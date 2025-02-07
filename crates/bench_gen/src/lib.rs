use std::f64::consts::PI;
use peniko::color::{AlphaColor, Rgba8, Srgb};
use peniko::kurbo::{Affine, BezPath, Point, Rect, Shape};
use rand::prelude::ThreadRng;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const SEED: [u8; 32] = [0; 32];

#[derive(Copy, Clone)]
pub struct Params {
    width: usize,
    height: usize,
    size: usize,
}

pub enum Command {
    FillRect(Rect, AlphaColor<Srgb>),
    FillPath(BezPath, AlphaColor<Srgb>),
}

pub struct FillRectAIterator {
    params: Params,
    rng: StdRng,
}

impl FillRectAIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for FillRectAIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let color = gen_color(&mut self.rng, 127);

        Some(Command::FillRect(
            Rect::new(x, y, x + (size as f64), y + (size as f64)),
            color,
        ))
    }
}

pub struct FillRectUIterator {
    params: Params,
    rng: StdRng,
}

impl FillRectUIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for FillRectUIterator {
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

        Some(Command::FillRect(
            Rect::new(x, y, x + (size as f64), y + (size as f64)),
            color,
        ))
    }
}

pub struct FillRectRotIterator {
    params: Params,
    angle: f64,
    rng: StdRng,
}

impl FillRectRotIterator {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            angle: 0.0,
            rng: StdRng::from_seed(SEED),
        }
    }
}

impl Iterator for FillRectRotIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        let size = self.params.size;
        let half_size = size as f64 / 2.0;

        let mut x = self.rng.random_range(0..=(self.params.width - size)) as f64;
        let mut y = self.rng.random_range(0..=(self.params.height - size)) as f64;

        let affine = Affine::rotate_about(self.angle * PI/180.0, Point::new(x + half_size, y + half_size));
        let color = gen_color(&mut self.rng, 127);
        let rect = Rect::new(x, y, x + (size as f64), y + (size as f64));

        Some(Command::FillPath(
            affine * rect.to_path(0.1),
            color,
        ))
    }
}

fn gen_color(rng: &mut StdRng, alpha: u8) -> AlphaColor<Srgb> {
    // Generate random color
    let r = rng.random_range(0..=255);
    let g = rng.random_range(0..=255);
    let b = rng.random_range(0..=255);

    AlphaColor::from_rgba8(r, g, b, alpha)
}
