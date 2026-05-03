use oxc_allocator::{Box, Vec};
use oxc_span::Span;

use crate::ast::{
  bindings::Variable,
  nodes::{
    attribute::VAttribute,
    javascript::{VInterpolation, VPureScript},
  },
};

#[derive(Debug)]
pub enum VNode<'a> {
  Element(Box<'a, VElement<'a>>),
  Text(Box<'a, VText<'a>>),
  Comment(Box<'a, VComment<'a>>),
  Interpolation(Box<'a, VInterpolation<'a>>),
  PureScript(Box<'a, VPureScript<'a>>),
}

#[derive(Debug)]
pub struct VElement<'a> {
  pub name: &'a str,
  pub raw_name: &'a str,
  pub start_tag: VStartTag<'a>,
  pub children: Vec<'a, VNode<'a>>,
  pub end_tag: Option<VEndTag>,
  pub variables: Vec<'a, Variable<'a>>,
  pub span: Span,
}

#[derive(Debug)]
pub struct VStartTag<'a> {
  pub attributes: Vec<'a, VAttribute<'a>>,
  pub self_closing: bool,
  pub span: Span,
}

#[derive(Debug)]
pub struct VEndTag {
  pub span: Span,
}

#[derive(Debug)]
pub struct VText<'a> {
  pub text: &'a str,
  pub span: Span,
}

/// This won't be serialized, will just simply skip to follow vue-eslint-parser's behavior.
#[derive(Debug)]
pub struct VComment<'a> {
  pub value: &'a str,
  pub span: Span,
}
