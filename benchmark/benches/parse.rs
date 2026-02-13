use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use oxc_allocator::Allocator;
use vue_oxc_toolkit::VueOxcParser;

fn bench(c: &mut Criterion) {
  const VUE_SOURCE: &str = include_str!("../vue.vue");

  c.bench_function("parse_vue", |b| {
    b.iter(|| {
      let allocator = Allocator::new();
      black_box(VueOxcParser::new(&allocator, VUE_SOURCE).parse());
    });
  });
}

criterion_group!(benches, bench);
criterion_main!(benches);
