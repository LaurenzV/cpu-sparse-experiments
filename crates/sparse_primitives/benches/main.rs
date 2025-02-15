mod tiling;

use criterion::{criterion_group, criterion_main};

criterion_group!(tg, tiling::tiling);
criterion_main!(tg);
