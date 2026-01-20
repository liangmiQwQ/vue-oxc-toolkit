use std::cell::RefCell;

use oxc_allocator::Allocator;
use oxc_ast::{AstBuilder, Comment, ast::Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::SourceType;

mod parse;
mod utils;

pub struct ParserImpl<'a> {
  allocator: &'a Allocator,
  source_text: &'a str,
  options: ParseOptions,
  empty_str: String,
  ast: AstBuilder<'a>,

  source_type: SourceType,
  comments: RefCell<oxc_allocator::Vec<'a, Comment>>,
  errors: RefCell<Vec<OxcDiagnostic>>,
}

impl<'a> ParserImpl<'a> {
  /// Create a [`ParserImpl`]
  pub fn new(allocator: &'a Allocator, source_text: &'a str, options: ParseOptions) -> Self {
    let ast = AstBuilder::new(allocator);
    Self {
      allocator,
      source_text,
      ast,
      source_type: SourceType::jsx(),
      comments: RefCell::from(ast.vec()),
      errors: RefCell::from(vec![]),
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
