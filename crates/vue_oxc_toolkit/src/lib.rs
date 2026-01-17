use oxc_allocator::Allocator;
use oxc_parser::{ParseOptions, ParserReturn};

use crate::parser::ParserImpl;

mod parser;

#[cfg(test)]
mod test;

pub struct VueOxcParser<'a> {
  allocator: &'a Allocator,
  source_text: &'a str,
  options: ParseOptions,
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
  pub fn parse(&self) -> ParserReturn<'a> {
    let _ret = ParserImpl::new(self.allocator, self.source_text, self.options).parse();

    todo!()
  }
}
