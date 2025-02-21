use crate::fine::{ScratchBuf, COLOR_COMPONENTS};
use crate::paint::Paint;
use crate::strip::Strip;
use crate::tiling::Tiles;
use crate::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};
use crate::{fine, strip, FillRule};
use peniko::Compose;

#[derive(Copy, Clone, Debug)]
/// The execution mode used for the rendering process.
pub enum ExecutionMode {
    /// Only use scalar execution. This is recommended if you want to have
    /// consistent results across different platforms and want to avoid unsafe code,
    /// and is the only option if you disabled the `simd` feature. Performance will be
    /// worse, though.
    Scalar,
    /// Select the best execution mode according to what is available on the host system.
    /// This is the recommended option for highest performance.
    #[cfg(feature = "simd")]
    Auto,
    /// Force the usage of neon SIMD instructions. This will lead to panics in case
    /// the CPU doesn't support neon.
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    Neon,
    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    Avx2,
}

#[cfg(feature = "simd")]
impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[cfg(not(feature = "simd"))]
impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Scalar
    }
}

pub(crate) trait KernelExecutor {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    );

    fn fill_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        compose: Compose,
    );

    fn strip_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
        compose: Compose,
    );
}

pub(crate) struct Scalar;

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub(crate) struct Neon;

impl KernelExecutor for Scalar {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        strip::scalar::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
    }

    fn fill_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        compose: Compose,
    ) {
        fine::scalar::fill_solid(scratch, color, x, width, compose);
    }

    fn strip_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
        compose: Compose,
    ) {
        fine::scalar::strip_solid(scratch, color, x, width, alphas, compose);
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub(crate) struct Avx2;

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl KernelExecutor for Avx2 {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        strip::scalar::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
    }

    fn fill_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        _: Compose,
    ) {
        // SAFETY: We are guaranteed to be running on a CPU that supports `avx2`.
        unsafe {
            fine::avx2::fill_solid(scratch, color, x, width);
        }
    }

    fn strip_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
        compose: Compose,
    ) {
        fine::scalar::strip_solid(scratch, color, x, width, alphas, compose);
    }
}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
impl KernelExecutor for Neon {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        // SAFETY: We are guaranteed to be running on a CPU that supports `neon`.
        unsafe {
            strip::neon::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
        }
    }

    fn fill_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        compose: Compose,
    ) {
        // SAFETY: We are guaranteed to be running on a CPU that supports `neon`.
        unsafe {
            fine::neon::fill_solid(scratch, color, x, width);
        }
    }

    fn strip_solid(
        scratch: &mut ScratchBuf,
        color: &[u8; COLOR_COMPONENTS],
        x: usize,
        width: usize,
        alphas: &[u32],
        compose: Compose,
    ) {
        // SAFETY: We are guaranteed to be running on a CPU that supports `neon`.
        unsafe {
            fine::neon::strip_solid(scratch, color, x, width, alphas);
        }
    }
}
