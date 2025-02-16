//! Tile generation

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, SamplingMode};
use peniko::kurbo::{Affine, BezPath, Stroke};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use sparse_primitives::flatten;
use sparse_primitives::tiling::{FlatLine, Point, Tiles};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const SEED: [u8; 32] = [0; 32];

pub enum IteratorType {
    SingleTile,
    VerticalColumn,
    HorizontalColumn,
    GeneralCase,
}

pub struct LineIterator {
    rng: StdRng,
    iterator_type: IteratorType,
}

impl LineIterator {
    pub fn new(iterator_type: IteratorType) -> Self {
        LineIterator {
            rng: StdRng::from_seed(SEED),
            iterator_type,
        }
    }

    fn gen_points(&mut self, x_min: f32, y_min: f32, x_max: f32, y_max: f32) -> (Point, Point) {
        let x0: f32 = self.rng.gen_range(x_min..x_max);
        let x1: f32 = self.rng.gen_range(x_min..x_max);
        let y0: f32 = self.rng.gen_range(y_min..y_max);
        let y1: f32 = self.rng.gen_range(y_min..y_max);

        let p0 = Point::new(x0, y0);
        let p1 = Point::new(x1, y1);

        (p0, p1)
    }
}

impl Iterator for LineIterator {
    type Item = FlatLine;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator_type {
            IteratorType::SingleTile => {
                let (p0, p1) = self.gen_points(0.0, 0.0, 4.0, 4.0);
                Some(FlatLine::new(p0, p1))
            }
            IteratorType::HorizontalColumn => {
                let (p0, p1) = self.gen_points(0.0, 0.0, 100.0, 4.0);
                Some(FlatLine::new(p0, p1))
            }
            IteratorType::VerticalColumn => {
                let (p0, p1) = self.gen_points(0.0, 0.0, 4.0, 100.0);
                Some(FlatLine::new(p0, p1))
            }
            IteratorType::GeneralCase => {
                let (p0, p1) = self.gen_points(0.0, 0.0, 100.0, 100.0);
                Some(FlatLine::new(p0, p1))
            }
        }
    }
}

fn read_from_file(name: &str) -> Vec<Vec<FlatLine>> {
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
        flatten::fill(&path, Affine::IDENTITY, &mut temp_buf);
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
        flatten::stroke(&path, &Stroke::new(3.0), Affine::IDENTITY, &mut temp_buf);
        buf.push(temp_buf);
    }

    buf
}

pub fn tiling(c: &mut Criterion) {
    let mut unit_group = c.benchmark_group("tiling_unit");
    // single_tile(&mut unit_group);
    // horizontal_column(&mut unit_group);
    // vertical_column(&mut unit_group);
    // general_case(&mut unit_group);
    unit_group.finish();

    let mut integration_group = c.benchmark_group("tiling_integration");
    integration_group.sample_size(30);
    ghostscript_tiger(&mut integration_group);
    coat_of_arms(&mut integration_group);
    integration_group.finish();
}

fn single_tile(g: &mut BenchmarkGroup<WallTime>) {
    let lines = LineIterator::new(IteratorType::SingleTile)
        .take(6000)
        .collect::<Vec<_>>();

    g.bench_function("single tile", |b| {
        b.iter(|| {
            let mut tiles = Tiles::new();
            tiles.make_tiles(&lines);
        })
    });
}

fn horizontal_column(g: &mut BenchmarkGroup<WallTime>) {
    let lines = LineIterator::new(IteratorType::HorizontalColumn)
        .take(6000)
        .collect::<Vec<_>>();

    g.bench_function("horizontal column", |b| {
        b.iter(|| {
            let mut tiles = Tiles::new();
            tiles.make_tiles(&lines);
        })
    });
}

fn vertical_column(g: &mut BenchmarkGroup<WallTime>) {
    let lines = LineIterator::new(IteratorType::VerticalColumn)
        .take(6000)
        .collect::<Vec<_>>();

    g.bench_function("vertical column", |b| {
        b.iter(|| {
            let mut tiles = Tiles::new();
            tiles.make_tiles(&lines);
        })
    });
}

fn general_case(g: &mut BenchmarkGroup<WallTime>) {
    let lines = LineIterator::new(IteratorType::GeneralCase)
        .take(6000)
        .collect::<Vec<_>>();

    g.bench_function("general case", |b| {
        b.iter(|| {
            let mut tiles = Tiles::new();
            tiles.make_tiles(&lines);
        })
    });
}

fn ghostscript_tiger(g: &mut BenchmarkGroup<WallTime>) {
    let lines = read_from_file("gs_tiger");

    g.bench_function("ghostscript tiger", |b| {
        b.iter(|| {
            let mut tiling = Tiles::new();

            for buf in &lines {
                tiling.make_tiles(buf);
            }
        })
    });
}

fn coat_of_arms(g: &mut BenchmarkGroup<WallTime>) {
    let lines = read_from_file("coat_of_arms");

    g.bench_function("coat of arms", |b| {
        b.iter(|| {
            let mut tiling = Tiles::new();

            for buf in &lines {
                tiling.make_tiles(buf);
            }
        })
    });
}
