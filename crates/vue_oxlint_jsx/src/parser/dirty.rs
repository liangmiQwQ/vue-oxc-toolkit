use oxc_allocator::GetAddress;
use oxc_ast::{AstKind, ast::Program};
use oxc_ast_visit::Visit;
use oxc_span::GetSpan;

use crate::codegen::DirtySet;

pub fn collect_dirty_nodes(program: &Program<'_>, dirty_nodes: DirtySet) -> DirtySet {
  let mut collector = DirtyNodeCollector { dirty_nodes };
  collector.visit_program(program);
  collector.dirty_nodes
}

struct DirtyNodeCollector {
  dirty_nodes: DirtySet,
}

impl<'a> Visit<'a> for DirtyNodeCollector {
  fn enter_node(&mut self, kind: AstKind<'a>) {
    if self.dirty_nodes.contains_span(kind.span()) {
      self.dirty_nodes.insert_node(kind.address());
    }
  }
}
