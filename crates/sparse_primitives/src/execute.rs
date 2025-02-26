use crate::strip::Strip;
use crate::tiling::Tiles;
use crate::{fine, strip, FillRule};

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
    /// the CPU doesn't support the target feature `neon`.
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    Neon,
    /// Force the usage of AVX2 SIMD instructions. This will lead to panics in case
    /// the CPU doesn't support the target features `avx2` and `fma`.
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

pub trait KernelExecutor: fine::Compose {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    );
}

pub struct Scalar;

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
pub struct Neon;

impl KernelExecutor for Scalar {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        strip::scalar::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
    }
}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
pub struct Avx2;

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl KernelExecutor for Avx2 {
    fn render_strips(
        tiles: &Tiles,
        strip_buf: &mut Vec<Strip>,
        alpha_buf: &mut Vec<u32>,
        fill_rule: FillRule,
    ) {
        unsafe {
            strip::avx2::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
        }
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
        unsafe {
            strip::neon::render_strips(tiles, strip_buf, alpha_buf, fill_rule);
        }
    }
}
