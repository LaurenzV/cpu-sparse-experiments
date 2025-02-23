mod fill;
mod sorting;
mod tiling;

use criterion::{criterion_group, criterion_main};

criterion_group!(tg, tiling::tiling);
criterion_group!(s, sorting::sorting);
criterion_group!(f, fill::fill);
criterion_main!(tg, s, f);
