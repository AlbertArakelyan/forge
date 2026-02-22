use criterion::{criterion_group, criterion_main, Criterion};

fn bench_startup(_c: &mut Criterion) {}

criterion_group!(benches, bench_startup);
criterion_main!(benches);
