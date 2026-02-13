use criterion::{Criterion, criterion_group, criterion_main};

fn bench(c: &mut Criterion) {
  const VUE_SOURCE: &str = include_str!("../vue.vue");

  c.bench_function("parse_vue", |b| {
    b.iter(|| {
      let x = 1;
      let y = 2;
      x + y
    });
  });
}

criterion_group!(benches, bench);
criterion_main!(benches);
