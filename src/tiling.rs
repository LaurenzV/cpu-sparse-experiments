// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::strip::Tile;

pub const TILE_WIDTH: u32 = 4;
pub const TILE_HEIGHT: u32 = 4;

const TILE_SCALE_X: f32 = 1.0 / TILE_WIDTH as f32;
const TILE_SCALE_Y: f32 = 1.0 / TILE_HEIGHT as f32;

/// This is just Line but f32
#[derive(Clone, Copy, Debug)]
pub struct FlatLine {
    // should these be vec2?
    pub p0: Point,
    pub p1: Point,
}

impl FlatLine {
    pub fn new(p0: Point, p1: Point) -> Self {
        Self { p0, p1 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PackedPoint {
    x: u16,
    y: u16,
}

impl PackedPoint {
    pub fn new(x: u16, y: u16) -> Self {
        PackedPoint { x, y }
    }

    pub fn unpack(&self) -> Point {
        let x = self.unpacked_x();
        let y = self.unpacked_y();

        Point::new(x, y)
    }

    pub fn packed_x(&self) -> u16 {
        self.x
    }

    pub fn packed_y(&self) -> u16 {
        self.y
    }

    pub fn unpacked_x(&self) -> f32 {
        self.x as f32 * (1.0 / TILE_SCALE)
    }

    pub fn unpacked_y(&self) -> f32 {
        self.y as f32 * (1.0 / TILE_SCALE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn pack(&self) -> PackedPoint {
        let x = (self.x * TILE_SCALE).round() as u16;
        let y = (self.y * TILE_SCALE).round() as u16;

        PackedPoint { x, y }
    }
}

const TILE_SCALE: f32 = 8192.0;
// scale factor relative to unit square in tile
const FRAC_TILE_SCALE: f32 = 8192.0 * 4.0;

pub(crate) fn scale_up(z: f32) -> u16 {
    (z * FRAC_TILE_SCALE).round() as u16
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    fn from_array(xy: [f32; 2]) -> Self {
        Point::new(xy[0], xy[1])
    }
}

impl std::ops::Add for Point {
    type Output = Self;

    fn add(self, rhs: Point) -> Self {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Point) -> Self {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

fn span(a: f32, b: f32) -> u32 {
    (a.max(b).ceil() - a.min(b).floor()).max(1.0) as u32
}

pub fn make_tiles(lines: &[FlatLine], tile_buf: &mut Vec<Tile>) {
    // Lines that cross vertical tile boundaries need special treatment during
    // anti aliasing. This case is detected via tile-relative x == 0. However,
    // lines can naturally start or end at a multiple of the 4x4 grid, too, but
    // these don't constitute crossings. We nudge these points ever so slightly,
    // by ensuring that xfrac0 and xfrac1 are always at least 1, which
    // corresponds to 1/8192 of a pixel.
    //
    // Note that we cannot check if line.p0.x or s0.x modulo 4 equal 0 because
    // those values operate at higher precision than xfrac values. For example,
    // the coordinate 4.00001 % 4 != 0, but after scaleUp, xfrac == 0.

    tile_buf.clear();
    for line in lines {
        let p0 = line.p0;
        let p1 = line.p1;
        let s0 = p0 * TILE_SCALE_X;
        let s1 = p1 * TILE_SCALE_Y;
        let count_x = span(s0.x, s1.x);
        let count_y = span(s0.y, s1.y);
        let mut x = s0.x.floor();
        if s0.x == x && s1.x < x {
            // s0.x is on right side of first tile
            x -= 1.0;
        }
        let mut y = s0.y.floor();
        if s0.y == y && s1.y < y {
            // s0.y is on bottom of first tile
            y -= 1.0;
        }
        let xfrac0 = scale_up(s0.x - x).max(1);
        let yfrac0 = scale_up(s0.y - y);
        let packed0 = PackedPoint::new(xfrac0, yfrac0);
        // These could be replaced with <2 and the max(1.0) in span removed
        if count_x == 1 {
            let xfrac1 = scale_up(s1.x - x).max(1);
            if count_y == 1 {
                let yfrac1 = scale_up(s1.y - y);

                // 1x1 tile
                tile_buf.push(Tile {
                    x: x as u16,
                    y: y as u16,
                    p0: PackedPoint::new(xfrac0, yfrac0),
                    p1: PackedPoint::new(xfrac1, yfrac1),
                });
            } else {
                // vertical column
                let slope = (s1.x - s0.x) / (s1.y - s0.y);
                let sign = (s1.y - s0.y).signum();
                let mut xclip0 = (s0.x - x) + (y - s0.y) * slope;
                let yclip = if sign > 0.0 {
                    xclip0 += slope;
                    scale_up(1.0)
                } else {
                    0
                };
                let mut last_packed = packed0;
                for i in 0..count_y - 1 {
                    let xclip = xclip0 + i as f32 * sign * slope;
                    let xfrac = scale_up(xclip).max(1);
                    let packed = PackedPoint::new(xfrac, yclip);
                    tile_buf.push(Tile {
                        x: x as u16,
                        y: (y + i as f32 * sign) as u16,
                        p0: last_packed,
                        p1: packed,
                    });
                    // flip y between top and bottom of tile
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                }
                let yfrac1 = scale_up(s1.y - (y + (count_y - 1) as f32 * sign));
                let packed1 = PackedPoint::new(xfrac1, yfrac1);

                tile_buf.push(Tile {
                    x: x as u16,
                    y: (y + (count_y - 1) as f32 * sign) as u16,
                    p0: last_packed,
                    p1: packed1,
                });
            }
        } else if count_y == 1 {
            // horizontal row
            let slope = (s1.y - s0.y) / (s1.x - s0.x);
            let sign = (s1.x - s0.x).signum();
            let mut yclip0 = (s0.y - y) + (x - s0.x) * slope;
            let xclip = if sign > 0.0 {
                yclip0 += slope;
                scale_up(1.0)
            } else {
                0
            };
            let mut last_packed = packed0;
            for i in 0..count_x - 1 {
                let yclip = yclip0 + i as f32 * sign * slope;
                let yfrac = scale_up(yclip).max(1);
                let packed = PackedPoint::new(xclip, yfrac);
                tile_buf.push(Tile {
                    x: (x + i as f32 * sign) as u16,
                    y: y as u16,
                    p0: last_packed,
                    p1: packed,
                });
                // flip x between left and right of tile
                last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);
            }
            let xfrac1 = scale_up(s1.x - (x + (count_x - 1) as f32 * sign)).max(1);
            let yfrac1 = scale_up(s1.y - y);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            tile_buf.push(Tile {
                x: (x + (count_x - 1) as f32 * sign) as u16,
                y: y as u16,
                p0: last_packed,
                p1: packed1,
            });
        } else {
            // general case
            let recip_dx = 1.0 / (s1.x - s0.x);
            let signx = (s1.x - s0.x).signum();
            let recip_dy = 1.0 / (s1.y - s0.y);
            let signy = (s1.y - s0.y).signum();
            // t parameter for next intersection with a vertical grid line
            let mut t_clipx = (x - s0.x) * recip_dx;
            let xclip = if signx > 0.0 {
                t_clipx += recip_dx;
                scale_up(1.0)
            } else {
                0
            };
            // t parameter for next intersection with a horizontal grid line
            let mut t_clipy = (y - s0.y) * recip_dy;
            let yclip = if signy > 0.0 {
                t_clipy += recip_dy;
                scale_up(1.0)
            } else {
                0
            };
            let x1 = x + (count_x - 1) as f32 * signx;
            let y1 = y + (count_y - 1) as f32 * signy;
            let mut xi = x;
            let mut yi = y;
            let mut last_packed = packed0;
            let mut count = 0;
            while xi != x1 || yi != y1 {
                count += 1;

                if t_clipy < t_clipx {
                    // intersected with horizontal grid line
                    let x_intersect = s0.x + (s1.x - s0.x) * t_clipy - xi;
                    let xfrac = scale_up(x_intersect).max(1); // maybe should clamp?
                    let packed = PackedPoint::new(xfrac, yclip);
                    tile_buf.push(Tile {
                        x: xi as u16,
                        y: yi as u16,
                        p0: last_packed,
                        p1: packed,
                    });
                    t_clipy += recip_dy.abs();
                    yi += signy;
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                } else {
                    // intersected with vertical grid line
                    let y_intersect = s0.y + (s1.y - s0.y) * t_clipx - yi;
                    let yfrac = scale_up(y_intersect).max(1); // maybe should clamp?
                    let packed = PackedPoint::new(xclip, yfrac);
                    tile_buf.push(Tile {
                        x: xi as u16,
                        y: yi as u16,
                        p0: last_packed,
                        p1: packed,
                    });
                    t_clipx += recip_dx.abs();
                    xi += signx;
                    last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);
                }
            }
            let xfrac1 = scale_up(s1.x - xi).max(1);
            let yfrac1 = scale_up(s1.y - yi);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            tile_buf.push(Tile {
                x: xi as u16,
                y: yi as u16,
                p0: last_packed,
                p1: packed1,
            });
        }
    }
    // This particular choice of sentinel tiles generates a sentinel strip.
    tile_buf.push(Tile {
        x: 0x3ffd,
        y: 0x3fff,
        p0: PackedPoint::new(0, 0),
        p1: PackedPoint::new(0, 0),
    });
    tile_buf.push(Tile {
        x: 0x3fff,
        y: 0x3fff,
        p0: PackedPoint::new(0, 0),
        p1: PackedPoint::new(0, 0),
    });
}

#[test]
fn tiling() {
    let l = FlatLine {
        p0: Point::new(1.3, 1.4),
        p1: Point::new(20.1, 50.2),
    };
    let mut tiles = vec![];
    make_tiles(&[l], &mut tiles);
    for tile in &tiles {
        let p0 = tile.p0.unpack();
        let p1 = tile.p1.unpack();
        println!(
            "@{}, {}: ({}, {}) - ({}, {})",
            tile.x, tile.y, p0.x, p0.y, p1.x, p1.y
        );
    }
}
