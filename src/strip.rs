// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CPU implementation of sparse strip rendering
//!
//! This is copied from the most recent GPU implementation, but has
//! path_id stripped out, as on CPU we'll be doing one path at a time.
//! That decision makes sense to some extent even when uploading to
//! GPU, though some mechanism is required to tie the strips to paint.
//!
//! If there becomes a single, unified code base for this, then the
//! path_id type should probably become a generic parameter.

use crate::tiling::{PackedPoint, TILE_WIDTH};
use crate::wide_tile::STRIP_HEIGHT;
use crate::FillRule;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Loc {
    x: u16,
    y: u16,
}

pub(crate) struct Footprint(pub(crate) u32);

pub struct Tile {
    pub x: u16,
    pub y: u16,
    pub p0: PackedPoint,
    pub p1: PackedPoint,
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p0 = self.p0.unpack();
        let p1 = self.p1.unpack();
        write!(
            f,
            "Tile {{ xy: ({}, {}), p0: ({:.4}, {:.4}), p1: ({:.4}, {:.4}) }}",
            self.x, self.y, p0.x, p0.y, p1.x, p1.y
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Strip {
    pub x: u32,
    pub y: u32,
    pub col: u32,
    pub winding: i32,
}

impl Loc {
    /// Two locations are on the same strip if they are on the same
    /// row and next to each other.
    pub(crate) fn same_strip(&self, other: &Self) -> bool {
        self.same_row(other) && (other.x - self.x) / 2 == 0
    }

    pub(crate) fn same_row(&self, other: &Self) -> bool {
        self.y == other.y
    }
}

impl Tile {
    pub(crate) fn loc(&self) -> Loc {
        Loc {
            x: self.x,
            y: self.y,
        }
    }

    pub(crate) fn footprint(&self) -> Footprint {
        let x0 = self.p0.unpacked_x();
        let x1 = self.p1.unpacked_x();
        // On CPU, might be better to do this as fixed point
        let xmin = x0.min(x1).floor() as u32;
        let xmax = (xmin + 1).max(x0.max(x1).ceil() as u32).min(TILE_WIDTH);
        Footprint((1 << xmax) - (1 << xmin))
    }

    pub(crate) fn delta(&self) -> i32 {
        (self.p1.packed_y() == 0) as i32 - (self.p0.packed_y() == 0) as i32
    }

    // Comparison function for sorting. Only compares loc, doesn't care
    // about points. Unpacking code has been validated to be efficient in
    // Godbolt.
    pub fn cmp(&self, b: &Tile) -> std::cmp::Ordering {
        let xya = ((self.y as u32) << 16) + (self.x as u32);
        let xyb = ((b.y as u32) << 16) + (b.x as u32);
        xya.cmp(&xyb)
    }
}

pub fn render_strips_scalar(
    tiles: &[Tile],
    strip_buf: &mut Vec<Strip>,
    alpha_buf: &mut Vec<u32>,
    fill_rule: FillRule,
) {
    strip_buf.clear();

    let mut strip_start = true;
    let mut cols = alpha_buf.len() as u32;
    let mut prev_tile = &tiles[0];
    let mut fp = prev_tile.footprint().0;
    let mut seg_start = 0;
    let mut delta = 0;

    // Note: the input should contain a sentinel tile, to avoid having
    // logic here to process the final strip.
    for i in 1..tiles.len() {
        let tile = &tiles[i];

        if prev_tile.loc() != tile.loc() {
            let start_delta = delta;
            let same_strip = prev_tile.loc().same_strip(&tile.loc());

            if same_strip {
                fp |= 8;
            }

            let x0 = fp.trailing_zeros();
            let x1 = 32 - fp.leading_zeros();
            let mut areas = [[start_delta as f32; 4]; 4];

            for tile in &tiles[seg_start..i] {
                delta += tile.delta();

                let p0 = tile.p0.unpack();
                let p1 = tile.p1.unpack();
                let inv_slope = (p1.x - p0.x) / (p1.y - p0.y);

                // Note: We are iterating in column-major order because the inner loop always
                // has a constant number of iterations, which makes it more SIMD-friendly. Worth
                // running some tests whether a different order allows for better performance.
                for x in x0..x1 {
                    // Relative x offset of the start point from the
                    // current column.
                    let rel_x = p0.x - x as f32;

                    for y in 0..4 {
                        // Relative y offset of the start
                        // point from the current row.
                        let rel_y = p0.y - y as f32;
                        // y values will be 1 if the point is below the current row,
                        // 0 if the point is above the current row, and between 0-1
                        // if it is on the same row.
                        let y0 = rel_y.clamp(0.0, 1.0);
                        let y1 = (p1.y - y as f32).clamp(0.0, 1.0);
                        // If != 0, then the line intersects the current row
                        // in the current tile.
                        let dy = y0 - y1;

                        // Note: getting rid of this predicate might help with
                        // auto-vectorization. That said, just getting rid of
                        // it causes artifacts (which may be divide by zero).
                        if dy != 0.0 {
                            // x intersection points in the current tile.
                            let xx0 = rel_x + (y0 - rel_y) * inv_slope;
                            let xx1 = rel_x + (y1 - rel_y) * inv_slope;
                            let xmin0 = xx0.min(xx1);
                            let xmax = xx0.max(xx1);
                            // Subtract a small delta to prevent a division by zero below.
                            let xmin = xmin0.min(1.0) - 1e-6;
                            // Clip x_max to the right side of the pixel.
                            let b = xmax.min(1.0);
                            // Clip x_max to the left side of the pixel.
                            let c = b.max(0.0);
                            // Clip x_min to the left side of the pixel.
                            let d = xmin.max(0.0);
                            // Calculate the covered area.
                            // TODO: How is this formula derived?
                            let a = (b + 0.5 * (d * d - c * c) - xmin) / (xmax - xmin);

                            // Above area calculation is under the assumption that the line
                            // covers the whole row, here we account for the fact that only a
                            // a fraction of the height could be covered.
                            areas[x as usize][y] += a * dy;
                        }

                        if p0.x == 0.0 {
                            areas[x as usize][y] += (y as f32 - p0.y + 1.0).clamp(0.0, 1.0);
                        } else if p1.x == 0.0 {
                            areas[x as usize][y] -= (y as f32 - p1.y + 1.0).clamp(0.0, 1.0);
                        }
                    }
                }
            }

            for x in x0..x1 {
                let mut alphas = 0u32;

                for y in 0..4 {
                    let area = areas[x as usize][y];

                    let area_u8 = match fill_rule {
                        FillRule::NonZero => (area.abs().min(1.0) * 255.0 + 0.5) as u32,
                        FillRule::EvenOdd => {
                            let even = area as i32 % 2;
                            // If we have for example 2.68, then opacity is 68%, while for
                            // 1.68 it would be (1 - 0.68) = 32%.
                            let add_val = even as f32;
                            // 1 for even, -1 for odd.
                            let sign = (-2 * even + 1) as f32;

                            ((add_val + sign * area.fract()) * 255.0 + 0.5) as u32
                        }
                    };

                    alphas += area_u8 << (y * 8);
                }

                alpha_buf.push(alphas);
            }

            if strip_start {
                let strip = Strip {
                    x: 4 * prev_tile.x as u32 + x0,
                    y: 4 * prev_tile.y as u32,
                    col: cols,
                    winding: start_delta,
                };

                strip_buf.push(strip);
            }

            cols += x1 - x0;
            fp = if same_strip { 1 } else { 0 };

            strip_start = !same_strip;
            seg_start = i;

            if !prev_tile.loc().same_row(&tile.loc()) {
                delta = 0;
            }
        }

        fp |= tile.footprint().0;

        prev_tile = tile;
    }
}

impl Strip {
    pub fn x(&self) -> u32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    pub fn strip_y(&self) -> u32 {
        self.y / STRIP_HEIGHT as u32
    }
}

#[cfg(test)]
mod tests {
    use crate::strip::Tile;
    use crate::tiling::{scale_up, PackedPoint};

    // TODO: Is this the correct behavior?
    #[test]
    fn footprint_at_edge() {
        let tile = Tile {
            x: 0,
            y: 0,
            p0: PackedPoint::new(scale_up(1.0), scale_up(0.0)),
            p1: PackedPoint::new(scale_up(1.0), scale_up(1.0)),
        };

        assert_eq!(tile.footprint().0, 0);
    }
}
