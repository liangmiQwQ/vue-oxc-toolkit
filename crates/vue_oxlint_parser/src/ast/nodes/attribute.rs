use oxc_allocator::Box;
use oxc_span::Span;

use crate::ast::nodes::directive::{VDirective, VForDirective, VOnDirective, VSlotDirective};

#[derive(Debug)]
pub enum VAttribute<'a> {
  Normal(Box<'a, VPureAttribute<'a>>),
  Directive(Box<'a, VDirective<'a>>),
  OnDirective(Box<'a, VOnDirective<'a>>),
  SlotDirective(Box<'a, VSlotDirective<'a>>),
  ForDirective(Box<'a, VForDirective<'a>>),
}

#[derive(Debug)]
pub struct VPureAttribute<'a> {
  pub key: VIdentifier<'a>,
  pub value: Option<VLiteral<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VIdentifier<'a> {
  pub name: &'a str,
  pub raw_name: &'a str,
  pub span: Span,
}

#[derive(Debug)]
pub struct VLiteral<'a> {
  pub value: &'a str,
  pub span: Span,
}
