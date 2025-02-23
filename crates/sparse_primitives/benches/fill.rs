use bench_gen::ColorIter;
use criterion::Criterion;
use peniko::Compose;
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
        };
    }

    fill!(clear, Compose::Clear);
    fill!(copy, Compose::Copy);
    fill!(dest, Compose::Dest);
    fill!(src_over, Compose::SrcOver);
    fill!(dest_over, Compose::DestOver);
    fill!(src_in, Compose::SrcIn);
    fill!(dest_in, Compose::DestIn);
    fill!(src_out, Compose::SrcOut);
    fill!(dest_out, Compose::DestOut);
    fill!(src_atop, Compose::SrcAtop);
    fill!(dest_atop, Compose::DestAtop);
    fill!(xor, Compose::Xor);
    fill!(plus, Compose::Plus);
}
