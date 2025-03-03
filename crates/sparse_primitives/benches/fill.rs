use bench_gen::ColorIter;
use criterion::Criterion;
use peniko::Compose;
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
use sparse_primitives::execute::Avx2;
#[cfg(all(target_arch = "aarch64", feature = "simd"))]
use sparse_primitives::execute::Neon;
use sparse_primitives::execute::Scalar;
use sparse_primitives::fine::Fine;
use sparse_primitives::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};

const FILL_ITERS: usize = 1000;

pub fn fill(c: &mut Criterion) {
    let mut g = c.benchmark_group("fill");

    macro_rules! fill_single {
        ($name:ident, $compose:path, $exec:ident) => {
            g.bench_function(
                format!("{} - {}", stringify!($name), stringify!($exec)),
                |b| {
                    b.iter(|| {
                        let mut out = vec![];
                        let mut fine = Fine::<$exec>::new(WIDE_TILE_WIDTH, STRIP_HEIGHT, &mut out);
                        let mut color = ColorIter::new(false);

                        for _ in 0..FILL_ITERS {
                            fine.fill(0, 254, &color.next().unwrap().into(), $compose);
                        }
                    })
                },
            );
        };
    }

    macro_rules! fill {
        ($name:ident, $compose:path) => {
            fill_single!($name, $compose, Scalar);
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            fill_single!($name, $compose, Neon);
            #[cfg(all(target_arch = "x86_64", feature = "simd"))]
            fill_single!($name, $compose, Avx2);
        };
    }

    fill!(src_over, Compose::SrcOver);
}
