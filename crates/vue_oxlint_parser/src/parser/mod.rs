//! Vue SFC recursive-descent parser.

mod script;
mod template;

use std::ptr;

use oxc_allocator::{Allocator, TakeIn, Vec as ArenaVec};
use oxc_ast::{
  Comment,
  ast::{Expression, FormalParameters, Statement},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::{ParseOptions, Token};
use oxc_span::{SourceType, Span};
use oxc_syntax::module_record::ModuleRecord;
use rustc_hash::FxHashSet;

use crate::ast::{
  VAttribute, VAttributeKey, VAttributeOrDirective, VAttributeValue, VComment, VDirective,
  VDirectiveArgument, VDirectiveArgumentKind, VDirectiveExpression, VDirectiveKey,
  VDirectiveModifier, VDirectiveName, VDirectiveValue, VElement, VEndTag, VForDirective,
  VInterpolation, VNode, VOnExpression, VQuote, VScript, VScriptKind, VSlotDirective, VStartTag,
  VText, VueSingleFileComponent,
};
use crate::lexer::Lexer;

/// Result of a Vue SFC parse.
///
/// Mirrors `oxc_parser::ParserReturn` in spirit: a single struct with the
/// parsed root, side-channel metadata, and a recoverable-vs-fatal split via
/// `errors` + `panicked`.
pub struct VueParserReturn<'a, 'b> {
  pub sfc: VueSingleFileComponent<'a, 'b>,
  pub irregular_whitespaces: Box<[Span]>,
  /// Spans coming directly from a single `oxc_parser` call — see the
  /// clean-codegen-mapping RFC for how the codegen side consumes this.
  pub clean_spans: FxHashSet<Span>,
  pub module_record: ModuleRecord<'b>,
  /// Tokens from the script side, produced by `oxc_parser` with
  /// [`oxc_parser::config::RuntimeParserConfig::new(true)`].
  pub script_tokens: oxc_allocator::Vec<'b, oxc_parser::Token>,
  /// Tokens from our first-party template lexer.
  pub template_tokens: oxc_allocator::Vec<'a, crate::lexer::VToken>,
  pub errors: Vec<OxcDiagnostic>,
  /// Set on unrecoverable structural errors (e.g. unclosed `<template>`).
  pub panicked: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VueParseConfig {
  /// Whether the consumer needs the parser to record `clean_spans`. The JSX
  /// crate sets this; the toolkit side doesn't need it.
  pub track_clean_spans: bool,
}

/// Vue SFC parser.
///
/// ## Lifetimes
///
/// - `'a` owns V-tree nodes (allocated in `allocator_a`).
/// - `'b` owns nodes produced by `oxc_parser` (allocated in `allocator_b`).
/// - `'b: 'a` — V-tree nodes may borrow from `oxc_parser` output, never the
///   reverse.
///
/// Two-allocator design is documented in the RFC; phase 1 wires the lifetime
/// plumbing without committing to its correctness — the open question is
/// flagged in the RFC.
#[allow(dead_code, reason = "phase 4 parser integration will consume the stored script-side state")]
pub struct VueParser<'a, 'b>
where
  'b: 'a,
{
  allocator_a: &'a Allocator,
  allocator_b: &'b Allocator,
  origin_source_text: &'a str,

  options: ParseOptions,
  config: VueParseConfig,

  /// Template-side source used by the lexer and recursive-descent parser.
  source_text: &'a str,

  /// Mirror of the JSX crate's mutable buffer trick for `oxc_parser` calls:
  /// wrap bytes are written here, parsed, then reset to match
  /// `origin_source_text`.
  ///
  /// Spans on the resulting AST refer to original SFC offsets, not the
  /// rewritten buffer.
  oxc_source_text: &'b str,
  mut_ptr_oxc_source_text: *mut [u8],

  source_type: SourceType,
  errors: Vec<OxcDiagnostic>,
  clean_spans: FxHashSet<Span>,
  script_comments: ArenaVec<'a, Comment>,
  script_tokens: ArenaVec<'b, Token>,
  module_record: ModuleRecord<'b>,
  script_lang: Option<&'a str>,
  script_set: bool,
  script_setup_set: bool,
  panicked: bool,
}

impl<'a, 'b> VueParser<'a, 'b>
where
  'b: 'a,
{
  pub fn new(
    allocator_a: &'a Allocator,
    allocator_b: &'b Allocator,
    source_text: &'a str,
    options: ParseOptions,
    config: VueParseConfig,
  ) -> Self {
    let alloced_str_a = allocator_a.alloc_slice_copy(source_text.as_bytes());
    let alloced_str_b = allocator_b.alloc_slice_copy(source_text.as_bytes());

    Self {
      allocator_a,
      allocator_b,
      origin_source_text: source_text,
      options,
      config,

      // SAFETY: both slices were copied from a `&str`.
      source_text: unsafe { str::from_utf8_unchecked(alloced_str_a) },
      mut_ptr_oxc_source_text: ptr::from_mut(alloced_str_b),
      oxc_source_text: unsafe { str::from_utf8_unchecked(alloced_str_b) },

      source_type: SourceType::mjs().with_unambiguous(true),
      errors: Vec::new(),
      clean_spans: FxHashSet::default(),
      script_comments: ArenaVec::new_in(allocator_a),
      script_tokens: ArenaVec::new_in(allocator_b),
      module_record: ModuleRecord::new(allocator_b),
      script_lang: None,
      script_set: false,
      script_setup_set: false,
      panicked: false,
    }
  }

  /// Parse the SFC.
  #[must_use]
  pub fn parse(mut self) -> VueParserReturn<'a, 'b> {
    let mut lexer = Lexer::new(self.allocator_a, self.source_text);
    lexer.lex_all();
    let template_tokens = lexer.take_tokens();
    self.errors.append(&mut lexer.take_errors());

    let children = self.parse_nodes(0, self.source_text.len() as u32, None);
    let irregular_whitespaces = collect_irregular_whitespaces(self.source_text);

    VueParserReturn {
      sfc: VueSingleFileComponent {
        children,
        script_comments: self.script_comments,
        source_type: self.source_type,
      },
      irregular_whitespaces,
      clean_spans: self.clean_spans,
      module_record: self.module_record,
      script_tokens: self.script_tokens,
      template_tokens,
      errors: self.errors,
      panicked: self.panicked,
    }
  }

  /// Reset the mutable source buffer to match the original source.
  ///
  /// Called after each in-place wrap-and-parse cycle (see the RFC's
  /// "Reusing the `oxc_parse` mutation trick" section).
  pub const fn sync_source_text(&mut self) {
    // SAFETY: `self.origin_source_text` and `self.mut_ptr_oxc_source_text` have
    // identical lengths; the former lives on the heap and the latter in the
    // arena, so the regions cannot overlap.
    unsafe {
      ptr::copy_nonoverlapping(
        self.origin_source_text.as_ptr(),
        self.mut_ptr_oxc_source_text.cast(),
        self.origin_source_text.len(),
      );
    }
  }

  fn parse_nodes(
    &mut self,
    start: u32,
    end: u32,
    close_tag: Option<&str>,
  ) -> ArenaVec<'a, VNode<'a, 'b>> {
    let mut nodes = ArenaVec::new_in(self.allocator_a);
    let mut pos = start as usize;
    let end = end as usize;

    while pos < end {
      let source = self.source_text.as_bytes();
      if source.get(pos..pos + 2) == Some(b"{{") {
        let Some(close_offset) = self.source_text[pos + 2..end].find("}}") else {
          self.push_fatal_error(
            "Interpolation is missing end delimiter.",
            Span::new(pos as u32, end as u32),
          );
          nodes.push(VNode::Text(self.text_node(pos, end)));
          break;
        };
        let expr_start = pos + 2;
        let expr_end = pos + 2 + close_offset;
        let trimmed_start = expr_start + self.source_text[expr_start..expr_end].len()
          - self.source_text[expr_start..expr_end].trim_start().len();
        let trimmed_end = expr_end
          - (self.source_text[expr_start..expr_end].len()
            - self.source_text[expr_start..expr_end].trim_end().len());
        let span = Span::new(pos as u32, (expr_end + 2) as u32);
        if let Some(expression) = self.parse_pure_expression(
          Span::new(trimmed_start as u32, trimmed_end as u32),
          self.allocator_b,
        ) {
          nodes.push(VNode::Interpolation(VInterpolation {
            expression: self.allocator_b.alloc(expression),
            span,
          }));
        } else {
          nodes.push(VNode::Text(self.text_node(pos, expr_end + 2)));
        }
        pos = expr_end + 2;
        continue;
      }

      if source.get(pos) != Some(&b'<') {
        let next_lt = memchr::memchr(b'<', &source[pos..end]).map_or(end, |offset| pos + offset);
        let next_interp = memchr::memchr(b'{', &source[pos..end])
          .and_then(|offset| {
            let candidate = pos + offset;
            (source.get(candidate..candidate + 2) == Some(b"{{")).then_some(candidate)
          })
          .unwrap_or(end);
        let next = next_lt.min(next_interp);
        if next > pos {
          nodes.push(VNode::Text(self.text_node(pos, next)));
        } else {
          nodes.push(VNode::Text(self.text_node(pos, pos + 1)));
          pos += 1;
          continue;
        }
        pos = next;
        continue;
      }

      if source.get(pos + 1) == Some(&b'/') {
        if let Some(tag) = close_tag
          && let Some(end_tag) = self.parse_end_tag_at(pos, tag)
        {
          nodes.push(VNode::Text(self.text_node(pos, end_tag.span.end as usize)));
          pos = end_tag.span.end as usize;
          continue;
        }
        let name_start = pos + 2;
        let name_end = scan_tag_name(source, name_start);
        if name_end > name_start && is_void_tag(&self.source_text[name_start..name_end]) {
          let text_end = source[pos..end]
            .iter()
            .position(|byte| *byte == b'>')
            .map_or(end, |i| (pos + i + 1).min(end));
          self.push_error(
            "Void elements must not have end tags.",
            Span::new(pos as u32, text_end as u32),
          );
          nodes.push(VNode::Text(self.text_node(pos, text_end)));
          pos = text_end;
          continue;
        }
        let text_end = source[pos..end]
          .iter()
          .position(|byte| *byte == b'>')
          .map_or(end, |i| (pos + i + 1).min(end));
        nodes.push(VNode::Text(self.text_node(pos, text_end)));
        pos = text_end;
        continue;
      }

      if self.source_text[pos..end].starts_with("<!--") {
        let comment_end =
          self.source_text[pos + 4..end].find("-->").map_or(end, |offset| pos + 4 + offset + 3);
        nodes.push(VNode::Comment(VComment {
          value: self.slice(pos + 4, comment_end.saturating_sub(3)),
          span: Span::new(pos as u32, comment_end as u32),
        }));
        pos = comment_end;
        continue;
      }

      if let Some((element, next_pos)) = self.parse_element_at(pos, end) {
        nodes.push(VNode::Element(element));
        pos = next_pos;
      } else {
        nodes.push(VNode::Text(self.text_node(pos, pos + 1)));
        pos += 1;
      }
    }

    nodes
  }

  fn parse_element_at(
    &mut self,
    pos: usize,
    limit: usize,
  ) -> Option<(&'a VElement<'a, 'b>, usize)> {
    let bytes = self.source_text.as_bytes();
    if bytes.get(pos) != Some(&b'<') || !bytes.get(pos + 1).is_some_and(u8::is_ascii_alphabetic) {
      return None;
    }

    let name_start = pos + 1;
    let name_end = scan_tag_name(bytes, name_start);
    let tag_name = self.source_text[name_start..name_end].to_ascii_lowercase();
    let Some(start_tag_end) = scan_tag_end(bytes, name_end, limit) else {
      self.push_fatal_error("Unexpected EOF in tag.", Span::new(pos as u32, limit as u32));
      return None;
    };
    let self_closing = bytes.get(start_tag_end.saturating_sub(2)..start_tag_end) == Some(b"/>");
    let start_tag = VStartTag {
      name_span: Span::new(name_start as u32, name_end as u32),
      attributes: self
        .parse_attributes(name_end, start_tag_end.saturating_sub(1 + usize::from(self_closing))),
      self_closing,
      span: Span::new(pos as u32, start_tag_end as u32),
    };

    if self_closing || is_void_tag(&tag_name) {
      return Some((
        &*self.allocator_a.alloc(VElement {
          start_tag,
          end_tag: None,
          children: ArenaVec::new_in(self.allocator_a),
          script: None,
          span: Span::new(pos as u32, start_tag_end as u32),
        }),
        start_tag_end,
      ));
    }

    let (body_end, end_tag, next_pos) = if let Some(close_start) =
      find_matching_close(self.source_text, start_tag_end, limit, &tag_name)
    {
      let end_tag = self.parse_end_tag_at(close_start, &tag_name);
      let next = end_tag.as_ref().map_or(close_start, |tag| tag.span.end as usize);
      (close_start, end_tag, next)
    } else {
      self.push_fatal_error(
        format!("Element <{tag_name}> is missing end tag."),
        Span::new(pos as u32, start_tag_end as u32),
      );
      (limit, None, limit)
    };

    let script = if tag_name == "script" {
      let kind = if has_setup_attribute(&start_tag, self.source_text) {
        VScriptKind::Setup
      } else {
        VScriptKind::Script
      };
      let lang = find_lang_attribute(&start_tag, self.source_text);
      let parser_kind = match kind {
        VScriptKind::Script => script::ScriptKind::Script,
        VScriptKind::Setup => script::ScriptKind::Setup,
      };
      let body_span = Span::new(start_tag_end as u32, body_end as u32);
      let element_span = Span::new(pos as u32, next_pos as u32);
      self
        .parse_script_block_with_diagnostic_span(body_span, element_span, lang, parser_kind)
        .map(|program| VScript { kind, body_span, program })
    } else {
      None
    };

    let children = if tag_name == "script" || tag_name == "style" {
      let mut raw_children = ArenaVec::new_in(self.allocator_a);
      if body_end > start_tag_end {
        raw_children.push(VNode::Text(self.text_node(start_tag_end, body_end)));
      }
      raw_children
    } else {
      self.parse_nodes(start_tag_end as u32, body_end as u32, Some(&tag_name))
    };

    Some((
      &*self.allocator_a.alloc(VElement {
        start_tag,
        end_tag,
        children,
        script,
        span: Span::new(pos as u32, next_pos as u32),
      }),
      next_pos,
    ))
  }

  fn parse_end_tag_at(&self, pos: usize, expected: &str) -> Option<VEndTag> {
    let bytes = self.source_text.as_bytes();
    if bytes.get(pos..pos + 2) != Some(b"</") {
      return None;
    }
    let name_start = pos + 2;
    let name_end = scan_tag_name(bytes, name_start);
    if !self.source_text[name_start..name_end].eq_ignore_ascii_case(expected) {
      return None;
    }
    let end = scan_tag_end(bytes, name_end, self.source_text.len())?;
    Some(VEndTag {
      name_span: Span::new(name_start as u32, name_end as u32),
      span: Span::new(pos as u32, end as u32),
    })
  }

  fn parse_attributes(
    &mut self,
    start: usize,
    end: usize,
  ) -> ArenaVec<'a, VAttributeOrDirective<'a, 'b>> {
    let mut attributes = ArenaVec::new_in(self.allocator_a);
    let bytes = self.source_text.as_bytes();
    let mut pos = start;

    while pos < end {
      while pos < end && is_html_whitespace(bytes[pos]) {
        pos += 1;
      }
      if pos >= end {
        break;
      }

      let key_start = pos;
      while pos < end && !is_html_whitespace(bytes[pos]) && bytes[pos] != b'=' {
        pos += 1;
      }
      let key_end = pos;
      while pos < end && is_html_whitespace(bytes[pos]) {
        pos += 1;
      }

      let (value, attr_end) = if pos < end && bytes[pos] == b'=' {
        pos += 1;
        while pos < end && is_html_whitespace(bytes[pos]) {
          pos += 1;
        }
        let value_start = pos;
        let quote = match bytes.get(pos).copied() {
          Some(b'"') => {
            pos += 1;
            VQuote::Double
          }
          Some(b'\'') => {
            pos += 1;
            VQuote::Single
          }
          _ => VQuote::Unquoted,
        };
        let raw_start = pos;
        match quote {
          VQuote::Double => {
            while pos < end && bytes[pos] != b'"' {
              pos += 1;
            }
          }
          VQuote::Single => {
            while pos < end && bytes[pos] != b'\'' {
              pos += 1;
            }
          }
          VQuote::Unquoted => {
            while pos < end && !is_html_whitespace(bytes[pos]) {
              pos += 1;
            }
          }
        }
        let raw_end = pos;
        if !matches!(quote, VQuote::Unquoted) && pos < end {
          pos += 1;
        }
        (
          Some(VAttributeValue {
            raw: self.slice(raw_start, raw_end),
            value: self.slice(raw_start, raw_end),
            span: Span::new(value_start as u32, pos as u32),
            quote,
          }),
          pos,
        )
      } else {
        (None, key_end)
      };

      let key_span = Span::new(key_start as u32, key_end as u32);
      let attr_span = Span::new(key_start as u32, attr_end as u32);
      let key = self.slice(key_start, key_end);
      if is_directive_key(key) {
        attributes.push(VAttributeOrDirective::Directive(
          self.parse_directive(key, key_span, value, attr_span),
        ));
      } else {
        let attr =
          VAttribute { key: VAttributeKey { name: key, span: key_span }, value, span: attr_span };
        attributes.push(VAttributeOrDirective::Attribute(attr));
      }
    }

    attributes
  }

  fn parse_directive(
    &mut self,
    key: &'a str,
    key_span: Span,
    value: Option<VAttributeValue<'a>>,
    span: Span,
  ) -> VDirective<'a, 'b> {
    let parsed_key = self.parse_directive_key(key, key_span);
    let value = value.and_then(|value| {
      let expression = if value.raw.trim().is_empty() {
        return None;
      } else {
        let expression_span = trim_span(value.raw, value.span, value.quote);
        self.parse_directive_expression(parsed_key.name.name, value.raw, expression_span)
      }?;

      Some(VDirectiveValue { raw: value.raw, span: value.span, quote: value.quote, expression })
    });

    VDirective { key: parsed_key, value, span }
  }

  fn parse_directive_expression(
    &mut self,
    name: &str,
    raw: &str,
    span: Span,
  ) -> Option<VDirectiveExpression<'a, 'b>> {
    match name {
      "for" => self.parse_v_for_expression(raw, span).map(VDirectiveExpression::VFor),
      "slot" => self.parse_v_slot_expression(span).map(VDirectiveExpression::VSlot),
      "on" => self.parse_v_on_expression(span).map(VDirectiveExpression::VOn),
      _ => self
        .parse_pure_expression(span, self.allocator_b)
        .map(|expression| VDirectiveExpression::Expression(&*self.allocator_b.alloc(expression))),
    }
  }

  fn parse_v_for_expression(&mut self, raw: &str, span: Span) -> Option<VForDirective<'b>> {
    let (left_start, left_end, right_start, right_end) = split_v_for_expression(raw, span)?;
    let left_span = Span::new(left_start, left_end);
    let right_span = Span::new(right_start, right_end);

    let left = self.parse_arrow_parameters(left_span)?;
    let right = self.parse_pure_expression(right_span, self.allocator_b)?;

    Some(VForDirective { left, right: self.allocator_b.alloc(right) })
  }

  fn parse_v_slot_expression(&mut self, span: Span) -> Option<VSlotDirective<'b>> {
    self.parse_arrow_parameters(span).map(|params| VSlotDirective { params })
  }

  fn parse_v_on_expression(&mut self, span: Span) -> Option<VOnExpression<'a, 'b>> {
    let ret = self.parse_program_region(span, b"{", b"}", self.allocator_b)?;
    let mut body = ret.program.body;
    let Some(Statement::BlockStatement(block)) = body.get_mut(0) else {
      return None;
    };
    let block_body = block.body.take_in(self.allocator_b);
    let mut statements = ArenaVec::new_in(self.allocator_a);
    for statement in block_body {
      statements.push(statement);
    }

    Some(VOnExpression { statements })
  }

  fn parse_arrow_parameters(&mut self, span: Span) -> Option<&'b FormalParameters<'b>> {
    let params = span.source_text(self.source_text).trim();
    let (start_wrap, end_wrap, trim_outer_parens) =
      if params.starts_with('(') && params.ends_with(')') {
        (b"(" as &[u8], b"=>0)" as &[u8], false)
      } else {
        (b"((" as &[u8], b")=>0)" as &[u8], true)
      };

    let mut ret = self.parse_program_region(span, start_wrap, end_wrap, self.allocator_b)?;
    let stmt = ret.program.body.get_mut(0)?;
    let Statement::ExpressionStatement(stmt) = stmt else {
      return None;
    };
    let arrow = match &mut stmt.expression {
      Expression::ArrowFunctionExpression(arrow) => arrow,
      Expression::ParenthesizedExpression(expression) => {
        let Expression::ArrowFunctionExpression(arrow) = &mut expression.expression else {
          return None;
        };
        arrow
      }
      _ => return None,
    };
    let mut params = arrow.params.take_in(self.allocator_b);
    if trim_outer_parens {
      params.span = span;
    }
    Some(self.allocator_b.alloc(params))
  }

  #[allow(clippy::option_if_let_else, reason = "the directive prefix split is clearer as branches")]
  fn parse_directive_key(&mut self, key: &'a str, span: Span) -> VDirectiveKey<'a, 'b> {
    let (name, rest, name_span) = if let Some(rest) = key.strip_prefix("v-") {
      let name_end = rest.find([':', '.']).map_or(key.len(), |index| 2 + index);
      (
        self.slice(span.start as usize + 2, span.start as usize + name_end),
        &key[name_end..],
        Span::new(span.start + 2, span.start + name_end as u32),
      )
    } else {
      let (name, rest) = match key.as_bytes()[0] {
        b':' | b'.' => ("bind", &key[1..]),
        b'@' => ("on", &key[1..]),
        b'#' => ("slot", &key[1..]),
        _ => unreachable!("caller checks directive prefix"),
      };
      (name, rest, Span::sized(span.start, 1))
    };

    let mut argument = None;
    let mut modifiers = ArenaVec::new_in(self.allocator_a);
    let rest_start = span.end - rest.len() as u32;
    let mut segment_start = 0;
    let mut in_argument = rest.starts_with(':') || !key.starts_with("v-");

    if rest.starts_with(':') {
      segment_start = 1;
    }

    for (index, byte) in rest.bytes().enumerate() {
      if byte != b'.' {
        continue;
      }
      if in_argument && index > segment_start {
        argument = Some(self.directive_argument(rest, rest_start, segment_start, index));
        in_argument = false;
      }
      let mod_start = index + 1;
      if mod_start < rest.len() {
        let mod_end = rest[mod_start..].find('.').map_or(rest.len(), |offset| mod_start + offset);
        modifiers.push(VDirectiveModifier {
          name: self.slice((rest_start as usize) + mod_start, (rest_start as usize) + mod_end),
          span: Span::new(rest_start + mod_start as u32, rest_start + mod_end as u32),
        });
      }
    }

    if in_argument && segment_start < rest.len() {
      argument = Some(self.directive_argument(rest, rest_start, segment_start, rest.len()));
    }

    VDirectiveKey { name: VDirectiveName { name, span: name_span }, argument, modifiers, span }
  }

  fn directive_argument(
    &mut self,
    _rest: &'a str,
    rest_start: u32,
    start: usize,
    end: usize,
  ) -> VDirectiveArgument<'a, 'b> {
    let raw = self.slice(rest_start as usize + start, rest_start as usize + end);
    let kind = if raw.starts_with('[') && raw.ends_with(']') {
      VDirectiveArgumentKind::Dynamic
    } else {
      VDirectiveArgumentKind::Static
    };

    let span = Span::new(rest_start + start as u32, rest_start + end as u32);
    let expression = if kind == VDirectiveArgumentKind::Dynamic && raw.len() >= 2 {
      self
        .parse_pure_expression(Span::new(span.start + 1, span.end - 1), self.allocator_b)
        .map(|expression| &*self.allocator_b.alloc(expression))
    } else {
      None
    };

    VDirectiveArgument { raw, kind, expression, span }
  }

  fn text_node(&self, start: usize, end: usize) -> VText<'a> {
    VText {
      raw: self.slice(start, end),
      value: self.slice(start, end),
      span: Span::new(start as u32, end as u32),
    }
  }

  fn slice(&self, start: usize, end: usize) -> &'a str {
    self.allocator_a.alloc_str(&self.source_text[start..end])
  }

  fn push_error(&mut self, message: impl Into<String>, span: Span) {
    self.errors.push(OxcDiagnostic::error(message.into()).with_label(span));
  }

  fn push_fatal_error(&mut self, message: impl Into<String>, span: Span) {
    self.panicked = true;
    self.push_error(message, span);
  }
}

fn scan_tag_name(bytes: &[u8], mut pos: usize) -> usize {
  while let Some(byte) = bytes.get(pos) {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.') {
      pos += 1;
    } else {
      break;
    }
  }
  pos
}

fn scan_tag_end(bytes: &[u8], mut pos: usize, limit: usize) -> Option<usize> {
  let mut quote = None;
  while pos < limit {
    let byte = bytes[pos];
    if let Some(active_quote) = quote {
      if byte == active_quote {
        quote = None;
      }
      pos += 1;
      continue;
    }
    if byte == b'"' || byte == b'\'' {
      quote = Some(byte);
      pos += 1;
      continue;
    }
    if byte == b'>' {
      return Some(pos + 1);
    }
    pos += 1;
  }
  None
}

fn find_matching_close(source: &str, start: usize, limit: usize, tag: &str) -> Option<usize> {
  let bytes = source.as_bytes();
  let mut pos = start;
  while pos < limit {
    let rel = memchr::memchr(b'<', &bytes[pos..limit])?;
    pos += rel;
    if bytes.get(pos + 1) == Some(&b'/') {
      let name_start = pos + 2;
      let name_end = scan_tag_name(bytes, name_start);
      if source[name_start..name_end].eq_ignore_ascii_case(tag) {
        return Some(pos);
      }
    }
    pos += 1;
  }
  None
}

const fn is_html_whitespace(byte: u8) -> bool {
  matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0C)
}

fn is_directive_key(key: &str) -> bool {
  key.starts_with("v-")
    || key.starts_with(':')
    || key.starts_with('@')
    || key.starts_with('#')
    || key.starts_with('.')
}

fn trim_span(raw: &str, value_span: Span, quote: VQuote) -> Span {
  let base_start = match quote {
    VQuote::Double | VQuote::Single => value_span.start + 1,
    VQuote::Unquoted => value_span.start,
  };
  let leading = raw.len() - raw.trim_start().len();
  let trailing = raw.len() - raw.trim_end().len();

  Span::new(base_start + leading as u32, base_start + (raw.len() - trailing) as u32)
}

fn split_v_for_expression(raw: &str, span: Span) -> Option<(u32, u32, u32, u32)> {
  for (index, _) in raw.char_indices() {
    let matched = if raw[index..].starts_with(" in ") || raw[index..].starts_with(" of ") {
      Some((index, 4))
    } else {
      None
    };

    if let Some((separator_start, separator_len)) = matched {
      let left = raw[..separator_start].trim();
      let right = raw[separator_start + separator_len..].trim();
      if left.is_empty() || right.is_empty() {
        return None;
      }

      let left_start = raw[..separator_start].len() - raw[..separator_start].trim_start().len();
      let left_end = left_start + left.len();
      let right_offset = separator_start
        + separator_len
        + (raw[separator_start + separator_len..].len() - right.len());

      return Some((
        span.start + left_start as u32,
        span.start + left_end as u32,
        span.start + right_offset as u32,
        span.start + (right_offset + right.len()) as u32,
      ));
    }
  }

  None
}

fn is_void_tag(name: &str) -> bool {
  matches!(
    name,
    "area"
      | "base"
      | "br"
      | "col"
      | "embed"
      | "hr"
      | "img"
      | "input"
      | "link"
      | "meta"
      | "param"
      | "source"
      | "track"
      | "wbr"
  )
}

fn has_setup_attribute(start_tag: &VStartTag<'_, '_>, source: &str) -> bool {
  start_tag.attributes.iter().any(|attr| match attr {
    VAttributeOrDirective::Attribute(attr) => attr.key.name == "setup",
    VAttributeOrDirective::Directive(_) => false,
  }) || source[start_tag.span.start as usize..start_tag.span.end as usize].contains(" setup")
}

fn find_lang_attribute<'a>(start_tag: &VStartTag<'a, '_>, _source: &str) -> Option<&'a str> {
  start_tag.attributes.iter().find_map(|attr| match attr {
    VAttributeOrDirective::Attribute(attr) if attr.key.name == "lang" => {
      attr.value.as_ref().map(|value| value.value)
    }
    _ => None,
  })
}

fn collect_irregular_whitespaces(source_text: &str) -> Box<[Span]> {
  let mut irregular_whitespaces = Vec::new();
  let mut offset = 0;
  for c in source_text.chars() {
    if oxc_syntax::identifier::is_irregular_whitespace(c) {
      irregular_whitespaces.push(Span::sized(offset, c.len_utf8() as u32));
    }
    offset += c.len_utf8() as u32;
  }

  irregular_whitespaces.into_boxed_slice()
}

#[cfg(test)]
mod tests {
  use oxc_allocator::Allocator;
  use oxc_ast::ast::{Expression, Statement};
  use oxc_parser::ParseOptions;

  use super::{VueParseConfig, VueParser};
  use crate::ast::{
    VAttributeOrDirective, VDirectiveArgumentKind, VDirectiveExpression, VNode, VScriptKind,
  };

  fn parse<'a>(
    allocator_a: &'a Allocator,
    allocator_b: &'a Allocator,
    source: &'a str,
  ) -> super::VueParserReturn<'a, 'a> {
    VueParser::new(
      allocator_a,
      allocator_b,
      source,
      ParseOptions::default(),
      VueParseConfig { track_clean_spans: true },
    )
    .parse()
  }

  #[test]
  fn parse_builds_top_level_v_tree() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let ret = parse(&allocator_a, &allocator_b, "<template><div id=\"app\">hi</div></template>");

    assert!(!ret.panicked);
    assert!(ret.errors.is_empty());
    assert_eq!(ret.sfc.children.len(), 1);

    let VNode::Element(template) = &ret.sfc.children[0] else {
      panic!("root should be an element");
    };
    assert_eq!(
      template.start_tag.name_span.source_text("<template><div id=\"app\">hi</div></template>"),
      "template"
    );
    assert_eq!(template.children.len(), 1);
  }

  #[test]
  fn parse_script_block_once_and_collects_side_channels() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let source = "<script lang=\"ts\">import foo from 'foo';\nconst answer = 42;</script>";
    let ret = parse(&allocator_a, &allocator_b, source);

    let VNode::Element(script) = &ret.sfc.children[0] else {
      panic!("root should be script");
    };
    let script = script.script.as_ref().expect("script program should be attached");

    assert_eq!(script.kind, VScriptKind::Script);
    assert!(ret.sfc.source_type.is_typescript());
    assert_eq!(script.program.body.len(), 2);
    assert!(matches!(script.program.body[0], Statement::ImportDeclaration(_)));
    assert_eq!(ret.module_record.import_entries.len(), 1);
    assert_eq!(ret.clean_spans.len(), 2);
    assert!(!ret.script_tokens.is_empty());
  }

  #[test]
  fn parse_template_interpolation_expression() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let ret = parse(&allocator_a, &allocator_b, "<template>{{ answer + 1 }}</template>");

    let VNode::Element(template) = &ret.sfc.children[0] else {
      panic!("root should be template");
    };
    let VNode::Interpolation(interpolation) = &template.children[0] else {
      panic!("template child should be interpolation");
    };

    assert!(matches!(interpolation.expression, Expression::BinaryExpression(_)));
  }

  #[test]
  fn parse_vue_directive_keys_and_values() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let source = r#"<template><button :class.foo="cls" @click.stop="submit" v-if="ok" #default="slotProps" /></template>"#;
    let ret = parse(&allocator_a, &allocator_b, source);

    let VNode::Element(template) = &ret.sfc.children[0] else {
      panic!("root should be template");
    };
    let VNode::Element(button) = &template.children[0] else {
      panic!("template child should be button");
    };

    let directives = button
      .start_tag
      .attributes
      .iter()
      .filter_map(|attribute| match attribute {
        VAttributeOrDirective::Directive(directive) => Some(directive),
        VAttributeOrDirective::Attribute(_) => None,
      })
      .collect::<Vec<_>>();

    assert_eq!(directives.len(), 4);
    assert_eq!(directives[0].key.name.name, "bind");
    assert_eq!(directives[0].key.argument.as_ref().unwrap().raw, "class");
    assert_eq!(directives[0].key.modifiers[0].name, "foo");
    assert!(matches!(
      directives[0].value.as_ref().unwrap().expression,
      VDirectiveExpression::Expression(Expression::Identifier(_))
    ));
    assert_eq!(directives[1].key.name.name, "on");
    assert_eq!(directives[1].key.argument.as_ref().unwrap().raw, "click");
    assert_eq!(directives[1].key.modifiers[0].name, "stop");
    assert_eq!(directives[2].key.name.name, "if");
    assert_eq!(directives[3].key.name.name, "slot");
    assert_eq!(directives[3].key.argument.as_ref().unwrap().raw, "default");
  }

  #[test]
  fn parse_dynamic_directive_argument() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let source = r#"<template><div v-bind:[name]="value" /></template>"#;
    let ret = parse(&allocator_a, &allocator_b, source);

    let VNode::Element(template) = &ret.sfc.children[0] else {
      panic!("root should be template");
    };
    let VNode::Element(div) = &template.children[0] else {
      panic!("template child should be div");
    };
    let VAttributeOrDirective::Directive(directive) = &div.start_tag.attributes[0] else {
      panic!("attribute should be a directive");
    };

    let argument = directive.key.argument.as_ref().unwrap();
    assert_eq!(argument.raw, "[name]");
    assert_eq!(argument.kind, VDirectiveArgumentKind::Dynamic);
    assert!(matches!(argument.expression, Some(Expression::Identifier(_))));
  }

  #[test]
  fn parse_special_directive_expressions() {
    let allocator_a = Allocator::new();
    let allocator_b = Allocator::new();
    let source = r#"<template><div v-for="(item, index) in items" v-slot="slotProps" @click="count += 1" /></template>"#;
    let ret = parse(&allocator_a, &allocator_b, source);

    let VNode::Element(template) = &ret.sfc.children[0] else {
      panic!("root should be template");
    };
    let VNode::Element(div) = &template.children[0] else {
      panic!("template child should be div");
    };

    let directives = div
      .start_tag
      .attributes
      .iter()
      .filter_map(|attribute| match attribute {
        VAttributeOrDirective::Directive(directive) => Some(directive),
        VAttributeOrDirective::Attribute(_) => None,
      })
      .collect::<Vec<_>>();

    let VDirectiveExpression::VFor(v_for) = &directives[0].value.as_ref().unwrap().expression
    else {
      panic!("v-for should parse as VFor");
    };
    assert_eq!(v_for.left.items.len(), 2);
    assert!(matches!(v_for.right, Expression::Identifier(_)));

    let VDirectiveExpression::VSlot(v_slot) = &directives[1].value.as_ref().unwrap().expression
    else {
      panic!("v-slot should parse as VSlot");
    };
    assert_eq!(v_slot.params.items.len(), 1);

    let VDirectiveExpression::VOn(v_on) = &directives[2].value.as_ref().unwrap().expression else {
      panic!("v-on should parse as VOn");
    };
    assert_eq!(v_on.statements.len(), 1);
  }
}
