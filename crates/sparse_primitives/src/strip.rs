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

use crate::tiling::{PackedPoint, Tile, TILE_WIDTH};
use crate::wide_tile::STRIP_HEIGHT;
use crate::FillRule;
use std::arch::is_aarch64_feature_detected;

pub(crate) struct Footprint(pub(crate) u32);

#[derive(Debug, Clone, Copy)]
pub struct Strip {
    pub x: i32,
    pub y: i32,
    pub col: u32,
    pub winding: i32,
}

#[inline(never)]
pub fn render_strips(
    tiles: &[Tile],
    strip_buf: &mut Vec<Strip>,
    alpha_buf: &mut Vec<u32>,
    fill_rule: FillRule,
    use_simd: bool,
) {
    if !use_simd {
        render_strips_scalar(tiles, strip_buf, alpha_buf, fill_rule);
    } else {
        #[cfg(target_arch = "aarch64")]
        if is_aarch64_feature_detected!("neon") {
            // SAFETY: We checked that the target feature `neon` is available.
            return unsafe { neon::render_strips(tiles, strip_buf, alpha_buf) };
        }

        render_strips_scalar(tiles, strip_buf, alpha_buf, fill_rule);
    }
}

fn render_strips_scalar(
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

                let p0 = tile.p0().unpack();
                let p1 = tile.p1().unpack();
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
                    x: 4 * prev_tile.x() + x0 as i32,
                    y: 4 * prev_tile.y(),
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
    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn strip_y(&self) -> u32 {
        // TODO: Don't convert?
        self.y as u32 / STRIP_HEIGHT as u32
    }
}

#[cfg(target_arch = "aarch64")]
mod neon {
    use crate::strip::Strip;
    use crate::tiling::Tile;
    use std::arch::aarch64::*;

    /// SAFETY: Caller must ensure that target feature 'neon' is available.
    pub unsafe fn render_strips(
        tiles: &[Tile],
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
    ) {
        // TODO: Clean up and improve this impementation
        // TODO: Add even-odd filling.
        unsafe {
            strip_buf.clear();

            let mut strip_start = true;
            let mut cols = alpha_buf.len() as u32;
            let mut prev_tile = &tiles[0];
            let mut fp = prev_tile.footprint().0;
            let mut seg_start = 0;
            let mut delta = 0;
            // Note: the input should contain a sentinel tile, to avoid having
            // logic here to process the final strip.
            const IOTA: [f32; 4] = [0.0, 1.0, 2.0, 3.0];
            let iota = vld1q_f32(IOTA.as_ptr() as *const f32);
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
                        // small gain possible here to unpack in simd, but llvm goes halfway
                        delta += tile.delta();
                        let p0 = tile.p0().unpack();
                        let p1 = tile.p1().unpack();
                        let slope = (p1.x - p0.x) / (p1.y - p0.y);
                        let vstarty = vsubq_f32(vdupq_n_f32(p0.y), iota);
                        let vy0 = vminq_f32(vmaxq_f32(vstarty, vdupq_n_f32(0.0)), vdupq_n_f32(1.0));
                        let vy1a = vsubq_f32(vdupq_n_f32(p1.y), iota);
                        let vy1 = vminq_f32(vmaxq_f32(vy1a, vdupq_n_f32(0.0)), vdupq_n_f32(1.0));
                        let vdy = vsubq_f32(vy0, vy1);
                        let mask = vceqzq_f32(vdy);
                        let vslope = vbslq_f32(mask, vdupq_n_f32(0.0), vdupq_n_f32(slope));
                        let vdy0 = vsubq_f32(vy0, vstarty);
                        let vdy1 = vsubq_f32(vy1, vstarty);
                        let mut vyedge = vdupq_n_f32(0.0);
                        if p0.x == 0.0 {
                            let ye = vsubq_f32(vdupq_n_f32(1.0), vstarty);
                            vyedge = vminq_f32(vmaxq_f32(ye, vdupq_n_f32(0.0)), vdupq_n_f32(1.0));
                        } else if p1.x == 0.0 {
                            let ye = vsubq_f32(vy1a, vdupq_n_f32(1.0));
                            vyedge = vminq_f32(vmaxq_f32(ye, vdupq_n_f32(-1.0)), vdupq_n_f32(0.0));
                        }
                        for x in x0..x1 {
                            let mut varea = vld1q_f32(areas.as_ptr().add(x as usize) as *const f32);
                            varea = vaddq_f32(varea, vyedge);
                            let vstartx = vdupq_n_f32(p0.x - x as f32);
                            let vxx0 = vfmaq_f32(vstartx, vdy0, vslope);
                            let vxx1 = vfmaq_f32(vstartx, vdy1, vslope);
                            let vxmin0 = vminq_f32(vxx0, vxx1);
                            let vxmax = vmaxq_f32(vxx0, vxx1);
                            let vxmin =
                                vsubq_f32(vminq_f32(vxmin0, vdupq_n_f32(1.0)), vdupq_n_f32(1e-6));
                            let vb = vminq_f32(vxmax, vdupq_n_f32(1.0));
                            let vc = vmaxq_f32(vb, vdupq_n_f32(0.0));
                            let vd = vmaxq_f32(vxmin, vdupq_n_f32(0.0));
                            let vd2 = vmulq_f32(vd, vd);
                            let vd2c2 = vfmsq_f32(vd2, vc, vc);
                            let vax = vfmaq_f32(vb, vd2c2, vdupq_n_f32(0.5));
                            let va = vdivq_f32(vsubq_f32(vax, vxmin), vsubq_f32(vxmax, vxmin));
                            varea = vfmaq_f32(varea, va, vdy);
                            vst1q_f32(areas.as_mut_ptr().add(x as usize) as *mut f32, varea);
                        }
                    }
                    for x in x0..x1 {
                        let mut alphas = 0u32;
                        let varea = vld1q_f32(areas.as_ptr().add(x as usize) as *const f32);
                        let vnzw = vminq_f32(vabsq_f32(varea), vdupq_n_f32(1.0));
                        let vscaled = vmulq_f32(vnzw, vdupq_n_f32(255.0));
                        let vbits = vreinterpretq_u8_u32(vcvtnq_u32_f32(vscaled));
                        let vbits2 = vuzp1q_u8(vbits, vbits);
                        let vbits3 = vreinterpretq_u32_u8(vuzp1q_u8(vbits2, vbits2));
                        vst1q_lane_u32::<0>(&mut alphas, vbits3);
                        alpha_buf.push(alphas);
                    }

                    if strip_start {
                        let strip = Strip {
                            x: 4 * prev_tile.x() + x0 as i32,
                            y: 4 * prev_tile.y(),
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
    }
}
