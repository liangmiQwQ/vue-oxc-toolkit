use oxc_allocator::Allocator;
use oxc_ast::{AstBuilder, Comment, ast::Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::SourceType;
use oxc_syntax::module_record::ModuleRecord;

mod directive;
mod error;
mod modules;
mod parse;

pub struct ParserImpl<'a> {
  allocator: &'a Allocator,
  source_text: &'a str,
  options: ParseOptions,
  empty_str: String,
  ast: AstBuilder<'a>,

  module_record: ModuleRecord<'a>,
  source_type: SourceType,
  comments: oxc_allocator::Vec<'a, Comment>,
  errors: Vec<OxcDiagnostic>,
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
      comments: ast.vec(),

      module_record: ModuleRecord::new(allocator),
      errors: vec![],
      empty_str: ".".repeat(source_text.len()),
      options,
    }
  }
}

pub struct ParserImplReturn<'a> {
  pub program: Program<'a>,
  pub module_record: ModuleRecord<'a>,

  pub fatal: bool,
  pub errors: Vec<OxcDiagnostic>,
}
