// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use vello_common::color::{AlphaColor, Srgb};
use vello_common::paint::Paint;
use vello_common::peniko::{Compose, Fill};
use vello_common::strip::Strip;

pub const WIDE_TILE_WIDTH: usize = 256;
pub const STRIP_HEIGHT: usize = 4;

#[derive(Debug)]
pub struct WideTile {
    pub x: usize,
    pub y: usize,
    pub bg: AlphaColor<Srgb>,
    pub cmds: Vec<Cmd>,
}

pub(crate) fn generate_commands(strip_buf: &[Strip], wide_tiles: &mut [WideTile], fill_rule: Fill, paint: Paint, width: usize, height: usize) {
    let width_tiles = width.div_ceil(WIDE_TILE_WIDTH);

    if strip_buf.is_empty() {
        return;
    }

    for i in 0..strip_buf.len() - 1 {
        let strip = &strip_buf[i];

        if strip.x >= width as i32 {
            // Don't render strips that are outside the viewport.
            continue;
        }

        if strip.y >= height as u16 {
            // Since strips are sorted by location, any subsequent strips will also be
            // outside the viewport, so we can abort entirely.
            break;
        }

        let next_strip = &strip_buf[i + 1];
        // Currently, strips can also start at a negative x position, since we don't
        // support viewport culling yet. However, when generating the commands
        // we only want to emit strips >= 0, so we calculate the adjustment
        // and then only include the alpha indices for columns where x >= 0.
        let x0_adjustment = (strip.x).min(0).unsigned_abs();
        let x0 = (strip.x + x0_adjustment as i32) as u32;
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
        let xtile1 = (x1 as usize).div_ceil(WIDE_TILE_WIDTH).min(width_tiles);
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
            wide_tiles[row_start + xtile].push(Cmd::Strip(cmd));
        }

        let active_fill = match fill_rule {
            Fill::NonZero => next_strip.winding != 0,
            Fill::EvenOdd => next_strip.winding % 2 != 0,
        };

        if active_fill
            && y == next_strip.strip_y()
            // Only fill if we are actually inside the viewport.
            && next_strip.x >= 0
        {
            x = x1;
            let x2 = next_strip.x as u32;
            let fxt0 = x1 as usize / WIDE_TILE_WIDTH;
            let fxt1 = (x2 as usize).div_ceil(WIDE_TILE_WIDTH);
            for xtile in fxt0..fxt1 {
                let x_tile_rel = x % WIDE_TILE_WIDTH as u32;
                let width = x2.min(((xtile + 1) * WIDE_TILE_WIDTH) as u32) - x;
                x += width;
                wide_tiles[row_start + xtile].fill(
                    x_tile_rel,
                    width,
                    paint.clone(),
                );
            }
        }
    }
}

impl WideTile {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            bg: AlphaColor::TRANSPARENT,
            cmds: vec![],
        }
    }
}

#[derive(Debug)]
pub enum Cmd {
    Fill(CmdFill),
    Strip(CmdStrip),
}

#[derive(Debug)]
pub struct CmdFill {
    pub x: u32,
    pub width: u32,
    pub paint: Paint,
}

#[derive(Debug)]
pub struct CmdStrip {
    pub x: u32,
    pub width: u32,
    pub alpha_ix: usize,
    pub paint: Paint,
}

impl WideTile {
    pub(crate) fn fill(&mut self, x: u32, width: u32, paint: Paint) {
        let Paint::Solid(s) = &paint else { todo!() };
        let can_override = x == 0 && width == WIDE_TILE_WIDTH as u32 && s.components[3] == 1.0;

        if can_override {
            self.cmds.clear();
            self.bg = *s;
        } else {
            self.cmds.push(Cmd::Fill(CmdFill {
                x,
                width,
                paint,
            }));
        }
    }

    pub(crate) fn push(&mut self, cmd: Cmd) {
        self.cmds.push(cmd)
    }
}
