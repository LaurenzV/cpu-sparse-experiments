// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::execute::KernelExecutor;
use crate::util::ColorExt;
use crate::{
    fine::Fine,
    wide_tile::{Cmd, CmdStrip, WideTile, STRIP_HEIGHT, WIDE_TILE_WIDTH},
    Pixmap,
};
use std::marker::PhantomData;
use vello_common::color::palette::css::BLACK;
use vello_common::color::AlphaColor;
use vello_common::flatten;
use vello_common::flatten::Line;
use vello_common::kurbo::{Affine, BezPath, Cap, Join, Stroke};
use vello_common::paint::Paint;
use vello_common::peniko::{BlendMode, Compose, Fill, Mix};
use vello_common::strip::Strip;
use vello_common::tile::Tiles;
use crate::wide_tile::generate_commands;

pub(crate) const DEFAULT_TOLERANCE: f64 = 0.1;

pub(crate) struct InnerContext<KE: KernelExecutor> {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) wide_tiles: Vec<WideTile>,
    pub(crate) alphas: Vec<u32>,
    pub(crate) line_buf: Vec<Line>,
    pub(crate) tiles: Tiles,
    pub(crate) strip_buf: Vec<Strip>,
    pub(crate) paint: Paint,
    pub(crate) stroke: Stroke,
    pub(crate) transform: Affine,
    pub(crate) fill_rule: Fill,
    pub(crate) blend_mode: BlendMode,
    phantom_data: PhantomData<KE>,
}

impl<KE: KernelExecutor> InnerContext<KE> {
    pub fn new(width: usize, height: usize) -> Self {
        let width_tiles = width.div_ceil(WIDE_TILE_WIDTH);
        let height_tiles = height.div_ceil(STRIP_HEIGHT);
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

        let transform = Affine::IDENTITY;
        let fill_rule = Fill::NonZero;
        let paint = BLACK.into();
        let stroke = Stroke {
            width: 1.0,
            join: Join::Bevel,
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            ..Default::default()
        };
        let blend_mode = BlendMode::new(Mix::Normal, Compose::SrcOver);

        Self {
            width,
            height,
            wide_tiles,
            alphas,
            line_buf,
            tiles,
            strip_buf,
            transform,
            paint,
            fill_rule,
            stroke,
            blend_mode,
            phantom_data: Default::default(),
        }
    }

    pub(crate) fn fill_path(&mut self, path: &BezPath) {
        flatten::fill(&path, self.transform, &mut self.line_buf);
        self.render_path(self.fill_rule, self.paint.clone());
    }

    pub(crate) fn stroke_path(&mut self, path: &BezPath) {
        flatten::stroke(&path, &self.stroke, self.transform, &mut self.line_buf);
        self.render_path(Fill::NonZero, self.paint.clone());
    }

    pub(crate) fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        self.blend_mode = blend_mode;
    }

    pub(crate) fn set_stroke(&mut self, stroke: Stroke) {
        self.stroke = stroke;
    }

    pub(crate) fn set_paint(&mut self, paint: Paint) {
        self.paint = paint;
    }

    pub(crate) fn set_fill_rule(&mut self, fill_rule: Fill) {
        self.fill_rule = fill_rule;
    }

    pub(crate) fn pre_concat_transform(&mut self, transform: Affine) {
        self.transform *= transform;
    }

    pub(crate) fn post_concat_transform(&mut self, transform: Affine) {
        self.transform = transform * self.transform;
    }

    pub(crate) fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    pub(crate) fn reset_transform(&mut self) {
        self.transform = Affine::IDENTITY;
    }

    pub(crate) fn current_transform(&self) -> Affine {
        self.transform
    }

    pub(crate) fn reset(&mut self) {
        for tile in &mut self.wide_tiles {
            tile.bg = AlphaColor::TRANSPARENT;
            tile.cmds.clear();
        }
    }

    pub(crate) fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        let mut fine = Fine::<KE>::new(pixmap.width, pixmap.height, &mut pixmap.buf);

        let width_tiles = self.width.div_ceil(WIDE_TILE_WIDTH);
        let height_tiles = self.height.div_ceil(STRIP_HEIGHT);
        for y in 0..height_tiles {
            for x in 0..width_tiles {
                let tile = &self.wide_tiles[y * width_tiles + x];
                fine.clear(tile.bg.premultiply().to_rgba8_fast());
                for cmd in &tile.cmds {
                    fine.run_cmd(cmd, &self.alphas, Compose::SrcOver);
                }
                fine.pack(x, y);
            }
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn height(&self) -> usize {
        self.height
    }

    pub(crate) fn wide_tiles(&self) -> &[WideTile] {
        &self.wide_tiles
    }

    pub(crate) fn alphas(&self) -> &[u32] {
        &self.alphas
    }

    pub(crate) fn line_buf(&self) -> &[Line] {
        &self.line_buf
    }

    pub(crate) fn tiles(&self) -> &Tiles {
        &self.tiles
    }

    pub(crate) fn strip_buf(&self) -> &[Strip] {
        &self.strip_buf
    }

    fn render_path(&mut self, fill_rule: Fill, paint: Paint) {
        self.tiles.make_tiles(&self.line_buf);
        self.tiles.sort_tiles();

        KE::render_strips(
            &self.tiles,
            &mut self.strip_buf,
            &mut self.alphas,
            fill_rule,
        );

        self.generate_commands(fill_rule, paint);
    }

    /// Generate the strip and fill commands for each wide tile using the current `strip_buf`.
    pub(crate) fn generate_commands(&mut self, fill_rule: Fill, paint: Paint) {
        generate_commands(&self.strip_buf, &mut self.wide_tiles, fill_rule, paint, self.width, self.height);
    }
}
