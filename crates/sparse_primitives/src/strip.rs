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

use crate::execute::KernelExecutor;
use crate::tiling::Tiles;
use crate::wide_tile::STRIP_HEIGHT;
use crate::FillRule;

#[derive(Debug, Clone, Copy)]
pub struct Strip {
    pub x: i32,
    pub y: u32,
    pub col: u32,
    pub winding: i32,
}

#[inline(never)]
pub fn render_strips<KE: KernelExecutor>(
    tiles: &Tiles,
    strip_buf: &mut Vec<Strip>,
    alpha_buf: &mut Vec<u32>,
    fill_rule: FillRule,
) {
    strip_buf.clear();

    KE::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
}

impl Strip {
    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    pub fn strip_y(&self) -> u32 {
        // TODO: Don't convert?
        self.y / STRIP_HEIGHT as u32
    }
}

pub(crate) mod scalar {
    use crate::strip::Strip;
    use crate::tiling::{Footprint, Tiles};
    use crate::FillRule;

    pub(crate) fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        let mut strip_start = true;
        let mut cols = alpha_buf.len() as u32;
        let mut prev_tile = tiles.get_tile(0);
        let mut fp = prev_tile.footprint();
        let mut seg_start = 0;
        let mut delta = 0;

        // Note: the input should contain a sentinel tile, to avoid having
        // logic here to process the final strip.
        for i in 1..tiles.len() {
            let tile = tiles.get_tile(i);

            if prev_tile.loc() != tile.loc() {
                let start_delta = delta;
                let same_strip = prev_tile.loc().same_strip(&tile.loc());

                if same_strip {
                    fp.extend(3);
                }

                let x0 = fp.x0();
                let x1 = fp.x1();
                let mut areas = [start_delta as f32; 16];

                for j in seg_start..i {
                    let tile = tiles.get_tile(j);

                    delta += tile.delta();

                    let p0 = tile.p0();
                    let p1 = tile.p1();
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
                            let mut a = (b + 0.5 * (d * d - c * c) - xmin) / (xmax - xmin);
                            // a can be NaN if dy == 0 (since inv_slope will be NaN).
                            // This code changes those NaNs to 0.
                            a = a.abs().max(0.).copysign(a);

                            areas[(x * 4) as usize + y] += a * dy;

                            // Making this branchless doesn't lead to any performance improvements
                            // according to my measurements.
                            if p0.x == 0.0 {
                                areas[(x * 4) as usize + y] +=
                                    (y as f32 - p0.y + 1.0).clamp(0.0, 1.0);
                            } else if p1.x == 0.0 {
                                areas[(x * 4) as usize + y] -=
                                    (y as f32 - p1.y + 1.0).clamp(0.0, 1.0);
                            }
                        }
                    }
                }

                macro_rules! fill {
                    ($rule:expr) => {
                        for x in x0..x1 {
                            let mut alphas = 0u32;

                            for y in 0..4 {
                                let area = areas[(x * 4) as usize + y];
                                let area_u8 = $rule(area);

                                alphas += area_u8 << (y * 8);
                            }

                            alpha_buf.push(alphas);
                        }
                    };
                }

                match fill_rule {
                    FillRule::NonZero => {
                        fill!(|area: f32| (area.abs().min(1.0) * 255.0 + 0.5) as u32)
                    }
                    FillRule::EvenOdd => {
                        fill!(|area: f32| {
                            let area_abs = area.abs();
                            let area_fract = area_abs.fract();
                            let odd = area_abs as i32 & 1;
                            // Even case: 2.68 -> The opacity should be (0 + 0.68) = 68%.
                            // Odd case: 1.68 -> The opacity should be (1 - 0.68) = 32%.
                            // `add_val` represents the 1, sign represents the minus.
                            // If we have for example 2.68, then opacity is 68%, while for
                            // 1.68 it would be (1 - 0.68) = 32%.
                            // So for odd, add_val should be 1, while for even it should be 0.
                            let add_val = odd as f32;
                            // 1 for even, -1 for odd.
                            let sign = -2.0 * add_val + 1.0;
                            let factor = add_val + sign * area_fract;

                            (factor * 255.0 + 0.5) as u32
                        })
                    }
                }

                if strip_start {
                    let strip = Strip {
                        x: 4 * prev_tile.x() + x0 as i32,
                        y: 4 * prev_tile.y() as u32,
                        col: cols,
                        winding: start_delta,
                    };

                    strip_buf.push(strip);
                }

                cols += x1 - x0;
                fp = if same_strip {
                    Footprint::from_index(0)
                } else {
                    Footprint::empty()
                };

                strip_start = !same_strip;
                seg_start = i;

                if !prev_tile.loc().same_row(&tile.loc()) {
                    delta = 0;
                }
            }

            fp.merge(&tile.footprint());

            prev_tile = tile;
        }
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use crate::strip::Strip;
    use crate::tiling::{Footprint, Tile, Tiles};
    use crate::FillRule;
    use std::arch::aarch64::*;

    /// SAFETY: The CPU needs to support the target feature `neon`.
    pub(crate) unsafe fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        // TODO: Clean up and improve this impementation
        let mut strip_start = true;
        let mut cols = alpha_buf.len() as u32;
        let mut prev_tile = tiles.get_tile(0);
        let mut fp = prev_tile.footprint();
        let mut seg_start = 0;
        let mut delta = 0;

        // Note: the input should contain a sentinel tile, to avoid having
        // logic here to process the final strip.
        const IOTA: [f32; 4] = [0.0, 1.0, 2.0, 3.0];
        let iota = vld1q_f32(IOTA.as_ptr());
        for i in 1..tiles.len() {
            let tile = tiles.get_tile(i);
            if prev_tile.loc() != tile.loc() {
                let start_delta = delta;
                let same_strip = prev_tile.loc().same_strip(&tile.loc());

                if same_strip {
                    fp.extend(3);
                }

                let x0 = fp.x0();
                let x1 = fp.x1();
                let mut areas = [[start_delta as f32; 4]; 4];

                for j in seg_start..i {
                    let tile = tiles.get_tile(j);
                    // small gain possible here to unpack in simd, but llvm goes halfway
                    delta += tile.delta();
                    let p0 = tile.p0();
                    let p1 = tile.p1();
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
                    match fill_rule {
                        FillRule::NonZero => {
                            let varea = vld1q_f32(areas.as_ptr().add(x as usize) as *const f32);
                            let vnzw = vminq_f32(vabsq_f32(varea), vdupq_n_f32(1.0));
                            let vscaled = vmulq_f32(vnzw, vdupq_n_f32(255.0));
                            let vbits = vreinterpretq_u8_u32(vcvtnq_u32_f32(vscaled));
                            let vbits2 = vuzp1q_u8(vbits, vbits);
                            let vbits3 = vreinterpretq_u32_u8(vuzp1q_u8(vbits2, vbits2));
                            vst1q_lane_u32::<0>(&mut alphas, vbits3);
                        }
                        FillRule::EvenOdd => {
                            let area_abs =
                                vabsq_f32(vld1q_f32(areas.as_ptr().add(x as usize) as *const f32));
                            let area_fract = vsubq_f32(area_abs, vrndmq_f32(area_abs));
                            let odd = {
                                let im1 = vdupq_n_s32(1);
                                let im2 = vcvtq_s32_f32(area_abs);
                                vandq_s32(im1, im2)
                            };
                            let add_val = vcvtq_f32_s32(odd);
                            let sign = vfmaq_f32(vdupq_n_f32(1.0), vdupq_n_f32(-2.0), add_val);
                            let factor = vfmaq_f32(add_val, sign, area_fract);
                            let res = vreinterpretq_u8_u32(vcvtnq_u32_f32(vmulq_f32(
                                factor,
                                vdupq_n_f32(255.0),
                            )));

                            // Pack into a single u32.
                            let packed1 = vuzp1q_u8(res, res);
                            let packed2 = vreinterpretq_u32_u8(vuzp1q_u8(packed1, packed1));
                            vst1q_lane_u32::<0>(&mut alphas, packed2);
                        }
                    }
                    alpha_buf.push(alphas);
                }

                if strip_start {
                    let strip = Strip {
                        x: 4 * prev_tile.x() + x0 as i32,
                        y: 4 * prev_tile.y() as u32,
                        col: cols,
                        winding: start_delta,
                    };

                    strip_buf.push(strip);
                }

                cols += x1 - x0;
                fp = if same_strip {
                    Footprint::from_index(0)
                } else {
                    Footprint::empty()
                };

                strip_start = !same_strip;
                seg_start = i;

                if !prev_tile.loc().same_row(&tile.loc()) {
                    delta = 0;
                }
            }

            fp.merge(&tile.footprint());

            prev_tile = tile;
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::strip::Strip;
    use crate::tiling::{Footprint, Tiles};
    use crate::FillRule;
    use std::arch::x86_64::*;

    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    unsafe fn clamp(val: __m256, min: f32, max: f32) -> __m256 {
        _mm256_max_ps(_mm256_min_ps(val, _mm256_set1_ps(max)), _mm256_set1_ps(min))
    }

    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    unsafe fn remove_nan(val: __m256) -> __m256 {
        let sign_bit = _mm256_set1_ps(-0.0);
        let abs = _mm256_andnot_ps(sign_bit, val);
        let im2 = _mm256_max_ps(abs, _mm256_set1_ps(0.0));
        let res = _mm256_or_ps(im2, _mm256_and_ps(sign_bit, val));
        res
    }

    /// SAFETY: The CPU needs to support the target feature `avx2`.
    #[target_feature(enable = "avx2")]
    unsafe fn abs_128(val: __m128) -> __m128 {
        let sign_bit = _mm_set1_ps(-0.0);
        _mm_andnot_ps(sign_bit, val)
    }

    /// SAFETY: The CPU needs to support the target feature `avx2` and `fma`.
    #[target_feature(enable = "avx2,fma")]
    pub(crate) unsafe fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        let mut strip_start = true;
        let mut cols = alpha_buf.len() as u32;
        let mut prev_tile = tiles.get_tile(0);
        let mut fp = prev_tile.footprint();
        let mut seg_start = 0;
        let mut delta = 0;

        // Note: the input should contain a sentinel tile, to avoid having
        // logic here to process the final strip.
        for i in 1..tiles.len() {
            let tile = tiles.get_tile(i);

            if prev_tile.loc() != tile.loc() {
                let start_delta = delta;
                let same_strip = prev_tile.loc().same_strip(&tile.loc());

                if same_strip {
                    fp.extend(3);
                }

                let x0 = fp.x0();
                let x1 = fp.x1();
                let mut areas = [start_delta as f32; 16];

                let ones = _mm256_set1_ps(1.0);
                let zeroes = _mm256_set1_ps(0.0);

                for j in seg_start..i {
                    let tile = tiles.get_tile(j);

                    delta += tile.delta();

                    let p0 = tile.p0();
                    let p1 = tile.p1();
                    let inv_slope = _mm256_set1_ps((p1.x - p0.x) / (p1.y - p0.y));

                    let p0_y = _mm256_set1_ps(p0.y);
                    let p0_x = _mm256_set1_ps(p0.x);
                    let p1_y = _mm256_set1_ps(p1.y);

                    for x in 0..2 {
                        let x__ = x * 2;
                        let x_ = x__ as f32;
                        let x =
                            _mm256_set_ps(x_ + 1.0, x_ + 1.0, x_ + 1.0, x_ + 1.0, x_, x_, x_, x_);
                        let rel_x = _mm256_sub_ps(p0_x, x);

                        let y = _mm256_set_ps(3.0, 2.0, 1.0, 0.0, 3.0, 2.0, 1.0, 0.0);
                        let rel_y = _mm256_sub_ps(p0_y, y);
                        let y0 = clamp(rel_y, 0.0, 1.0);
                        let y1 = clamp(_mm256_sub_ps(p1_y, y), 0.0, 1.0);
                        let dy = _mm256_sub_ps(y0, y1);

                        let xx0 = _mm256_fmadd_ps(_mm256_sub_ps(y0, rel_y), inv_slope, rel_x);
                        let xx1 = _mm256_fmadd_ps(_mm256_sub_ps(y1, rel_y), inv_slope, rel_x);
                        let xmin0 = _mm256_min_ps(xx0, xx1);
                        let xmax = _mm256_max_ps(xx0, xx1);
                        let xmin = _mm256_sub_ps(_mm256_min_ps(xmin0, ones), _mm256_set1_ps(1e-6));

                        let b = _mm256_min_ps(xmax, ones);
                        let c = _mm256_max_ps(b, zeroes);
                        let d = _mm256_max_ps(xmin, zeroes);
                        let a = {
                            let im1 = _mm256_mul_ps(d, d);
                            let im2 = _mm256_mul_ps(c, c);
                            let im3 = _mm256_sub_ps(im1, im2);
                            let im4 = _mm256_fmadd_ps(_mm256_set1_ps(0.5), im3, b);
                            let im5 = _mm256_sub_ps(im4, xmin);
                            let im6 = _mm256_div_ps(im5, _mm256_sub_ps(xmax, xmin));
                            remove_nan(im6)
                        };

                        let mut area = _mm256_loadu_ps(areas.as_ptr().add((4 * x__) as usize));
                        area = _mm256_fmadd_ps(a, dy, area);

                        if p0.x == 0.0 {
                            let im1 = clamp(_mm256_add_ps(ones, _mm256_sub_ps(y, p0_y)), 0.0, 1.0);
                            area = _mm256_add_ps(area, im1);
                        } else if p1.x == 0.0 {
                            let im1 = clamp(_mm256_add_ps(ones, _mm256_sub_ps(y, p1_y)), 0.0, 1.0);
                            area = _mm256_sub_ps(area, im1);
                        }

                        _mm256_storeu_ps(areas.as_mut_ptr().add((4 * x__) as usize), area);
                    }
                }

                macro_rules! fill {
                    ($rule:expr) => {
                        for x in x0..x1 {
                            let area_u32 = $rule((x * 4) as usize);

                            alpha_buf.push(area_u32);
                        }
                    };
                }

                match fill_rule {
                    FillRule::NonZero => {
                        fill!(|idx: usize| {
                            let area = _mm_loadu_ps(areas.as_ptr().add(idx));
                            let abs = abs_128(area);
                            let minned = _mm_min_ps(abs, _mm_set1_ps(1.0));
                            let mulled = _mm_mul_ps(minned, _mm_set1_ps(255.0));
                            let added = _mm_round_ps::<0b1000>(mulled);
                            let converted = _mm_cvtps_epi32(added);

                            let shifted = _mm_packus_epi16(converted, converted);
                            let shifted = _mm_packus_epi16(shifted, shifted);
                            _mm_extract_epi32::<0>(shifted) as u32
                        })
                    }

                    FillRule::EvenOdd => {
                        fill!(|idx: usize| {
                            let area = _mm_loadu_ps(areas.as_ptr().add(idx));
                            let area_abs = abs_128(area);
                            let floored = _mm_floor_ps(area_abs);
                            let area_fract = _mm_sub_ps(area_abs, floored);
                            let odd = _mm_and_si128(_mm_set1_epi32(1), _mm_cvtps_epi32(floored));
                            let add_val = _mm_cvtepi32_ps(odd);
                            let sign = _mm_fmadd_ps(_mm_set1_ps(-2.0), add_val, _mm_set1_ps(1.0));
                            let factor = _mm_fmadd_ps(sign, area_fract, add_val);
                            let rounded =
                                _mm_round_ps::<0b1000>(_mm_mul_ps(factor, _mm_set1_ps(255.0)));
                            let converted = _mm_cvtps_epi32(rounded);

                            let shifted = _mm_packus_epi16(converted, converted);
                            let shifted = _mm_packus_epi16(shifted, shifted);
                            _mm_extract_epi32::<0>(shifted) as u32
                        })
                    }
                }

                if strip_start {
                    let strip = Strip {
                        x: 4 * prev_tile.x() + x0 as i32,
                        y: 4 * prev_tile.y() as u32,
                        col: cols,
                        winding: start_delta,
                    };

                    strip_buf.push(strip);
                }

                cols += x1 - x0;
                fp = if same_strip {
                    Footprint::from_index(0)
                } else {
                    Footprint::empty()
                };

                strip_start = !same_strip;
                seg_start = i;

                if !prev_tile.loc().same_row(&tile.loc()) {
                    delta = 0;
                }
            }

            fp.merge(&tile.footprint());

            prev_tile = tile;
        }
    }

    // fn calculate_areas(tiles: &Tiles, areas: [f32; 16], )
}
