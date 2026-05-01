use oxc_allocator::GetAddress;
use oxc_ast::{AstKind, ast::Program};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, SPAN, Span};

use crate::codegen::DirtySet;

pub fn collect_dirty_nodes(program: &Program<'_>, clean_ranges: &[Span]) -> DirtySet {
  let mut collector = DirtyNodeCollector { clean_ranges, dirty_nodes: DirtySet::default() };
  collector.visit_program(program);
  collector.dirty_nodes
}

struct DirtyNodeCollector<'a> {
  clean_ranges: &'a [Span],
  dirty_nodes: DirtySet,
}

impl DirtyNodeCollector<'_> {
  fn is_dirty(&self, span: Span) -> bool {
    span == SPAN
      || !self.clean_ranges.iter().any(|range| range.start <= span.start && span.end <= range.end)
  }
}

impl<'a> Visit<'a> for DirtyNodeCollector<'_> {
  fn enter_node(&mut self, kind: AstKind<'a>) {
    if self.is_dirty(kind.span()) {
      self.dirty_nodes.insert(kind.address());
    }
  }
}
