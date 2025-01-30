// Copyright 2024 the Piet Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A simple pixmap type

use zune_png::zune_core::options::DecoderOptions;

#[derive(Debug)]
pub struct Pixmap {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) buf: Vec<u8>,
}

impl Pixmap {
    pub fn new(width: usize, height: usize) -> Self {
        let buf = vec![0; width * height * 4];
        Self { width, height, buf }
    }

    pub fn from_png(data: &[u8]) -> Option<Pixmap> {
        let options = DecoderOptions::new_cmd().png_set_add_alpha_channel(true);
        let mut decoder = zune_png::PngDecoder::new_with_options(data, options);
        decoder.decode_headers().ok()?;

        let dimensions = decoder.get_dimensions()?;

        // TODO: Check correct colorspace
        let mut decoded = decoder.decode().ok()?.u8()?;

        for pixel in decoded.chunks_exact_mut(4) {
            pixel[0] = pixel[0] * pixel[3];
            pixel[1] = pixel[1] * pixel[3];
            pixel[2] = pixel[2] * pixel[3];
        }

        Some(Self {
            width: dimensions.0,
            height: dimensions.1,
            buf: decoded,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    /// Convert from premultiplied to separate alpha.
    ///
    /// Not fast, but useful for saving to PNG etc.
    pub fn unpremultiply(&mut self) {
        for rgba in self.buf.chunks_exact_mut(4) {
            let alpha = rgba[3] as f32 * (1.0 / 255.0);
            if alpha != 0.0 {
                rgba[0] = (rgba[0] as f32 / alpha).round().min(255.0) as u8;
                rgba[1] = (rgba[1] as f32 / alpha).round().min(255.0) as u8;
                rgba[2] = (rgba[2] as f32 / alpha).round().min(255.0) as u8;
            }
        }
    }
}
