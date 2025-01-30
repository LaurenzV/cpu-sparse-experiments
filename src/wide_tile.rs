// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::paint::Paint;
use peniko::color::{AlphaColor, Srgb};

pub const WIDE_TILE_WIDTH: usize = 256;
pub const STRIP_HEIGHT: usize = 4;

#[derive(Debug)]
pub struct WideTile {
    pub x: usize,
    pub y: usize,
    pub bg: AlphaColor<Srgb>,
    pub cmds: Vec<Cmd>,
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
        if let Paint::Solid(s) = &paint {
            if x == 0 && width == WIDE_TILE_WIDTH as u32 && s.components[3] == 1.0 {
                self.cmds.clear();
                self.bg = *s;
            } else {
                self.cmds.push(Cmd::Fill(CmdFill { x, width, paint }));
            }
        } else {
            self.cmds.push(Cmd::Fill(CmdFill { x, width, paint }));
        }
    }

    pub(crate) fn push(&mut self, cmd: Cmd) {
        self.cmds.push(cmd)
    }
}
