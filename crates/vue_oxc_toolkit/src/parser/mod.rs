use std::cell::RefCell;

use oxc_allocator::Allocator;
use oxc_ast::{AstBuilder, Comment, ast::Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::SourceType;

mod implement;
mod utils;

pub struct ParserImpl<'a> {
  pub allocator: &'a Allocator,
  pub source_type: SourceType,
  pub source_text: &'a str,
  pub ast: AstBuilder<'a>,
  comments: RefCell<oxc_allocator::Vec<'a, Comment>>,
  empty_str: String,
}

pub struct ParserImplReturn<'a> {
  pub program: Program<'a>,
  pub fatal: bool,
  pub errors: Vec<OxcDiagnostic>,
}
