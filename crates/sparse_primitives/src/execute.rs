use crate::{fine, strip};
use vello_common::execute::{Avx2, Scalar};

pub trait KernelExecutor: fine::Compose + strip::Render {}

impl KernelExecutor for Scalar {}

#[cfg(all(target_arch = "x86_64", feature = "simd"))]
impl KernelExecutor for Avx2 {}

#[cfg(all(target_arch = "aarch64", feature = "simd"))]
impl KernelExecutor for Neon {}
