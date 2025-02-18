use criterion::measurement::WallTime;
use criterion::{BatchSize, BenchmarkGroup, Criterion};
use peniko::color::palette::css::LIME_GREEN;
use peniko::color::AlphaColor;
use sparse_primitives::fine::Fine;
use sparse_primitives::tiling::Tiles;
use sparse_primitives::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};
use sparse_primitives::ExecutionMode;

pub fn filling(c: &mut Criterion) {
    let mut g = c.benchmark_group("filling");

    g.bench_function("scalar", |b| {
        b.iter(|| {
            let mut out = vec![];
            let mut fine = Fine::new(
                WIDE_TILE_WIDTH,
                STRIP_HEIGHT,
                &mut out,
                ExecutionMode::Scalar,
            );

            for i in 0..1000 {
                fine.fill(0, WIDE_TILE_WIDTH, &LIME_GREEN.into());
            }
        })
    });

    #[cfg(feature = "simd")]
    {
        g.bench_function("simd", |b| {
            b.iter(|| {
                let mut out = vec![];
                let mut fine =
                    Fine::new(WIDE_TILE_WIDTH, STRIP_HEIGHT, &mut out, ExecutionMode::Auto);

                for i in 0..1000 {
                    fine.fill(0, WIDE_TILE_WIDTH, &LIME_GREEN.with_alpha(0.5).into());
                }
            })
        });
    }
}
