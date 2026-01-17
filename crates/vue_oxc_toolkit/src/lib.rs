use oxc_allocator::{Allocator, Dummy};
use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::ParseOptions;
use oxc_span::Span;
use oxc_syntax::module_record::ModuleRecord;

use crate::parser::{ParserImpl, ParserImplReturn};

mod parser;

#[cfg(test)]
mod test;

pub struct VueOxcParser<'a> {
  allocator: &'a Allocator,
  source_text: &'a str,
  options: ParseOptions,
}

/// The return value of [`VueOxcParser::parse`]
/// Has the same struct as [`oxc_parser::ParserReturn`]
/// A workaround to pass #[`non_exhaustive`] of [`oxc_parser::ParserReturn`]
///
/// Do not provide `is_flow_language` field, it should be always false, as vue do not support it
#[non_exhaustive]
pub struct VueParserReturn<'a> {
  pub program: Program<'a>,
  pub module_record: ModuleRecord<'a>,
  pub errors: Vec<OxcDiagnostic>,
  pub irregular_whitespaces: Box<[Span]>,
  pub panicked: bool,
}

impl<'a> VueOxcParser<'a> {
  pub fn new(allocator: &'a Allocator, source_text: &'a str) -> Self {
    Self {
      allocator,
      source_text,
      options: ParseOptions::default(),
    }
  }

  #[must_use]
  pub const fn with_options(mut self, options: ParseOptions) -> Self {
    self.options = options;
    self
  }
}

impl<'a> VueOxcParser<'a> {
  #[must_use]
  pub fn parse(self) -> VueParserReturn<'a> {
    let ParserImplReturn {
      program,
      errors,
      fatal,
    } = ParserImpl::new(self.allocator, self.source_text, self.options).parse();

    if fatal {
      return self.fatal(errors);
    }

    todo!()
  }

  fn fatal(&self, errors: Vec<OxcDiagnostic>) -> VueParserReturn<'a> {
    VueParserReturn {
      program: Program::dummy(self.allocator),
      module_record: ModuleRecord::new(self.allocator),
      errors,
      irregular_whitespaces: Box::new([]),
      panicked: true,
    }
  }
}
