use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use oxc_allocator::Allocator;
use vue_oxc_toolkit::VueOxcParser;

fn bench(c: &mut Criterion) {
  let mut group = c.benchmark_group("vue_parse_by_size");

  let samples = [
    ("small", include_str!("../small.vue")),
    ("medium", include_str!("../medium.vue")),
    ("large", include_str!("../large.vue")),
  ];

  for (name, html) in samples {
    group.bench_function(name, |b| {
      b.iter(|| {
        let allocator = Allocator::new();
        black_box(VueOxcParser::new(&allocator, html).parse());
      });
    });
  }

  group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
