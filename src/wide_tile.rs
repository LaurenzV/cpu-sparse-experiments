// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use peniko::color::{AlphaColor, Srgb};

pub const WIDE_TILE_WIDTH: usize = 256;
pub const STRIP_HEIGHT: usize = 4;

#[derive(Debug)]
pub struct WideTile {
    pub bg: AlphaColor<Srgb>,
    pub cmds: Vec<Cmd>,
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
    pub color: AlphaColor<Srgb>,
}

#[derive(Debug)]
pub struct CmdStrip {
    pub x: u32,
    pub width: u32,
    pub alpha_ix: usize,
    pub color: AlphaColor<Srgb>,
}

impl Default for WideTile {
    fn default() -> Self {
        Self {
            bg: AlphaColor::TRANSPARENT,
            cmds: vec![],
        }
    }
}

impl WideTile {
    pub(crate) fn fill(&mut self, x: u32, width: u32, color: AlphaColor<Srgb>) {
        if x == 0 && width == WIDE_TILE_WIDTH as u32 && color.components[3] == 1.0 {
            self.cmds.clear();
            self.bg = color;
        } else {
            self.cmds.push(Cmd::Fill(CmdFill { x, width, color }));
        }
    }

    pub(crate) fn push(&mut self, cmd: Cmd) {
        self.cmds.push(cmd)
    }
}
