use bench_gen::ColorIter;
use criterion::Criterion;
use peniko::Compose;
use rand::rngs::StdRng;
use rand::RngCore;
use rand::SeedableRng;
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
use sparse_primitives::execute::Avx2;
#[cfg(all(target_arch = "aarch64", feature = "simd"))]
use sparse_primitives::execute::Neon;
use sparse_primitives::execute::Scalar;
use sparse_primitives::fine::Fine;
use sparse_primitives::wide_tile::{STRIP_HEIGHT, WIDE_TILE_WIDTH};

const STRIP_ITERS: usize = 400;
const SEED: [u8; 32] = [0; 32];

pub fn strip(c: &mut Criterion) {
    let mut g = c.benchmark_group("strip");

    macro_rules! strip_single {
        ($name:ident, $compose:path, $exec:ident) => {
            let mut alphas = vec![];
            let mut rng = StdRng::from_seed(SEED);

            for _ in 0..STRIP_ITERS {
                alphas.push(rng.next_u32());
            }

            g.bench_function(
                format!("{} - {}", stringify!($name), stringify!($exec)),
                |b| {
                    b.iter(|| {
                        let mut out = vec![];
                        let mut fine = Fine::<$exec>::new(WIDE_TILE_WIDTH, STRIP_HEIGHT, &mut out);
                        let mut color = ColorIter::new(false);

                        for _ in 0..STRIP_ITERS {
                            fine.strip(
                                0,
                                WIDE_TILE_WIDTH,
                                &alphas,
                                &color.next().unwrap().into(),
                                $compose,
                            );
                        }
                    })
                },
            );
        };
    }

    macro_rules! strip {
        ($name:ident, $compose:path) => {
            strip_single!($name, $compose, Scalar);
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            strip_single!($name, $compose, Neon);
            #[cfg(all(target_arch = "x86_64", feature = "simd"))]
            strip_single!($name, $compose, Avx2);
        };
    }

    strip!(src_over, Compose::SrcOver);
}
