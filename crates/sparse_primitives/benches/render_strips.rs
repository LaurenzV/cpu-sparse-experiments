use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use peniko::Style::Fill;
#[cfg(all(target_arch = "x86_64", feature = "simd"))]
use sparse_primitives::execute::Avx2;
#[cfg(all(target_arch = "aarch64", feature = "simd"))]
use sparse_primitives::execute::Neon;
use sparse_primitives::execute::Scalar;
use sparse_primitives::kurbo::{Affine, BezPath, Stroke};
use sparse_primitives::strip::render_strips;
use sparse_primitives::tiling::{FlatLine, Tile, Tiles};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub fn _render_strips(c: &mut Criterion) {
    let mut g = c.benchmark_group("render_strips");
    g.sample_size(20);

    macro_rules! single {
        ($name:ident, $func:ident, $exec:ident) => {
            $func(&mut g);

            fn $func(g: &mut BenchmarkGroup<WallTime>) {
                let tiles = read_from_file(stringify!($name));

                g.bench_function(
                    format!("{} - {}", stringify!($name), stringify!($exec)),
                    move |b| {
                        b.iter(|| {
                            let mut strip_buf = vec![];
                            let mut alpha_buf = vec![];

                            for tile in &tiles {
                                render_strips::<$exec>(
                                    tile,
                                    &mut strip_buf,
                                    &mut alpha_buf,
                                    Fill::NonZero,
                                );
                            }
                        })
                    },
                );
            }
        };
    }

    macro_rules! render_strips {
        ($name:ident) => {{
            single!($name, a, Scalar);
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            single!($name, b, Neon);
            #[cfg(all(target_arch = "x86_64", feature = "simd"))]
            single!($name, c, Avx2);
        }};
    }

    render_strips!(gs_tiger);
    render_strips!(coat_of_arms);
}

// TODO: Deduplicate
fn read_from_file(name: &str) -> Vec<Tiles> {
    let fills_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("benches/assets/{}_fills.txt", name));
    let strokes_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("benches/assets/{}_strokes.txt", name));

    let mut buf = vec![];

    let file = File::open(fills_path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let l = line.unwrap();
        let path = BezPath::from_svg(&l).unwrap();
        let mut line_buf = vec![];
        sparse_primitives::flatten::fill(&path, Affine::IDENTITY, &mut line_buf);
        let mut tiles = Tiles::new();
        tiles.make_tiles(&line_buf);
        tiles.sort_tiles();

        buf.push(tiles);
    }

    let file = File::open(strokes_path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let l = line.unwrap();
        let path = BezPath::from_svg(&l).unwrap();
        let mut line_buf = vec![];
        // Obviously not 100% accurate since the stroke width isn't always the same, but good
        // enough for benching purposes.
        sparse_primitives::flatten::stroke(
            &path,
            &Stroke::new(3.0),
            Affine::IDENTITY,
            &mut line_buf,
        );

        let mut tiles = Tiles::new();
        tiles.make_tiles(&line_buf);
        tiles.sort_tiles();

        buf.push(tiles);
    }

    buf
}
