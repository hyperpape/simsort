#[path = "../src/minhash.rs"]
mod minhash;
#[path = "../src/tsp.rs"]
mod tsp;
#[path = "../src/utils.rs"]
mod utils;

use std::path::Path;

use crate::minhash::minhash;
use crate::tsp::Tsp;

use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn build_distances(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_build_distances");
    for size in [4096, 8196, 16384, 32768].iter() {
        group.throughput(Throughput::Elements((size * size) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| utils::build_linear_distances(size));
        });
    }
}

fn bench_minhash(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench_minhash");

    for count in [1024, 4096, 16384, 65536, 262144, 1048576, 4194304].iter() {
        let file_path = Path::new("./testdata/").join(format!("rand{}", count));

        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| minhash::minhash(&file_path));
        });
    }
}

criterion_group!(benches, build_distances, bench_minhash);
criterion_main!(benches);
