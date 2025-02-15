// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::{Debug, Formatter};

pub const TILE_WIDTH: u32 = 4;
pub const TILE_HEIGHT: u32 = 4;

const TILE_SCALE_X: f32 = 1.0 / TILE_WIDTH as f32;
const TILE_SCALE_Y: f32 = 1.0 / TILE_HEIGHT as f32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Loc {
    // TODO: Unlike y, will not always be positive since we cannot ignore tiles where x < 0 because
    // they still impact the winding number. We should be able to change this once we have viewport
    // culling.
    pub x: i32,
    pub y: u16,
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
    pub(crate) fn from_index(index: u8) -> Footprint {
        Footprint(1 << index)
    }

    /// Create a new footprint from a single index, i.e. [start, end).
    pub(crate) fn from_range(start: u8, end: u8) -> Footprint {
        Footprint((1 << end) - (1 << start))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// The start point of the covered range (inclusive).
    pub(crate) fn x0(&self) -> u32 {
        self.0.trailing_zeros()
    }

    /// The end point of the covered range (exclusive).
    pub(crate) fn x1(&self) -> u32 {
        32 - self.0.leading_zeros()
    }

    /// Extend the range with a single index.
    pub(crate) fn extend(&mut self, index: u8) {
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
#[derive(Debug, Clone)]
pub struct Tile {
    /// The index of the tile in the x direction.
    x: u16,
    /// The index of the tile in the y direction.
    y: u16,
    /// The start point of the line in that tile.
    pub p0: PackedPoint,
    /// The end point of the line in that tile.
    pub p1: PackedPoint,
}

impl Tile {
    pub fn new(x: i32, y: u16, p0: PackedPoint, p1: PackedPoint) -> Self {
        // As mentioned in the comment in `Loc`, for the x position we need to be
        // able to store negative numbers. Because of this, we basically offset all numbers by
        // 1, assigning negative numbers to 0 x + 1 to all others. This way, we can still
        // sort them efficiently by packing x and y into a u32.
        let x = (x + 1).max(0) as u16;
        Self { x, y, p0, p1 }
    }

    pub fn x(&self) -> i32 {
        self.x as i32 - 1
    }

    pub fn y(&self) -> u16 {
        self.y
    }

    pub(crate) fn loc(&self) -> Loc {
        Loc {
            x: self.x(),
            y: self.y,
        }
    }

    pub(crate) fn footprint(&self) -> Footprint {
        let x0 = self.p0.unpacked_x();
        let x1 = self.p1.unpacked_x();
        let x_min = x0.min(x1).floor();
        let x_max = x0.max(x1).ceil();
        // On CPU, might be better to do this as fixed point
        let start_i = x_min as u32;
        let end_i = (start_i + 1).max(x_max as u32).min(TILE_WIDTH);

        Footprint::from_range(start_i as u8, end_i as u8)
    }

    pub(crate) fn delta(&self) -> i32 {
        (self.p1.packed_y() == 0) as i32 - (self.p0.packed_y() == 0) as i32
    }

    pub(crate) fn cmp(&self, b: &Tile) -> std::cmp::Ordering {
        // Note(raph): Verified in godbolt that this is efficient.
        let xya = ((self.y as u32) << 16) + (self.x as u32);
        let xyb = ((b.y as u32) << 16) + (b.x as u32);
        xya.cmp(&xyb)
    }
}

/// Same as a line, but uses f32 instead.
#[derive(Clone, Copy, Debug)]
pub struct FlatLine {
    pub p0: Point,
    pub p1: Point,
}

impl FlatLine {
    pub fn new(p0: Point, p1: Point) -> Self {
        Self { p0, p1 }
    }
}

/// Stores a point within a tile (which can range from 0 to TILE_WIDTH/TILE_HEIGHT) as a
/// u16, to reduce the memory footprint when sorting tiles.
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
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
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

const TILE_SCALE: f32 = 8192.0;
// scale factor relative to unit square in tile
const FRAC_TILE_SCALE: f32 = 8192.0 * 4.0;

fn scale_up(z: f32) -> u16 {
    ((z * FRAC_TILE_SCALE) + 0.5) as u16
}

fn scale_down(z: u16) -> f32 {
    z as f32 / FRAC_TILE_SCALE
}

pub fn make_tiles(lines: &[FlatLine], tile_buf: &mut Vec<Tile>) {
    tile_buf.clear();

    // Calculate how many tiles are covered between two positions. p0 and p1 are scaled
    // to the tile unit square.
    let spanned_tiles =
        |p0: f32, p1: f32| -> u32 { (p0.max(p1).ceil() - p0.min(p1).floor()).max(1.0) as u32 };

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
        // anti-aliasing. This case is detected via tile-relative x == 0. However,
        // lines can naturally start or end at a multiple of the 4x4 grid, too, but
        // these don't constitute crossings. We nudge these points ever so slightly,
        // by ensuring that xfrac0 and xfrac1 are always at least 1, which
        // corresponds to 1/8192 of a pixel. By doing so, whenever we encounter a point
        // at a tile relative 0, we can treat it as an edge crossing. This is somewhat
        // of a hack and in theory we should rather solve the underlying issue in the
        // strip generation code, but it works for now.

        if p.x.fract() == 0.0 {
            Point {
                x: p.x + 1.0 / FRAC_TILE_SCALE,
                y: p.y,
            }
        } else {
            p
        }
    };

    let mut push_tile = |x: f32, y: f32, p0: PackedPoint, p1: PackedPoint| {
        if y >= 0.0 {
            tile_buf.push(Tile::new(x as i32, y as u16, p0, p1));
        }
    };

    for line in lines {
        // Points scaled to the tile unit square.
        let s0 = nudge_point(round_point(line.p0 * TILE_SCALE_X));
        let s1 = nudge_point(round_point(line.p1 * TILE_SCALE_Y));

        // Count how many tiles are covered on each axis.
        let tile_count_x = spanned_tiles(s0.x, s1.x);
        let tile_count_y = spanned_tiles(s0.y, s1.y);

        // Note: This code is technically unreachable now, because we always nudge x points at tile-relative 0
        // position. But we might need it again in the future if we change the logic.
        let mut x = s0.x.floor();
        if s0.x == x && s1.x < x {
            // s0.x is on right side of first tile.
            x -= 1.0;
        }

        let mut y = s0.y.floor();
        if s0.y == y && s1.y < y {
            // Since the end point of the line is above the start point,
            // s0.y is conceptually on bottom of the previous tile instead of at the top
            // of the current tile, so we need to adjust the y location.
            y -= 1.0;
        }

        let xfrac0 = scale_up(s0.x - x);
        let yfrac0 = scale_up(s0.y - y);
        let packed0 = PackedPoint::new(xfrac0, yfrac0);

        if tile_count_x == 1 {
            let xfrac1 = scale_up(s1.x - x);

            if tile_count_y == 1 {
                let yfrac1 = scale_up(s1.y - y);

                // A 1x1 tile.
                push_tile(
                    x,
                    y,
                    PackedPoint::new(xfrac0, yfrac0),
                    PackedPoint::new(xfrac1, yfrac1),
                );
            } else {
                // A vertical column.
                let inv_slope = (s1.x - s0.x) / (s1.y - s0.y);
                // TODO: Get rid of the sign by changing direction of line?
                let sign = (s1.y - s0.y).signum();

                // For downward lines, xclip0 and yclip store the x and y intersection points
                // at the bottom side of the current tile. For upward lines, they store the in
                // intersection points at the top side of the current tile.
                let mut xclip0 = (s0.x - x) + (y - s0.y) * inv_slope;
                // We handled the case of a 1x1 tile before, so in this case the line will
                // definitely cross the tile either at the top or bottom, and thus yclip is
                // either 0 or 1.
                let yclip = if sign > 0.0 {
                    // If the line goes downward, instead store where the line would intersect
                    // the first tile at the bottom
                    xclip0 += inv_slope;
                    scale_up(1.0)
                } else {
                    // Otherwise, the line goes up, and thus will intersect the top side of the
                    // tile.
                    0
                };

                let mut last_packed = packed0;
                // For the first tile, as well as all subsequent tiles that are intersected
                // at the top and bottom, calculate the x intersection points and push the
                // corresponding tiles.

                // Note: This could perhaps be SIMD-optimized, but initial experiments suggest
                // that in the vast majority of cases the number of tiles is between 0-5, so
                // it's probably not really worth it.
                for i in 0..tile_count_y - 1 {
                    // Calculate the next x intersection point.
                    let xclip = xclip0 + i as f32 * sign * inv_slope;
                    // The .max(1) is necessary to indicate that the point actually crosses the
                    // edge instead of ending at it. Perhaps we can figure out a different way
                    // to represent this.
                    let xfrac = scale_up(xclip).max(1);
                    let packed = PackedPoint::new(xfrac, yclip);

                    push_tile(x, y, last_packed, packed);

                    // Flip y between top and bottom of tile (i.e. from TILE_HEIGHT
                    // to 0 or 0 to TILE_HEIGHT).
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                    y += sign;
                }

                // Push the last tile, which might be at a fractional y offset.
                let yfrac1 = scale_up(s1.y - y);
                let packed1 = PackedPoint::new(xfrac1, yfrac1);

                push_tile(x, y, last_packed, packed1);
            }
        } else if tile_count_y == 1 {
            // A horizontal row.
            // Same explanations apply as above, but instead in the horizontal direction.

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

            for i in 0..tile_count_x - 1 {
                let yclip = yclip0 + i as f32 * sign * slope;
                let yfrac = scale_up(yclip).max(1);
                let packed = PackedPoint::new(xclip, yfrac);

                push_tile(x, y, last_packed, packed);

                last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);

                x += sign
            }

            let xfrac1 = scale_up(s1.x - x);
            let yfrac1 = scale_up(s1.y - y);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            push_tile(x, y, last_packed, packed1);
        } else {
            // General case (i.e. more than one tile covered in both directions). We perform a DDA
            // to "walk" along the path and find out which tiles are intersected by the line
            // and at which positions.

            let recip_dx = 1.0 / (s1.x - s0.x);
            let sign_x = (s1.x - s0.x).signum();
            let recip_dy = 1.0 / (s1.y - s0.y);
            let sign_y = (s1.y - s0.y).signum();

            // How much we advance at each intersection with a vertical grid line.
            let mut t_clipx = (x - s0.x) * recip_dx;

            // Similarly to the case "horizontal column", if the line goes to the right,
            // we will always intersect the tiles on the right side (except for perhaps the last
            // tile, but this case is handled separately in the end). Otherwise, we always intersect
            // on the left side.
            let xclip = if sign_x > 0.0 {
                t_clipx += recip_dx;
                scale_up(1.0)
            } else {
                0
            };

            // How much we advance at each intersection with a horizontal grid line.
            let mut t_clipy = (y - s0.y) * recip_dy;

            // Same as xclip, but for the vertical direction, analogously to the
            // "vertical column" case.
            let yclip = if sign_y > 0.0 {
                t_clipy += recip_dy;
                scale_up(1.0)
            } else {
                0
            };

            // x and y coordinates of the target tile.
            let x1 = x + (tile_count_x - 1) as f32 * sign_x;
            let y1 = y + (tile_count_y - 1) as f32 * sign_y;
            let mut xi = x;
            let mut yi = y;
            let mut last_packed = packed0;

            loop {
                // See issue 46 for why we don't just use an inequality check.
                let x_cond = if sign_x > 0.0 { xi >= x1 } else { xi <= x1 };
                let y_cond = if sign_y > 0.0 { yi >= y1 } else { yi <= y1 };

                if x_cond && y_cond {
                    break;
                }

                if t_clipy < t_clipx {
                    // Intersected with a horizontal grid line.
                    let x_intersect = s0.x + (s1.x - s0.x) * t_clipy - xi;
                    let xfrac = scale_up(x_intersect).max(1); // maybe should clamp?
                    let packed = PackedPoint::new(xfrac, yclip);

                    push_tile(xi, yi, last_packed, packed);

                    t_clipy += recip_dy.abs();
                    yi += sign_y;
                    last_packed = PackedPoint::new(packed.x, packed.y ^ FRAC_TILE_SCALE as u16);
                } else {
                    // Intersected with vertical grid line.
                    let y_intersect = s0.y + (s1.y - s0.y) * t_clipx - yi;
                    let yfrac = scale_up(y_intersect).max(1); // maybe should clamp?
                    let packed = PackedPoint::new(xclip, yfrac);

                    push_tile(xi, yi, last_packed, packed);

                    t_clipx += recip_dx.abs();
                    xi += sign_x;
                    last_packed = PackedPoint::new(packed.x ^ FRAC_TILE_SCALE as u16, packed.y);
                }
            }

            // The last tile, where the end point is possibly not at an integer coordinate.
            let xfrac1 = scale_up(s1.x - xi);
            let yfrac1 = scale_up(s1.y - yi);
            let packed1 = PackedPoint::new(xfrac1, yfrac1);

            push_tile(xi, yi, last_packed, packed1);
        }
    }

    // This particular choice of sentinel tiles generates a sentinel strip.
    push_tile(
        0x3ffd as f32,
        0x3fff as f32,
        PackedPoint::new(0, 0),
        PackedPoint::new(0, 0),
    );
    push_tile(
        0x3fff as f32,
        0x3fff as f32,
        PackedPoint::new(0, 0),
        PackedPoint::new(0, 0),
    );
}

pub fn sort_tiles(tile_buf: &mut [Tile]) {
    tile_buf.sort_unstable_by(Tile::cmp);
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
    fn footprint_from_index() {
        let fp1 = Footprint::from_index(0);
        assert_eq!(fp1.x0(), 0);
        assert_eq!(fp1.x1(), 1);

        let fp2 = Footprint::from_index(3);
        assert_eq!(fp2.x0(), 3);
        assert_eq!(fp2.x1(), 4);

        let fp3 = Footprint::from_index(6);
        assert_eq!(fp3.x0(), 6);
        assert_eq!(fp3.x1(), 7);
    }

    #[test]
    fn footprint_from_range() {
        let fp1 = Footprint::from_range(1, 3);
        assert_eq!(fp1.x0(), 1);
        assert_eq!(fp1.x1(), 3);

        // Same comment as for empty.
        let fp2 = Footprint::from_range(2, 2);
        assert_eq!(fp2.x0(), 32);
        assert_eq!(fp2.x1(), 0);

        let fp3 = Footprint::from_range(3, 7);
        assert_eq!(fp3.x0(), 3);
        assert_eq!(fp3.x1(), 7);
    }

    #[test]
    fn footprint_extend() {
        let mut fp = Footprint::empty();
        fp.extend(5);
        assert_eq!(fp.x0(), 5);
        assert_eq!(fp.x1(), 6);

        fp.extend(3);
        assert_eq!(fp.x0(), 3);
        assert_eq!(fp.x1(), 6);

        fp.extend(8);
        assert_eq!(fp.x0(), 3);
        assert_eq!(fp.x1(), 9);

        fp.extend(0);
        assert_eq!(fp.x0(), 0);
        assert_eq!(fp.x1(), 9);

        fp.extend(9);
        assert_eq!(fp.x0(), 0);
        assert_eq!(fp.x1(), 10);
    }

    #[test]
    fn footprint_merge() {
        let mut fp1 = Footprint::from_range(2, 4);
        let fp2 = Footprint::from_range(5, 6);
        fp1.merge(&fp2);

        assert_eq!(fp1.x0(), 2);
        assert_eq!(fp1.x1(), 6);

        let mut fp3 = Footprint::from_range(5, 9);
        let fp4 = Footprint::from_range(7, 10);
        fp3.merge(&fp4);

        assert_eq!(fp3.x0(), 5);
        assert_eq!(fp3.x1(), 10);
    }

    #[test]
    fn footprint_at_tile_edge() {
        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(1.0), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(1.0), scale_up(1.0)),
        };

        assert!(tile.footprint().is_empty());
    }

    #[test]
    fn footprints_in_tile() {
        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(0.5), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(0.55), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().x0(), 2);
        assert_eq!(tile.footprint().x1(), 3);

        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(0.1), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(0.6), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().x0(), 0);
        assert_eq!(tile.footprint().x1(), 3);

        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(0.0), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(1.0), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().x0(), 0);
        assert_eq!(tile.footprint().x1(), 4);

        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(0.74), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(1.76), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().x0(), 2);
        assert_eq!(tile.footprint().x1(), 4);
    }

    #[test]
    fn issue_46_infinite_loop() {
        let mut line = FlatLine {
            p0: Point { x: 22.0, y: 552.0 },
            p1: Point { x: 224.0, y: 388.0 },
        };
        let mut buf = vec![];
        make_tiles(&[line], &mut buf);
    }
}
