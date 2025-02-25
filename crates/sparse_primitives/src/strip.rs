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

use crate::execute::{KernelExecutor, Scalar};
use crate::tiling::{Footprint, Tiles};
use crate::wide_tile::STRIP_HEIGHT;
use crate::FillRule;

#[derive(Debug, Clone, Copy)]
pub struct Strip {
    pub x: i32,
    pub y: u32,
    pub col: u32,
    pub winding: i32,
}

pub trait RenderStrip {
    fn calculate_areas(
        tiles: &Tiles,
        tile_start: u32,
        tile_end: u32,
        x0: u32,
        x1: u32,
        areas: &mut [f32; 16],
        delta: &mut i32,
    );
    fn fill_alphas(
        areas: &[f32; 16],
        alpha_buf: &mut Vec<u32>,
        x0: u32,
        x1: u32,
        fill_rule: FillRule,
    );
}

#[inline(never)]
pub fn render_strips<KE: KernelExecutor>(
    tiles: &Tiles,
    strip_buf: &mut Vec<Strip>,
    alpha_buf: &mut Vec<u32>,
    fill_rule: FillRule,
) {
    strip_buf.clear();

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

            KE::calculate_areas(tiles, seg_start, i, x0, x1, &mut areas, &mut delta);
            KE::fill_alphas(&areas, alpha_buf, x0, x1, fill_rule);

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
    use crate::execute::Scalar;
    use crate::strip::{RenderStrip, Strip};
    use crate::tiling::{Footprint, Tiles};
    use crate::FillRule;

    impl RenderStrip for Scalar {
        #[inline(always)]
        fn calculate_areas(
            tiles: &Tiles,
            tile_start: u32,
            tile_end: u32,
            x0: u32,
            x1: u32,
            areas: &mut [f32; 16],
            delta: &mut i32,
        ) {
            for j in tile_start..tile_end {
                let tile = tiles.get_tile(j);

                *delta += tile.delta();

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
                    let areas = &mut areas[(x * 4) as usize..][..4];

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

                        areas[y] += a * dy;

                        // Making this branchless doesn't lead to any performance improvements
                        // according to my measurements on x86. Haven't checked ARM.
                        if p0.x == 0.0 {
                            areas[y] += (y as f32 - p0.y + 1.0).clamp(0.0, 1.0);
                        } else if p1.x == 0.0 {
                            areas[y] -= (y as f32 - p1.y + 1.0).clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }

        #[inline(always)]
        fn fill_alphas(
            areas: &[f32; 16],
            alpha_buf: &mut Vec<u32>,
            x0: u32,
            x1: u32,
            fill_rule: FillRule,
        ) {
            macro_rules! fill {
                ($rule:expr) => {
                    for x in x0..x1 {
                        let mut alphas = 0u32;
                        let areas = &areas[(x * 4) as usize..][..4];

                        for y in 0..4 {
                            let area = areas[y];
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
        }
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) mod neon {
    use crate::execute::{Neon, Scalar};
    use crate::strip::RenderStrip;
    use crate::tiling::Tiles;
    use crate::FillRule;
    use std::arch::aarch64::*;

    impl RenderStrip for Neon {
        /// SAFETY: The CPU needs to support the target feature `neon`.
        #[inline(always)]
        fn calculate_areas(
            tiles: &Tiles,
            tile_start: u32,
            tile_end: u32,
            x0: u32,
            x1: u32,
            areas: &mut [f32; 16],
            delta: &mut i32,
        ) {
            unsafe {
                let ones = vdupq_n_f32(1.0);
                let zeroes = vdupq_n_f32(0.0);

                for j in tile_start..tile_end {
                    let tile = tiles.get_tile(j);

                    *delta += tile.delta();

                    let p0 = tile.p0();
                    let p1 = tile.p1();
                    let inv_slope = vdupq_n_f32((p1.x - p0.x) / (p1.y - p0.y));

                    let p0_y = vdupq_n_f32(p0.y);
                    let p0_x = vdupq_n_f32(p0.x);
                    let p1_y = vdupq_n_f32(p1.y);

                    for x in x0..x1 {
                        let x_ = vdupq_n_f32(x as f32);
                        let rel_x = vsubq_f32(p0_x, x_);

                        let y = vld1q_f32([0.0, 1.0, 2.0, 3.0].as_ptr());
                        let rel_y = vsubq_f32(p0_y, y);
                        let y0 = clamp(rel_y, 0.0, 1.0);
                        let y1 = clamp(vsubq_f32(p1_y, y), 0.0, 1.0);
                        let dy = vsubq_f32(y0, y1);

                        let xx0 = vfmaq_f32(rel_x, inv_slope, vsubq_f32(y0, rel_y));
                        let xx1 = vfmaq_f32(rel_x, inv_slope, vsubq_f32(y1, rel_y));
                        let xmin0 = vminq_f32(xx0, xx1);
                        let xmax = vmaxq_f32(xx0, xx1);
                        let xmin = vsubq_f32(vminq_f32(xmin0, ones), vdupq_n_f32(1e-6));

                        let b = vminq_f32(xmax, ones);
                        let c = vmaxq_f32(b, zeroes);
                        let d = vmaxq_f32(xmin, zeroes);
                        let a = {
                            let im1 = vmulq_f32(d, d);
                            let im2 = vmulq_f32(c, c);
                            let im3 = vsubq_f32(im1, im2);
                            let im4 = vfmaq_f32(b, im3, vdupq_n_f32(0.5));
                            let im5 = vsubq_f32(im4, xmin);
                            let im6 = vdivq_f32(im5, vsubq_f32(xmax, xmin));
                            remove_nan(im6)
                        };

                        let mut area = vld1q_f32(areas.as_ptr().add((4 * x) as usize));
                        area = vfmaq_f32(area, dy, a);

                        if p0.x == 0.0 {
                            let im1 = clamp(vaddq_f32(ones, vsubq_f32(y, p0_y)), 0.0, 1.0);
                            area = vaddq_f32(area, im1);
                        } else if p1.x == 0.0 {
                            let im1 = clamp(vaddq_f32(ones, vsubq_f32(y, p1_y)), 0.0, 1.0);
                            area = vsubq_f32(area, im1);
                        }

                        vst1q_f32(areas.as_mut_ptr().add((4 * x) as usize), area);
                    }
                }
            }
        }

        /// SAFETY: The CPU needs to support the target feature `neon`.
        #[inline(always)]
        fn fill_alphas(
            areas: &[f32; 16],
            alpha_buf: &mut Vec<u32>,
            x0: u32,
            x1: u32,
            fill_rule: FillRule,
        ) {
            unsafe {
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
                            let area = vld1q_f32(areas.as_ptr().add(idx));
                            let abs = vabsq_f32(area);
                            let minned = vminq_f32(abs, vdupq_n_f32(1.0));
                            let mulled = vmulq_f32(minned, vdupq_n_f32(255.0));
                            let rounded = vrndnq_f32(mulled);
                            let converted = vcvtq_u32_f32(rounded);

                            let shifted = vmovn_u32(converted);
                            let shifted = vmovn_u16(vcombine_u16(shifted, vdup_n_u16(0)));
                            vget_lane_u32::<0>(vreinterpret_u32_u8(shifted))
                        })
                    }
                    FillRule::EvenOdd => {
                        fill!(|idx: usize| {
                            let area = vld1q_f32(areas.as_ptr().add(idx));
                            let area_abs = vabsq_f32(area);
                            let floored = vrndmq_f32(area_abs);
                            let area_fract = vsubq_f32(area_abs, floored);
                            let odd = vandq_u32(vdupq_n_u32(1), vcvtq_u32_f32(floored));

                            let add_val = vcvtq_f32_u32(odd);
                            let sign = vfmaq_n_f32(vdupq_n_f32(1.0), add_val, -2.0);
                            let factor = vfmaq_f32(add_val, area_fract, sign);
                            let rounded = vrndnq_f32(vmulq_f32(factor, vdupq_n_f32(255.0)));
                            let converted = vcvtq_u32_f32(rounded);

                            let shifted = vmovn_u32(converted);
                            let shifted = vmovn_u16(vcombine_u16(shifted, vdup_n_u16(0)));
                            vget_lane_u32::<0>(vreinterpret_u32_u8(shifted))
                        })
                    }
                }
            }
        }
    }

    unsafe fn remove_nan(val: float32x4_t) -> float32x4_t {
        let mask = vceqq_f32(val, val);

        vreinterpretq_f32_u32(vandq_u32(vreinterpretq_u32_f32(val), mask))
    }

    unsafe fn clamp(val: float32x4_t, min: f32, max: f32) -> float32x4_t {
        vmaxq_f32(vminq_f32(val, vdupq_n_f32(max)), vdupq_n_f32(min))
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) mod avx2 {
    use crate::execute::Avx2;
    use crate::strip::{RenderStrip, Strip};
    use crate::tiling::{Footprint, Tiles};
    use crate::FillRule;
    use std::arch::x86_64::*;

    impl RenderStrip for Avx2 {
        /// SAFETY: The CPU needs to support the target feature `avx2` and `fma`.
        #[inline(always)]
        fn calculate_areas(
            tiles: &Tiles,
            tile_start: u32,
            tile_end: u32,
            _: u32,
            _: u32,
            areas: &mut [f32; 16],
            delta: &mut i32,
        ) {
            unsafe {
                let ones = _mm256_set1_ps(1.0);
                let zeroes = _mm256_set1_ps(0.0);

                for j in tile_start..tile_end {
                    let tile = tiles.get_tile(j);

                    *delta += tile.delta();

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
            }
        }

        /// SAFETY: The CPU needs to support the target feature `avx2` and `fma`.
        #[inline(always)]
        fn fill_alphas(
            areas: &[f32; 16],
            alpha_buf: &mut Vec<u32>,
            x0: u32,
            x1: u32,
            fill_rule: FillRule,
        ) {
            unsafe {
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
            }
        }
    }

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
}
