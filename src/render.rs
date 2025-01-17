// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Lots of unused arguments from todo methods. Remove when all methods are implemented.
#![allow(unused)]

use peniko::kurbo::BezPath;
use peniko::{
    color::{palette, AlphaColor, Srgb},
    kurbo::Affine,
    BrushRef,
};
use std::collections::BTreeMap;

use crate::{
    fine::Fine,
    strip::{self, Strip, Tile},
    tiling::{self, FlatLine},
    wide_tile::{Cmd, CmdStrip, WideTile, STRIP_HEIGHT, WIDE_TILE_WIDTH},
    FillRule, Pixmap,
};

pub struct CsRenderCtx {
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

pub struct CsResourceCtx;

impl CsRenderCtx {
    pub fn new(width: usize, height: usize) -> Self {
        let width_tiles = (width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        let tiles = (0..width_tiles * height_tiles)
            .map(|_| WideTile::default())
            .collect();
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

    pub fn reset(&mut self) {
        for tile in &mut self.tiles {
            tile.bg = AlphaColor::TRANSPARENT;
            tile.cmds.clear();
        }
    }

    pub fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        let mut fine = Fine::new(pixmap.width, pixmap.height, &mut pixmap.buf);
        let width_tiles = (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        let height_tiles = (self.height + STRIP_HEIGHT - 1) / STRIP_HEIGHT;
        for y in 0..height_tiles {
            for x in 0..width_tiles {
                let tile = &self.tiles[y * width_tiles + x];
                fine.clear(tile.bg.components);
                for cmd in &tile.cmds {
                    fine.run_cmd(cmd, &self.alphas);
                }
                fine.pack(x, y);
            }
        }
    }

    pub fn tile_stats(&self) {
        let mut histo = BTreeMap::new();
        let mut total = 0;
        for tile in &self.tiles {
            let count = tile.cmds.len();
            total += count;
            *histo.entry(count).or_insert(0) += 1;
        }
        println!("total = {total}, {histo:?}");
    }

    /// Render a path, which has already been flattened into `line_buf`.
    fn render_path(&mut self, fill_rule: FillRule, brush: BrushRef) {
        tiling::make_tiles(&self.line_buf, &mut self.tile_buf);
        println!("{:?}", self.tile_buf.len());
        self.tile_buf.sort_unstable_by(Tile::cmp);

        crate::simd::render_strips(&self.tile_buf, &mut self.strip_buf, &mut self.alphas);
        let color = brush_to_color(brush);
        let width_tiles = (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        for i in 0..self.strip_buf.len() - 1 {
            let strip = &self.strip_buf[i];

            // Don't render strips that are outside the viewport vertically.
            if strip.y() as usize >= self.height {
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
                    color,
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
                    self.tiles[row_start + xtile].fill(x_tile_rel, width, color);
                }
            }
        }
    }

    pub fn debug_dump(&self) {
        let width_tiles = (self.width + WIDE_TILE_WIDTH - 1) / WIDE_TILE_WIDTH;
        for (i, tile) in self.tiles.iter().enumerate() {
            if !tile.cmds.is_empty() || tile.bg.components[3] != 0.0 {
                let x = i % width_tiles;
                let y = i / width_tiles;
            }
        }
    }

    pub fn fill(&mut self, path: &Path, fill_rule: FillRule, brush: BrushRef) {
        let affine = self.get_affine();
        crate::flatten::fill(&path.path, affine, &mut self.line_buf);
        self.render_path(fill_rule, brush);
    }

    pub fn stroke(&mut self, path: &Path, stroke: &peniko::kurbo::Stroke, brush: BrushRef) {
        let affine = self.get_affine();
        crate::flatten::stroke(&path.path, stroke, affine, &mut self.line_buf);
        self.render_path(FillRule::NonZero, brush);
    }

    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    fn get_affine(&self) -> Affine {
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
