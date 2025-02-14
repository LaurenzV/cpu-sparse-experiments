// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::{Debug, Formatter};

pub const TILE_WIDTH: u32 = 4;
pub const TILE_HEIGHT: u32 = 4;

const TILE_SCALE_X: f32 = 1.0 / TILE_WIDTH as f32;
const TILE_SCALE_Y: f32 = 1.0 / TILE_HEIGHT as f32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loc {
    // TODO: Benchmark i16 vs i32.
    pub x: i32,
    // In practice will always be positive since we can (and also do) just ignore tiles where y < 0,
    // but the same does not apply for x, where we do need to preserve tiles where x < 0 (so that
    // filling works correctly).
    pub y: i32,
}

impl Loc {
    pub fn zero() -> Self {
        Loc { x: 0, y: 0 }
    }
    /// Check whether two locations are on the same strip. This is the case if they are in the same
    /// row and right next to each other.
    pub(crate) fn same_strip(&self, other: &Self) -> bool {
        self.same_row(other) && (other.x - self.x).abs() <= 1
    }

    pub(crate) fn same_row(&self, other: &Self) -> bool {
        self.y == other.y
    }

    pub(crate) fn cmp(&self, b: &Loc) -> std::cmp::Ordering {
        (self.y, self.x).cmp(&(b.y, b.x))
    }
}

/// A footprint represents in a compact fashion the range of pixels covered by a tile.
/// We represent this as a u32 so that we can work with bit-shifting for better performance.
pub(crate) struct Footprint(pub(crate) u32);

impl Footprint {
    /// Create a new, empty footprint.
    pub(crate) fn empty() -> Footprint {
        Footprint(0)
    }

    /// Create a new footprint from a single index, i.e. [i, i + 1).
    pub(crate) fn with_index(index: u8) -> Footprint {
        Footprint(1 << index)
    }

    /// Create a new footprint from a single index, i.e. [start, end).
    pub(crate) fn with_range(start: u8, end: u8) -> Footprint {
        Footprint((1 << end) - (1 << start))
    }

    /// The start point of the covered range (inclusive).
    pub(crate) fn x0(&self) -> u32 {
        self.0.trailing_zeros()
    }

    /// The end point of the covered range (exclusive).
    pub(crate) fn x1(&self) -> u32 {
        32 - self.0.leading_zeros()
    }

    /// Add a new index to the range.
    pub(crate) fn add(&mut self, index: u8) {
        self.0 |= (1 << index) as u32;
    }

    /// Merge another footprint with the current one.
    pub(crate) fn merge(&mut self, fp: &Footprint) {
        self.0 |= fp.0;
    }
}

/// A tile represents an aligned area on the pixmap, used to subdivide the viewport into sub-areas
/// (currently 4x4) and analyze line intersections inside each such area.
///
/// Keep in mind that it is possible to have multiple tiles with the same index,
/// namely if we have multiple lines crossing the same 4x4 area!
///
/// We are storing the points in a packed fashion in order to (TODO: find out).
#[derive(Debug)]
pub struct Tile {
    /// The index of the tile in the x- and y-direction.
    loc: Loc,
    /// The start point of the line in that tile.
    p0: PackedPoint,
    /// The end point of the line in that tile.
    p1: PackedPoint,
}

impl Tile {
    pub fn new(x: i32, y: i32, p0: PackedPoint, p1: PackedPoint) -> Self {
        Self {
            loc: Loc { x, y },
            p0,
            p1,
        }
    }

    pub fn new_u16(x: u16, y: u16, p0: PackedPoint, p1: PackedPoint) -> Self {
        Self {
            loc: Loc {
                x: x as i32,
                y: y as i32,
            },
            p0,
            p1,
        }
    }

    pub fn p0(&self) -> PackedPoint {
        self.p0
    }

    pub fn p1(&self) -> PackedPoint {
        self.p1
    }

    pub fn x(&self) -> i32 {
        self.loc.x
    }

    pub fn y(&self) -> i32 {
        self.loc.y
    }

    pub(crate) fn loc(&self) -> Loc {
        self.loc
    }

    pub(crate) fn footprint(&self) -> Footprint {
        let x0 = self.p0.unpacked_x();
        let x1 = self.p1.unpacked_x();
        let x_min = x0.min(x1);
        let x_max = x0.max(x1);
        // On CPU, might be better to do this as fixed point
        let start_i = x_min.floor() as u32;
        let end_i = (start_i + 1).max(x_max.ceil() as u32).min(TILE_WIDTH);
        Footprint((1 << end_i) - (1 << start_i))
    }

    pub(crate) fn delta(&self) -> i32 {
        (self.p1.packed_y() == 0) as i32 - (self.p0.packed_y() == 0) as i32
    }

    // TODO: Verify that this is efficient.
    pub fn cmp(&self, b: &Tile) -> std::cmp::Ordering {
        self.loc.cmp(&b.loc)
    }
}

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

#[derive(Clone, Copy)]
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

impl Debug for PackedPoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.unpacked_x(), self.unpacked_y())
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
    tile_buf.clear();

    let round = |f: f32| -> f32 {
        // Round to the same resolution as used by our u16 representation
        // (see scale_up). This avoids discrepancies between the f32 and
        // u16 values when checking for alignment with the tile grid.
        //
        // We round just the fractional part to avoid precision issues for large
        // coordinates.)
        let i = f.trunc();
        let frac = f.fract();
        i + (frac * FRAC_TILE_SCALE).round() / FRAC_TILE_SCALE
    };
    let round_point = |p: Point| -> Point {
        Point {
            x: round(p.x),
            y: round(p.y),
        }
    };
    let nudge_point = |p: Point| -> Point {
        // Lines that cross vertical tile boundaries need special treatment during
        // anti aliasing. This case is detected via tile-relative x == 0. However,
        // lines can naturally start or end at a multiple of the 4x4 grid, too, but
        // these don't constitute crossings. We nudge these points ever so slightly,
        // by ensuring that xfrac0 and xfrac1 are always at least 1, which
        // corresponds to 1/8192 of a pixel.

        if p.x.fract() == 0.0 {
            Point {
                x: p.x + 1.0 / FRAC_TILE_SCALE,
                y: p.y,
            }
        } else {
            p
        }
    };

    let mut push_tile = |tile: Tile| {
        if tile.y() >= 0 {
            tile_buf.push(tile);
        }
    };

    for line in lines {
        let p0 = line.p0;
        let p1 = line.p1;
        let s0 = nudge_point(round_point(p0 * TILE_SCALE_X));
        let s1 = nudge_point(round_point(p1 * TILE_SCALE_Y));
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
        let xfrac0 = scale_up(s0.x - x);
        let yfrac0 = scale_up(s0.y - y);
        let packed0 = PackedPoint::new(xfrac0, yfrac0);
        // These could be replaced with <2 and the max(1.0) in span removed
        if count_x == 1 {
            let xfrac1 = scale_up(s1.x - x);
            if count_y == 1 {
                let yfrac1 = scale_up(s1.y - y);

                // 1x1 tile
                push_tile(Tile::new(
                    x as i32,
                    y as i32,
                    PackedPoint::new(xfrac0, yfrac0),
                    PackedPoint::new(xfrac1, yfrac1),
                ));
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
                    push_tile(Tile::new(
                        x as i32,
                        (y + i as f32 * sign) as i32,
                        last_packed,
                        packed,
                    ));
                    // flip y between top and bottom of tile
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                }
                let yfrac1 = scale_up(s1.y - (y + (count_y - 1) as f32 * sign));
                let packed1 = PackedPoint::new(xfrac1, yfrac1);

                push_tile(Tile::new(
                    x as i32,
                    (y + (count_y - 1) as f32 * sign) as i32,
                    last_packed,
                    packed1,
                ));
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
                push_tile(Tile::new(
                    (x + i as f32 * sign) as i32,
                    y as i32,
                    last_packed,
                    packed,
                ));
                // flip x between left and right of tile
                last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);
            }
            let xfrac1 = scale_up(s1.x - (x + (count_x - 1) as f32 * sign));
            let yfrac1 = scale_up(s1.y - y);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            push_tile(Tile::new(
                (x + (count_x - 1) as f32 * sign) as i32,
                y as i32,
                last_packed,
                packed1,
            ));
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
                    push_tile(Tile::new(xi as i32, yi as i32, last_packed, packed));
                    t_clipy += recip_dy.abs();
                    yi += signy;
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                } else {
                    // intersected with vertical grid line
                    let y_intersect = s0.y + (s1.y - s0.y) * t_clipx - yi;
                    let yfrac = scale_up(y_intersect).max(1); // maybe should clamp?
                    let packed = PackedPoint::new(xclip, yfrac);
                    push_tile(Tile::new(xi as i32, yi as i32, last_packed, packed));
                    t_clipx += recip_dx.abs();
                    xi += signx;
                    last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);
                }
            }
            let xfrac1 = scale_up(s1.x - xi);
            let yfrac1 = scale_up(s1.y - yi);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            push_tile(Tile::new(xi as i32, yi as i32, last_packed, packed1));
        }
    }
    // This particular choice of sentinel tiles generates a sentinel strip.
    push_tile(Tile::new_u16(
        0x3ffd,
        0x3fff,
        PackedPoint::new(0, 0),
        PackedPoint::new(0, 0),
    ));
    push_tile(Tile::new_u16(
        0x3fff,
        0x3fff,
        PackedPoint::new(0, 0),
        PackedPoint::new(0, 0),
    ));

    println!("{:#?}", tile_buf);
}

#[cfg(test)]
mod tests {
    use crate::tiling::{make_tiles, scale_up, FlatLine, Footprint, Loc, PackedPoint, Point, Tile};

    #[test]
    fn footprint_empty() {
        let fp1 = Footprint::empty();
        // Not optimal behavior, but currently how it is.
        assert_eq!(fp1.x0(), 32);
        assert_eq!(fp1.x1(), 0);
    }

    #[test]
    fn footprint_with_index() {
        let fp1 = Footprint::with_index(0);
        assert_eq!(fp1.x0(), 0);
        assert_eq!(fp1.x1(), 1);

        let fp2 = Footprint::with_index(3);
        assert_eq!(fp2.x0(), 3);
        assert_eq!(fp2.x1(), 4);

        let fp3 = Footprint::with_index(6);
        assert_eq!(fp3.x0(), 6);
        assert_eq!(fp3.x1(), 7);
    }

    #[test]
    fn footprint_from_range() {
        let fp1 = Footprint::with_range(1, 3);
        assert_eq!(fp1.x0(), 1);
        assert_eq!(fp1.x1(), 3);

        // Same comment as for empty.
        let fp2 = Footprint::with_range(2, 2);
        assert_eq!(fp2.x0(), 32);
        assert_eq!(fp2.x1(), 0);

        let fp3 = Footprint::with_range(3, 7);
        assert_eq!(fp3.x0(), 3);
        assert_eq!(fp3.x1(), 7);
    }

    // TODO: Is this the correct behavior?
    #[test]
    fn footprint_at_edge() {
        let tile = Tile {
            loc: Loc::zero(),
            p0: PackedPoint::new(scale_up(1.0), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(1.0), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().0, 0);
    }

    #[test]
    #[ignore]
    // TODO: Fix this
    fn infinite_loop() {
        let mut line = FlatLine {
            p0: Point { x: 22.0, y: 552.0 },
            p1: Point { x: 224.0, y: 388.0 },
        };
        let mut buf = vec![];
        make_tiles(&[line], &mut buf);
    }
}
