//! All the directives defined there will be serialized into VAttribute struct with `{ directive: true }`

use crate::ast::nodes::{
  attribute::VIdentifier,
  javascript::{
    VDirectiveArgumentExpression, VDirectiveExpression, VForExpression, VOnExpression,
    VSlotExpression,
  },
};
use oxc_allocator::{Box, Vec};
use oxc_span::Span;

/// For normal directives, like `v-bind`, `v-model`, `v-if`.
#[derive(Debug)]
pub struct VDirective<'a> {
  pub key: VDirectiveKey<'a>,
  pub value: VDirectiveExpression<'a>,
  pub modifiers: Vec<'a, VIdentifier<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VOnDirective<'a> {
  pub key: VDirectiveKey<'a>,
  pub value: VOnExpression<'a>,
  pub modifiers: Vec<'a, VIdentifier<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VSlotDirective<'a> {
  pub key: VDirectiveKey<'a>,
  pub value: VSlotExpression<'a>,
  pub modifiers: Vec<'a, VIdentifier<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VForDirective<'a> {
  pub key: VDirectiveKey<'a>,
  pub value: VForExpression<'a>,
  pub modifiers: Vec<'a, VIdentifier<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VDirectiveKey<'a> {
  pub name: &'a VIdentifier<'a>,
  pub argument: VDirectiveArgument<'a>,
  pub span: Span,
}

#[derive(Debug)]
pub enum VDirectiveArgument<'a> {
  VDirectiveArgument(Box<'a, VDirectiveArgumentExpression<'a>>),
  VIdentifier(Box<'a, VIdentifier<'a>>),
}
