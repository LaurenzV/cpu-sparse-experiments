use bench_gen::ColorIter;
use criterion::Criterion;
use peniko::color::palette::css::LIME_GREEN;
use peniko::Compose;
use sparse_primitives::execute::{Neon, Scalar};
use sparse_primitives::fine::Fine;
use sparse_primitives::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};

const FILL_ITERS: usize = 1000;

pub fn filling(c: &mut Criterion) {
    let mut g = c.benchmark_group("filling");

    g.bench_function("scalar", |b| {
        b.iter(|| {
            let mut out = vec![];
            let mut fine = Fine::<Scalar>::new(WIDE_TILE_WIDTH, STRIP_HEIGHT, &mut out);
            let mut color = ColorIter::new(false);

            for _ in 0..FILL_ITERS {
                fine.fill(0, 254, &color.next().unwrap().into(), Compose::SrcOver);
            }
        })
    });

    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    {
        g.bench_function("neon", |b| {
            b.iter(|| {
                let mut out = vec![];
                let mut fine = Fine::<Neon>::new(WIDE_TILE_WIDTH, STRIP_HEIGHT, &mut out);
                let mut color = ColorIter::new(false);

                for _ in 0..FILL_ITERS {
                    fine.fill(
                        0,
                        WIDE_TILE_WIDTH,
                        &color.next().unwrap().into(),
                        Compose::SrcOver,
                    );
                }
            })
        });
    }
}
