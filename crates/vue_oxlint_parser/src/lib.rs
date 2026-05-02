//! `vue_oxlint_parser` — first-party Vue SFC parser.
//!
//! Parses a Vue Single-File Component and produces a [`VueSingleFileComponent`] AST.

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::module_name_repetitions)]

pub mod ast;
pub mod irregular_whitespaces;
pub mod parser;

#[cfg(test)]
pub mod test;

pub use ast::*;

use oxc_allocator::Allocator;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Token;
use oxc_span::Span;
use oxc_syntax::module_record::ModuleRecord;
use rustc_hash::FxHashSet;

/// Public return type from [`parse_sfc`].
pub struct VueSfcParserReturn<'a> {
  pub sfc: VueSingleFileComponent<'a>,
  pub errors: Vec<OxcDiagnostic>,
  pub panicked: bool,
  pub clean_spans: FxHashSet<Span>,
  pub irregular_whitespaces: Box<[Span]>,
  pub module_record: ModuleRecord<'a>,
  pub script_tokens: Vec<Token>,
}

/// Parse a Vue SFC source string and return the AST.
#[must_use]
pub fn parse_sfc<'a>(allocator: &'a Allocator, source_text: &'a str) -> VueSfcParserReturn<'a> {
  parser::parse_impl(allocator, source_text)
}

#[cfg(test)]
mod tests {
  use crate::test::test_sfc;

  test_sfc!(basic_vue, "basic.vue");
  test_sfc!(script_setup, "script_setup.vue");
  test_sfc!(typescript, "typescript.vue");
  test_sfc!(directives, "directives.vue");
  test_sfc!(interpolation, "interpolation.vue");
}
