// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Lots of unused arguments from todo methods. Remove when all methods are implemented.
#![allow(unused)]

use crate::paint::Paint;
use crate::strip::render_strips_scalar;
use crate::{
    fine::Fine,
    strip::{self, Strip, Tile},
    tiling::{self, FlatLine},
    wide_tile::{Cmd, CmdStrip, WideTile, STRIP_HEIGHT, WIDE_TILE_WIDTH},
    FillRule, Pixmap,
};
use peniko::kurbo::{BezPath, Rect};
use peniko::{
    color::{palette, AlphaColor, Srgb},
    kurbo::Affine,
    BrushRef,
};
use std::collections::BTreeMap;

pub struct RenderContext {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<WideTile>,
    pub alphas: Vec<u32>,

    /// These are all scratch buffers, to be used for path rendering. They're here solely
    /// so the allocations can be reused.
    pub line_buf: Vec<FlatLine>,
    pub tile_buf: Vec<Tile>,
    pub strip_buf: Vec<Strip>,

    transform: Affine,
}

impl RenderContext {
    /// Create a new render context.
    pub fn new(width: usize, height: usize) -> Self {
        let width_tiles = (width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        let mut tiles = Vec::with_capacity(width_tiles * height_tiles);

        for w in 0..width_tiles {
            for h in 0..height_tiles {
                tiles.push(WideTile::new(w * WIDE_TILE_WIDTH, h * STRIP_HEIGHT));
            }
        }

        let alphas = vec![];
        let line_buf = vec![];
        let tile_buf = vec![];
        let strip_buf = vec![];
        Self {
            width,
            height,
            tiles,
            alphas,
            line_buf,
            tile_buf,
            strip_buf,
            transform: Affine::IDENTITY,
        }
    }

    /// Reset the current render context.
    pub fn reset(&mut self) {
        for tile in &mut self.tiles {
            tile.bg = AlphaColor::TRANSPARENT;
            tile.cmds.clear();
        }
    }

    /// Render the current render context into a pixmap.
    pub fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        let mut fine = Fine::new(pixmap.width, pixmap.height, &mut pixmap.buf);
        let width_tiles = (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (self.height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        for y in 0..height_tiles {
            for x in 0..width_tiles {
                let tile = &self.tiles[y * width_tiles + x];
                fine.clear_scalar(tile.bg.premultiply().to_rgba8().to_u8_array());
                for cmd in &tile.cmds {
                    fine.run_cmd(cmd, &self.alphas);
                }
                fine.pack_scalar(x, y);
            }
        }
    }

    /// Render a path, which has already been flattened into `line_buf`.
    fn render_path(&mut self, fill_rule: FillRule, paint: Paint) {
        tiling::make_tiles(&self.line_buf, &mut self.tile_buf);
        self.tile_buf.sort_unstable_by(Tile::cmp);

        render_strips_scalar(
            &self.tile_buf,
            &mut self.strip_buf,
            &mut self.alphas,
            fill_rule,
        );
        self.generate_commands(fill_rule, paint);
    }

    fn wide_tiles_per_row(&self) -> usize {
        (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH
    }

    /// Generate the strip and fill commands for each wide tile using the current `strip_buf`.
    fn generate_commands(&mut self, fill_rule: FillRule, paint: Paint) {
        let width_tiles = self.wide_tiles_per_row();

        for i in 0..self.strip_buf.len() - 1 {
            let strip = &self.strip_buf[i];

            if strip.x() as usize >= self.width {
                // Don't render strips that are outside the viewport.
                continue;
            }

            if strip.y() as usize >= self.height {
                // Since strips are sorted by location, any subsequent strips will also be
                // outside the viewport, so we can abort entirely.
                break;
            }

            let next_strip = &self.strip_buf[i + 1];
            let x0 = strip.x();
            let y = strip.strip_y();
            let row_start = y as usize * width_tiles;
            let strip_width = next_strip.col - strip.col;
            let x1 = x0 + strip_width;
            let xtile0 = x0 as usize / WIDE_TILE_WIDTH;
            let xtile1 = (x1 as usize + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
            let mut x = x0;
            let mut col = strip.col;

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
                self.tiles[row_start + xtile].push(Cmd::Strip(cmd));
            }

            if fill_rule.active_fill(next_strip.winding) && y == next_strip.strip_y() {
                x = x1;
                let x2 = next_strip.x();
                let fxt0 = x1 as usize / WIDE_TILE_WIDTH;
                let fxt1 = (x2 as usize + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
                for xtile in fxt0..fxt1 {
                    let x_tile_rel = x % WIDE_TILE_WIDTH as u32;
                    let width = x2.min(((xtile + 1) * WIDE_TILE_WIDTH) as u32) - x;
                    x += width;
                    self.tiles[row_start + xtile].fill(x_tile_rel, width, paint.clone());
                }
            }
        }
    }

    /// Fill a path.
    pub fn fill(&mut self, path: &Path, fill_rule: FillRule, paint: Paint) {
        let affine = self.current_transform();
        crate::flatten::fill(&path.path, affine, &mut self.line_buf);
        self.render_path(fill_rule, paint);
    }

    /// Stroke a path.
    pub fn stroke(&mut self, path: &Path, stroke: &peniko::kurbo::Stroke, paint: Paint) {
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
