//! V* AST node types — the Vue-side AST produced by the template parser.
//!
//! Modeled after vue-eslint-parser's AST. Nodes are arena-allocated in the
//! "Vue allocator" (`oxc_allocator::Allocator`) so `Box<'a, _>`/`Vec<'a, _>`
//! borrow into a single bump arena. Strings are `&'a str` slices into the
//! original SFC source which the caller is required to keep alive for the
//! lifetime of the AST.
//!
//! All nodes derive `serde::Serialize` so the entire tree can be exported
//! to JSON for transfer across the napi boundary.

use oxc_allocator::{Box as ArenaBox, Vec as ArenaVec};
use serde::Serialize;

/// Byte-offset span (UTF-8 byte indices into the original SFC source).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Span {
  pub start: u32,
  pub end: u32,
}

impl Span {
  #[must_use]
  pub const fn new(start: u32, end: u32) -> Self {
    Self { start, end }
  }
}

/// Top-level result of parsing a `.vue` SFC.
///
/// Note: `script_program` and other `oxc_ast` contents are *not* included in
/// this struct — they live in the JS allocator and are serialized separately.
/// See `parser::ParsedSfc`.
#[derive(Debug, Serialize)]
pub struct VDocumentFragment<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub children: ArenaVec<'a, VRootChild<'a>>,
}

impl<'a> VDocumentFragment<'a> {
  #[must_use]
  pub const fn new(range: Span, children: ArenaVec<'a, VRootChild<'a>>) -> Self {
    Self { r#type: "VDocumentFragment", range, children }
  }
}

/// Children of `VDocumentFragment` — top-level SFC blocks plus surrounding
/// whitespace/text nodes.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VRootChild<'a> {
  Element(ArenaBox<'a, VElement<'a>>),
  Text(ArenaBox<'a, VText<'a>>),
}

#[derive(Debug, Serialize)]
pub struct VElement<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub name: &'a str,
  pub raw_name: &'a str,
  pub namespace: VNamespace,
  pub start_tag: ArenaBox<'a, VStartTag<'a>>,
  pub end_tag: Option<ArenaBox<'a, VEndTag>>,
  pub children: ArenaVec<'a, VElementChild<'a>>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum VNamespace {
  #[serde(rename = "html")]
  Html,
  #[serde(rename = "svg")]
  Svg,
  #[serde(rename = "mathml")]
  MathMl,
}

#[derive(Debug, Serialize)]
pub struct VStartTag<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub self_closing: bool,
  pub attributes: ArenaVec<'a, VAttribute<'a>>,
}

#[derive(Debug, Serialize)]
pub struct VEndTag {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
}

#[derive(Debug, Serialize)]
pub enum VElementChild<'a> {
  #[serde(rename = "VElement")]
  Element(ArenaBox<'a, VElement<'a>>),
  #[serde(rename = "VText")]
  Text(ArenaBox<'a, VText<'a>>),
  #[serde(rename = "VExpressionContainer")]
  ExpressionContainer(ArenaBox<'a, VExpressionContainer<'a>>),
}

#[derive(Debug, Serialize)]
pub struct VText<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub value: &'a str,
}

#[derive(Debug, Serialize)]
pub struct VExpressionContainer<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  /// Raw expression source between the delimiters (`{{` / `}}` for mustache,
  /// or the attribute value source for directives).
  pub raw_expression: &'a str,
  /// Span of the inner expression source (excluding mustache delimiters).
  pub expression_range: Span,
  /// `true` when this container holds a `v-for` or otherwise non-expression
  /// payload that the simple parser does not analyse beyond text capture.
  pub raw: bool,
}

#[derive(Debug, Serialize)]
pub struct VAttribute<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub directive: bool,
  pub key: ArenaBox<'a, VAttributeKey<'a>>,
  pub value: Option<ArenaBox<'a, VAttributeValue<'a>>>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VAttributeKey<'a> {
  Identifier(VIdentifier<'a>),
  Directive(VDirectiveKey<'a>),
}

#[derive(Debug, Serialize)]
pub struct VIdentifier<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub name: &'a str,
  pub raw_name: &'a str,
}

#[derive(Debug, Serialize)]
pub struct VDirectiveKey<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  /// Directive name without the `v-` / shorthand prefix (e.g. `bind`, `on`,
  /// `slot`, `for`, `model`).
  pub name: &'a str,
  /// Argument source text (e.g. for `:foo` or `v-bind:foo`, this is `foo`).
  pub argument: Option<&'a str>,
  pub modifiers: ArenaVec<'a, &'a str>,
  /// Raw source text of the whole key (e.g. `v-bind:foo.sync`, `:foo`,
  /// `@click.stop`, `#default`).
  pub raw: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VAttributeValue<'a> {
  Literal(VLiteral<'a>),
  Expression(VExpressionContainer<'a>),
}

#[derive(Debug, Serialize)]
pub struct VLiteral<'a> {
  #[serde(rename = "type")]
  pub r#type: &'static str,
  pub range: Span,
  pub value: &'a str,
}
