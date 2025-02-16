use criterion::measurement::WallTime;
use criterion::{BatchSize, BenchmarkGroup, Criterion};
use peniko::kurbo::{Affine, BezPath, Stroke};
use sparse_primitives::tiling::{FlatLine, Tiler};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub(crate) fn flattened_from_file(name: &str) -> Vec<Vec<FlatLine>> {
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
        let mut temp_buf = vec![];
        sparse_primitives::flatten::fill(&path, Affine::IDENTITY, &mut temp_buf);
        buf.push(temp_buf);
    }

    let file = File::open(strokes_path).unwrap();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let l = line.unwrap();
        let path = BezPath::from_svg(&l).unwrap();
        let mut temp_buf = vec![];
        // Obviously not 100% accurate since the stroke width isn't always the same, but good
        // enough for benching purposes.
        sparse_primitives::flatten::stroke(
            &path,
            &Stroke::new(3.0),
            Affine::IDENTITY,
            &mut temp_buf,
        );
        buf.push(temp_buf);
    }

    buf
}

pub fn sorting(c: &mut Criterion) {
    let mut integration_group = c.benchmark_group("sorting_integration");
    integration_group.sample_size(30);
    ghostscript_tiger(&mut integration_group);
    coat_of_arms(&mut integration_group);
    integration_group.finish();
}

fn ghostscript_tiger(g: &mut BenchmarkGroup<WallTime>) {
    let mut tiles = flattened_from_file("gs_tiger")
        .iter()
        .map(|i| {
            let mut tiler = Tiler::new();
            tiler.make_tiles(i);

            tiler
        })
        .collect::<Vec<_>>();

    g.bench_with_input("ghostscript tiger", &mut tiles.clone(), |b, i| {
        b.iter_batched_ref(
            || i.clone(),
            |input| {
                for buf in input {
                    buf.sort_tiles();
                }
            }, BatchSize::SmallInput)
    });
}

fn coat_of_arms(g: &mut BenchmarkGroup<WallTime>) {
    let tiles = flattened_from_file("coat_of_arms")
        .iter()
        .map(|i| {
            let mut tiler = Tiler::new();
            tiler.make_tiles(i);

            tiler
        })
        .collect::<Vec<_>>();

    g.bench_with_input("coat of arms", &tiles, |b, i| {
        b.iter_batched_ref(
            || i.clone(),
            |input| {
                for buf in input {
                    buf.sort_tiles();
                }
            }, BatchSize::SmallInput)
    });
}