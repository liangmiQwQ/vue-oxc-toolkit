//! Token types emitted by the Vue template lexer.
//!
//! The lexer produces a flat stream of these tokens; the parser consumes
//! them and assembles `V*` AST nodes. Span fields are byte offsets into the
//! original SFC source.

use crate::ast::Span;

/// A lexed token.
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
  pub span: Span,
  pub kind: TokenKind<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum TokenKind<'a> {
  /// `<name` — start of an opening tag. The name slice points into source.
  TagOpen { name: &'a str },
  /// An attribute name inside a start tag.
  AttrName { name: &'a str },
  /// `=` between attribute name and value.
  AttrEq,
  /// Attribute value. `quote` is `Some('"' | '\'')` for quoted, `None` for
  /// unquoted. `inner_span` excludes any quote characters.
  AttrValue { value: &'a str, quote: Option<u8>, inner_span: Span },
  /// `/>` ending the start tag.
  TagSelfClose,
  /// `>` ending the start tag.
  TagEnd,
  /// `</name>` — full end tag consumed in one shot.
  EndTag { name: &'a str },
  /// Plain text content.
  Text { text: &'a str },
  /// `{{ expression }}` — entire mustache. `expr_span` covers just the
  /// expression text between the delimiters.
  Mustache { expr: &'a str, expr_span: Span },
  /// `<!-- ... -->` (or a recovered bogus comment).
  Comment,
  /// `<![CDATA[ ... ]]>`.
  Cdata,
  /// `<!DOCTYPE ...>` and similar bang-led constructs.
  Bang,
  /// `<? ... ?>` processing instruction.
  ProcessingInstruction,
  /// End of input.
  Eof,
}

/// Lexer mode — set by the parser to drive contextual tokenization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexMode<'a> {
  /// Default content. Recognizes tags and `{{` mustaches.
  Data,
  /// Inside a start tag, lexing attributes until `>` or `/>`.
  InTag,
  /// Lexing an unquoted attribute value: consumes everything up to the
  /// next ASCII whitespace, `>`, or `/>`. The parser switches to this
  /// mode immediately after consuming an `AttrEq` token.
  AttrValueUnquoted,
  /// Raw-text element body (`<script>`, `<style>`). Emits a single `Text`
  /// token spanning until the matching `</name>`.
  RawText { name: &'a str },
  /// RCDATA element body (`<textarea>`). Emits text and mustaches until
  /// the matching `</name>`, but does not recognize start tags.
  RcData { name: &'a str },
}
