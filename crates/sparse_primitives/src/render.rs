// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Lots of unused arguments from todo methods. Remove when all methods are implemented.
#![allow(unused)]

use crate::paint::Paint;
use crate::rect::lines_to_rect;
use crate::strip::render_strips;
use crate::tiling::{Point, Tile, Tiles};
use crate::{
    fine::Fine,
    strip::{self, Strip},
    tiling::{self, FlatLine},
    wide_tile::{Cmd, CmdStrip, WideTile, STRIP_HEIGHT, WIDE_TILE_WIDTH},
    FillRule, Pixmap,
};
use peniko::kurbo::{BezPath, Rect, Shape};
use peniko::{
    color::{palette, AlphaColor, Srgb},
    kurbo,
    kurbo::Affine,
    BrushRef,
};
use std::collections::BTreeMap;

pub(crate) const DEFAULT_TOLERANCE: f64 = 0.1;

pub struct RenderContext {
    pub width: usize,
    pub height: usize,
    pub wide_tiles: Vec<WideTile>,
    pub alphas: Vec<u32>,

    /// These are all scratch buffers, to be used for path rendering. They're here solely
    /// so the allocations can be reused.
    pub line_buf: Vec<FlatLine>,
    pub tiles: Tiles,
    pub strip_buf: Vec<Strip>,
    #[cfg(feature = "simd")]
    use_simd: bool,

    transform: Affine,
}

impl RenderContext {
    /// Create a new render context.
    pub fn new(width: usize, height: usize) -> Self {
        let width_tiles = (width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        let mut wide_tiles = Vec::with_capacity(width_tiles * height_tiles);

        for w in 0..width_tiles {
            for h in 0..height_tiles {
                wide_tiles.push(WideTile::new(w * WIDE_TILE_WIDTH, h * STRIP_HEIGHT));
            }
        }

        let alphas = vec![];
        let line_buf = vec![];
        let tiles = Tiles::new();
        let strip_buf = vec![];
        Self {
            width,
            height,
            wide_tiles,
            alphas,
            line_buf,
            tiles,
            strip_buf,
            #[cfg(feature = "simd")]
            use_simd: option_env!("SIMD").is_some(),
            transform: Affine::IDENTITY,
        }
    }

    /// Reset the current render context.
    pub fn reset(&mut self) {
        for tile in &mut self.wide_tiles {
            tile.bg = AlphaColor::TRANSPARENT;
            tile.cmds.clear();
        }
    }

    /// Render the current render context into a pixmap.
    pub fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        let mut fine = Fine::new(
            pixmap.width,
            pixmap.height,
            &mut pixmap.buf,
            #[cfg(feature = "simd")]
            self.use_simd,
        );

        let width_tiles = (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (self.height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        for y in 0..height_tiles {
            for x in 0..width_tiles {
                let tile = &self.wide_tiles[y * width_tiles + x];
                fine.clear(tile.bg.premultiply().to_rgba8().to_u8_array());
                for cmd in &tile.cmds {
                    fine.run_cmd(cmd, &self.alphas);
                }
                fine.pack(x, y);
            }
        }
    }

    fn render_path(&mut self, fill_rule: FillRule, paint: Paint) {
        if let Some(rect) = lines_to_rect(&self.line_buf, self.width, self.height) {
            // Path is actually a rectangle, so used fast path for rectangles.
            self.render_filled_rect(&rect, paint);
        } else {
            self.tiles.make_tiles(&self.line_buf);
            self.tiles.sort_tiles();

            render_strips(
                &self.tiles,
                &mut self.strip_buf,
                &mut self.alphas,
                fill_rule,
                #[cfg(feature = "simd")]
                self.use_simd,
            );

            self.generate_commands(fill_rule, paint);
        }
    }

    fn wide_tiles_per_row(&self) -> usize {
        (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH
    }

    /// Generate the strip and fill commands for each wide tile using the current `strip_buf`.
    pub(crate) fn generate_commands(&mut self, fill_rule: FillRule, paint: Paint) {
        let width_tiles = self.wide_tiles_per_row();

        if self.strip_buf.is_empty() {
            return;
        }

        for i in 0..self.strip_buf.len() - 1 {
            let strip = &self.strip_buf[i];

            if strip.x() >= self.width as i32 || strip.y() < 0 {
                // Don't render strips that are outside the viewport.
                continue;
            }

            if strip.y() >= self.height as u32 {
                // Since strips are sorted by location, any subsequent strips will also be
                // outside the viewport, so we can abort entirely.
                break;
            }

            let next_strip = &self.strip_buf[i + 1];
            // Currently, strips can also start at a negative x position, since we don't
            // support viewport culling yet. However, when generating the commands
            // we only want to emit strips >= 0, so we calculate the adjustment
            // and then only include the alpha indices for columns where x >= 0.
            let x0_adjustment = (strip.x()).min(0).abs() as u32;
            let x0 = (strip.x() + x0_adjustment as i32) as u32;
            let y = strip.strip_y();
            let row_start = y as usize * width_tiles;
            let mut col = strip.col + x0_adjustment;
            // Can potentially be 0, if the next strip's x values is also < 0.
            let strip_width = next_strip.col.saturating_sub(col);
            let x1 = x0 + strip_width;
            let xtile0 = x0 as usize / WIDE_TILE_WIDTH;
            // It's possible that a strip extends into a new wide tile, but we don't actually
            // have as many wide tiles (e.g. because the pixmap width is only 512, but
            // strip ends at 513), so take the minimum between the rounded values and `width_tiles`.
            let xtile1 = ((x1 as usize + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH).min(width_tiles);
            let mut x = x0;

            for xtile in xtile0..xtile1 {
                let x_tile_rel = x % WIDE_TILE_WIDTH as u32;
                let width = x1.min(((xtile + 1) * WIDE_TILE_WIDTH) as u32) - x;
                let cmd = CmdStrip {
                    x: x_tile_rel,
                    width,
                    alpha_ix: col as usize,
                    paint: paint.clone(),
                };
                x += width;
                col += width;
                self.wide_tiles[row_start + xtile].push(Cmd::Strip(cmd));
            }

            if fill_rule.active_fill(next_strip.winding)
                && y == next_strip.strip_y()
                // Only fill if we are actually inside the viewport.
                && next_strip.x() >= 0
            {
                x = x1;
                let x2 = next_strip.x() as u32;
                let fxt0 = x1 as usize / WIDE_TILE_WIDTH;
                let fxt1 = (x2 as usize + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
                for xtile in fxt0..fxt1 {
                    let x_tile_rel = x % WIDE_TILE_WIDTH as u32;
                    let width = x2.min(((xtile + 1) * WIDE_TILE_WIDTH) as u32) - x;
                    x += width;
                    self.wide_tiles[row_start + xtile].fill(x_tile_rel, width, paint.clone());
                }
            }
        }
    }

    /// Fill a path.
    pub fn fill_path(&mut self, path: &Path, fill_rule: FillRule, paint: Paint) {
        let affine = self.current_transform();
        crate::flatten::fill(&path.path, affine, &mut self.line_buf);
        self.render_path(fill_rule, paint);
    }

    /// Stroke a path.
    pub fn stroke_path(&mut self, path: &Path, stroke: &kurbo::Stroke, paint: Paint) {
        let affine = self.current_transform();
        crate::flatten::stroke(&path.path, stroke, affine, &mut self.line_buf);
        self.render_path(FillRule::NonZero, paint);
    }

    /// Pre-concatenate a new transform to the current transformation matrix.
    pub fn transform(&mut self, transform: Affine) {
        self.transform = self.transform * transform;
    }

    /// Set the current transformation matrix.
    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    /// Set the current transformation matrix.
    pub fn reset_transform(&mut self) {
        self.transform = Affine::IDENTITY;
    }

    /// Return the current transformation matrix.
    pub fn current_transform(&self) -> Affine {
        self.transform
    }
}

#[derive(Clone)]
pub struct Path {
    pub path: BezPath,
}

impl From<BezPath> for Path {
    fn from(value: BezPath) -> Self {
        Self { path: value }
    }
}

/// Get the color from the brush.
///
/// This is a hacky function that will go away when we implement
/// other brushes. The general form is to match on whether it's a
/// solid color. If not, then issue a cmd to render the brush into
/// a brush buffer, then fill/strip as needed to composite into
/// the main buffer.
fn brush_to_color(brush: BrushRef) -> AlphaColor<Srgb> {
    match brush {
        BrushRef::Solid(c) => c,
        _ => palette::css::MAGENTA,
    }
}
