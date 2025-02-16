// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tiling of paths.

use std::fmt::{Debug, Formatter};

pub const TILE_SIZE: u32 = 4;

const TILE_SCALE: f32 = TILE_SIZE as f32;
const INV_TILE_SCALE: f32 = 1.0 / TILE_SIZE as f32;
const NUDGE_FACTOR: f32 = 0.0000001;

/// Handles the tiling of paths.
#[derive(Clone, Debug)]
pub struct Tiler {
    tile_buf: Vec<Tile>,
    tile_index_buf: Vec<TileIndex>,
    sorted: bool,
}

impl Tiler {
    pub fn new() -> Self {
        Self {
            tile_buf: vec![],
            sorted: false,
            tile_index_buf: vec![],
        }
    }

    pub fn len(&self) -> u32 {
        self.tile_buf.len() as u32
    }

    pub fn reset(&mut self) {
        self.tile_buf.clear();
        self.tile_index_buf.clear();
        self.sorted = false;
    }

    pub fn sort_tiles(&mut self) {
        self.sorted = true;
        eprintln!("{:#?}", self.tile_buf);
        self.tile_index_buf.sort_unstable_by(TileIndex::cmp);
    }

    /// Get the tile at a certain index.
    ///
    /// Panics if the tiler hasn't been sorted before.
    pub fn get_tile(&self, index: u32) -> &Tile {
        assert!(self.sorted);

        &self.tile_buf[self.tile_index_buf[index as usize].index()]
    }

    /// Make the tiles.
    pub fn make_tiles(&mut self, lines: &[FlatLine]) {
        self.reset();

        // Calculate how many tiles are covered between two positions. p0 and p1 are scaled
        // to the tile unit square.
        let spanned_tiles =
            |p0: f32, p1: f32| -> u32 { (p0.max(p1).ceil() - p0.min(p1).floor()).max(1.0) as u32 };

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

        let mut push_tile = |x: f32, y: f32, p0: Point, p1: Point| {
            if y >= 0.0 {
                let tile = Tile::new(x as i32, y as u16, p0, p1);
                self.tile_index_buf
                    .push(TileIndex::from_tile(self.tile_buf.len() as u32, &tile));
                self.tile_buf.push(tile);
            }
        };

        for line in lines {
            // Points scaled to the tile unit square.
            let s0 = nudge_point(scale_down(line.p0));
            let s1 = nudge_point(scale_down(line.p1));

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
            let packed0 = Point::new(xfrac0, yfrac0);

            if tile_count_x == 1 {
                let xfrac1 = scale_up(s1.x - x);

                if tile_count_y == 1 {
                    let yfrac1 = scale_up(s1.y - y);

                    // A 1x1 tile.
                    push_tile(x, y, Point::new(xfrac0, yfrac0), Point::new(xfrac1, yfrac1));
                } else {
                    // A vertical column.
                    let inv_slope = (s1.x - s0.x) / (s1.y - s0.y);
                    let sign = (s1.y - s0.y).signum();

                    // For downward lines, xclip0 and yclip store the x and y intersection points
                    // at the bottom side of the current tile. For upward lines, they store the in
                    // intersection points at the top side of the current tile.
                    let mut xclip0 = (s0.x - x) + (y - s0.y) * inv_slope;
                    // We handled the case of a 1x1 tile before, so in this case the line will
                    // definitely cross the tile either at the top or bottom, and thus yclip is
                    // either 0 or 1.
                    let (yclip, flip) = if sign > 0.0 {
                        // If the line goes downward, instead store where the line would intersect
                        // the first tile at the bottom
                        xclip0 += inv_slope;
                        (scale_up(1.0), scale_up(-1.0))
                    } else {
                        // Otherwise, the line goes up, and thus will intersect the top side of the
                        // tile.
                        (scale_up(0.0), scale_up(1.0))
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
                        let xfrac = scale_up(xclip).max(NUDGE_FACTOR);
                        let packed = Point::new(xfrac, yclip);

                        push_tile(x, y, last_packed, packed);

                        // Flip y between top and bottom of tile (i.e. from TILE_HEIGHT
                        // to 0 or 0 to TILE_HEIGHT).
                        last_packed = Point::new(packed.x, packed.y + flip);
                        y += sign;
                    }

                    // Push the last tile, which might be at a fractional y offset.
                    let yfrac1 = scale_up(s1.y - y);
                    let packed1 = Point::new(xfrac1, yfrac1);

                    push_tile(x, y, last_packed, packed1);
                }
            } else if tile_count_y == 1 {
                // A horizontal row.
                // Same explanations apply as above, but instead in the horizontal direction.

                let slope = (s1.y - s0.y) / (s1.x - s0.x);
                let sign = (s1.x - s0.x).signum();

                let mut yclip0 = (s0.y - y) + (x - s0.x) * slope;
                let (xclip, flip) = if sign > 0.0 {
                    yclip0 += slope;
                    (scale_up(1.0), scale_up(-1.0))
                } else {
                    (scale_up(0.0), scale_up(1.0))
                };

                let mut last_packed = packed0;

                for i in 0..tile_count_x - 1 {
                    let yclip = yclip0 + i as f32 * sign * slope;
                    let yfrac = scale_up(yclip).max(0.0000001);
                    let packed = Point::new(xclip, yfrac);

                    push_tile(x, y, last_packed, packed);

                    last_packed = Point::new(packed.x + flip, packed.y);

                    x += sign;
                }

                let xfrac1 = scale_up(s1.x - x);
                let yfrac1 = scale_up(s1.y - y);
                let packed1 = Point::new(xfrac1, yfrac1);

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
                let (xclip, flip_x) = if sign_x > 0.0 {
                    t_clipx += recip_dx;
                    (scale_up(1.0), scale_up(-1.0))
                } else {
                    (scale_up(0.0), scale_up(1.0))
                };

                // How much we advance at each intersection with a horizontal grid line.
                let mut t_clipy = (y - s0.y) * recip_dy;

                // Same as xclip, but for the vertical direction, analogously to the
                // "vertical column" case.
                let (yclip, flip_y) = if sign_y > 0.0 {
                    t_clipy += recip_dy;
                    (scale_up(1.0), scale_up(-1.0))
                } else {
                    (scale_up(0.0), scale_up(1.0))
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
                        let xfrac = scale_up(x_intersect).max(NUDGE_FACTOR);
                        let packed = Point::new(xfrac, yclip);

                        push_tile(xi, yi, last_packed, packed);

                        t_clipy += recip_dy.abs();
                        yi += sign_y;
                        last_packed = Point::new(packed.x, packed.y + flip_y);
                    } else {
                        // Intersected with vertical grid line.
                        let y_intersect = s0.y + (s1.y - s0.y) * t_clipx - yi;
                        let yfrac = scale_up(y_intersect).max(NUDGE_FACTOR);
                        let packed = Point::new(xclip, yfrac);

                        push_tile(xi, yi, last_packed, packed);

                        t_clipx += recip_dx.abs();
                        xi += sign_x;
                        last_packed = Point::new(packed.x + flip_x, packed.y);
                    }
                }

                // The last tile, where the end point is possibly not at an integer coordinate.
                let xfrac1 = scale_up(s1.x - xi);
                let yfrac1 = scale_up(s1.y - yi);
                let packed1 = Point::new(xfrac1, yfrac1);

                push_tile(xi, yi, last_packed, packed1);
            }
        }

        // This particular choice of sentinel tiles generates a sentinel strip.
        push_tile(
            0x3ffd as f32,
            0x3fff as f32,
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        );
        push_tile(
            0x3fff as f32,
            0x3fff as f32,
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        );
    }
}

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

#[derive(Clone, Debug)]
struct TileIndex {
    x: u16,
    y: u16,
    index: u32,
}

impl TileIndex {
    pub fn from_tile(index: u32, tile: &Tile) -> Self {
        let x = (tile.x + 1).max(0) as u16;
        let y = tile.y;

        Self { x, y, index }
    }

    pub(crate) fn cmp(&self, b: &TileIndex) -> std::cmp::Ordering {
        let xya = ((self.y as u32) << 16) + (self.x as u32);
        let xyb = ((b.y as u32) << 16) + (b.x as u32);
        xya.cmp(&xyb)
    }

    pub fn index(&self) -> usize {
        self.index as usize
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
    x: i32,
    /// The index of the tile in the y direction.
    y: u16,
    /// The start point of the line in that tile.
    p0: Point,
    /// The end point of the line in that tile.
    p1: Point,
}

impl Tile {
    pub fn new(x: i32, y: u16, p0: Point, p1: Point) -> Self {
        Self {
            // We don't need to store the exact negative location, just that it is negative,
            // so that the winding number calculation is correct.
            x: x.max(-1),
            y,
            p0,
            p1,
        }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> u16 {
        self.y
    }

    pub fn p0(&self) -> Point {
        self.p0
    }

    pub fn p1(&self) -> Point {
        self.p1
    }

    pub(crate) fn loc(&self) -> Loc {
        Loc {
            x: self.x(),
            y: self.y,
        }
    }

    pub(crate) fn footprint(&self) -> Footprint {
        let x0 = self.p0().x;
        let x1 = self.p1().x;
        let x_min = x0.min(x1).floor();
        let x_max = x0.max(x1).ceil();
        // On CPU, might be better to do this as fixed point
        let start_i = x_min as u32;
        let end_i = (start_i + 1).max(x_max as u32).min(TILE_SIZE);

        Footprint::from_range(start_i as u8, end_i as u8)
    }

    pub(crate) fn delta(&self) -> i32 {
        (self.p1().y == 0.0) as i32 - (self.p0().y == 0.0) as i32
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

const FRAC_TILE_SCALE: f32 = 8192.0 * 4.0;

const fn scale_up(z: f32) -> f32 {
    z * 4.0
}

fn scale_down(z: Point) -> Point {
    z * INV_TILE_SCALE
}

#[cfg(test)]
mod tests {
    use crate::tiling::{scale_up, FlatLine, Footprint, Point, Tile, Tiler};

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
        let tile = Tile::new(
            0,
            0,
            Point::new(scale_up(1.0), scale_up(0.0)),
            Point::new(scale_up(1.0), scale_up(1.0)),
        );

        assert!(tile.footprint().is_empty());
    }

    #[test]
    fn footprints_in_tile() {
        let tile = Tile::new(
            0,
            0,
            Point::new(scale_up(0.5), scale_up(0.0)),
            Point::new(scale_up(0.55), scale_up(1.0)),
        );

        assert_eq!(tile.footprint().x0(), 2);
        assert_eq!(tile.footprint().x1(), 3);

        let tile = Tile::new(
            0,
            0,
            Point::new(scale_up(0.1), scale_up(0.0)),
            Point::new(scale_up(0.6), scale_up(1.0)),
        );

        assert_eq!(tile.footprint().x0(), 0);
        assert_eq!(tile.footprint().x1(), 3);

        let tile = Tile::new(
            0,
            0,
            Point::new(scale_up(0.0), scale_up(0.0)),
            Point::new(scale_up(1.0), scale_up(1.0)),
        );

        assert_eq!(tile.footprint().x0(), 0);
        assert_eq!(tile.footprint().x1(), 4);

        let tile = Tile::new(
            0,
            0,
            Point::new(scale_up(0.74), scale_up(0.0)),
            Point::new(scale_up(1.76), scale_up(1.0)),
        );

        assert_eq!(tile.footprint().x0(), 2);
        assert_eq!(tile.footprint().x1(), 4);
    }

    #[test]
    fn issue_46_infinite_loop() {
        let mut line = FlatLine {
            p0: Point { x: 22.0, y: 552.0 },
            p1: Point { x: 224.0, y: 388.0 },
        };

        let mut tiler = Tiler::new();
        tiler.make_tiles(&[line]);
    }
}
