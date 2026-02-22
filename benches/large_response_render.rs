use criterion::{criterion_group, criterion_main, Criterion};

fn bench_large_response_render(_c: &mut Criterion) {}

criterion_group!(benches, bench_large_response_render);
criterion_main!(benches);
