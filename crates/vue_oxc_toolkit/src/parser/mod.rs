use std::cell::RefCell;

use oxc_allocator::Allocator;
use oxc_ast::{AstBuilder, Comment, ast::Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::SourceType;

mod parse;
mod utils;

#[cfg(test)]
mod test_safety;

pub struct ParserImpl<'a> {
  allocator: &'a Allocator,
  source_type: SourceType,
  source_text: &'a str,
  ast: AstBuilder<'a>,
  comments: RefCell<oxc_allocator::Vec<'a, Comment>>,
  empty_str: String,
  options: ParseOptions,
}

impl<'a> ParserImpl<'a> {
  /// Create a [`ParserImpl`]
  pub fn new(allocator: &'a Allocator, source_text: &'a str, options: ParseOptions) -> Self {
    let ast = AstBuilder::new(allocator);
    Self {
      allocator,
      source_type: SourceType::jsx(),
      source_text,
      ast,
      comments: RefCell::from(ast.vec()),
      empty_str: ".".repeat(source_text.len()),
      options,
    }
  }
}

pub struct ParserImplReturn<'a> {
  pub program: Program<'a>,
  pub fatal: bool,
  pub errors: Vec<OxcDiagnostic>,
}
