//! Thin wrapper around the local `oxc_codegen` fork.
//!
//! The fork stays close to upstream and adds only one toolkit-specific hook:
//! a callback that reports generated ranges for each printed AST node.

use std::{cell::RefCell, rc::Rc};

use oxc_ast::ast::Program;
use oxc_codegen::{Codegen as OxcCodegen, CodegenOptions};

pub use oxc_codegen::CodegenHook;

pub struct Codegen<'a, H: CodegenHook + 'a> {
  hook: H,
  _source: &'a str,
}

impl<'a, H: CodegenHook + 'a> Codegen<'a, H> {
  pub const fn new(source: &'a str, hook: H) -> Self {
    Self { hook, _source: source }
  }

  pub fn build(self, program: &Program<'a>) -> (String, H) {
    let hook = Rc::new(RefCell::new(self.hook));
    let codegen_hook = Rc::clone(&hook);
    let options = CodegenOptions { single_quote: true, ..CodegenOptions::default() };

    let result = OxcCodegen::new()
      .with_options(options)
      .with_codegen_hook(move |span, virtual_start, virtual_end| {
        codegen_hook.borrow_mut().record(span, virtual_start, virtual_end);
      })
      .build(program);

    let hook = Rc::try_unwrap(hook)
      .unwrap_or_else(|_| unreachable!("codegen hook should be released after build"))
      .into_inner();

    (result.code, hook)
  }
}
