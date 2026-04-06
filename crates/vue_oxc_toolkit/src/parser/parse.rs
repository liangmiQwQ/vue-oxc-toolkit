use std::cmp::Ordering;
use std::collections::HashSet;

use oxc_allocator::{self, Dummy, Vec as ArenaVec};
use oxc_ast::ast::{Directive, Expression, FormalParameterKind, JSXChild, Program, Statement};
use oxc_ast::{AstBuilder, NONE};

use oxc_span::{SPAN, Span};
use oxc_syntax::module_record::ModuleRecord;
use vize_armature::{ParserOptions, SourceLocation, TemplateChildNode, WhitespaceStrategy};

use crate::is_void_tag;
use crate::parser::error;
use crate::parser::{ResParse, ResParseExt};

use super::ParserImpl;
use super::ParserImplReturn;

impl<'a> ParserImpl<'a> {
  pub fn parse(mut self) -> ParserImplReturn<'a> {
    let result = self.analyze();
    match result {
      Ok(()) => {
        self.fix_module_records();

        let Self {
          source_text,
          ast,
          module_record,
          source_type,
          comments,
          errors,
          global,
          setup,
          sfc_struct_jsx_statement: sfc_return,
          ..
        } = self;

        ParserImplReturn {
          program: ast.program(
            Span::new(0, self.source_text.len() as u32),
            source_type.with_jsx(true),
            source_text,
            comments,
            None, // no hashbang needed for vue files
            global.directives,
            Self::get_body_statements(
              global.statements,
              setup.statements,
              setup.directives,
              sfc_return,
              ast,
            ),
          ),
          fatal: false,
          errors,
          module_record,
        }
      }
      Err(()) => ParserImplReturn {
        program: Program::dummy(self.allocator),
        fatal: true,
        errors: self.errors,
        module_record: ModuleRecord::new(self.allocator),
      },
    }
  }

  fn get_body_statements(
    mut statements: ArenaVec<'a, Statement<'a>>,
    mut setup: ArenaVec<'a, Statement<'a>>,
    setup_directives: ArenaVec<'a, Directive<'a>>,
    sfc_return: Option<Statement<'a>>,
    ast: AstBuilder<'a>,
  ) -> ArenaVec<'a, Statement<'a>> {
    if let Some(ret) = sfc_return {
      setup.push(ret);
    }

    let params = ast.alloc_formal_parameters(
      SPAN,
      FormalParameterKind::ArrowFormalParameters,
      ast.vec(),
      NONE,
    );

    let body = ast.alloc_function_body(SPAN, setup_directives, setup);

    statements.push(ast.statement_expression(
      SPAN,
      Expression::ArrowFunctionExpression(
        ast.alloc_arrow_function_expression(SPAN, false, true, NONE, params, NONE, body),
      ),
    ));

    statements
  }
}

impl<'a> ParserImpl<'a> {
  fn analyze(&mut self) -> ResParse<()> {
    let vize_allocator = vize_armature::Allocator::new();
    let bump = vize_allocator.as_bump();

    let options = ParserOptions {
      whitespace: WhitespaceStrategy::Condense,
      is_void_tag: |name| is_void_tag!(name),
      ..Default::default()
    };

    // vize doesn't parse <script>/<style> in RAWTEXT mode yet (TODO in vize).
    // Sanitize source so that `<` inside script/style blocks don't create fake elements.
    let sanitized = sanitize_rawtext_blocks(self.source_text);
    let vize_source = sanitized.as_deref().unwrap_or(self.source_text);

    let (root, vize_errors) = vize_armature::parse_with_options(bump, vize_source, options);

    // Process errors, but filter MissingEndTag which can be a false positive from
    // rawtext sanitization gaps and is handled by should_panic anyway.
    let panicked = error::process_vize_errors(&vize_errors, &mut self.errors);
    if panicked {
      return ResParse::panic();
    }

    // First pass: process script elements, collect children as JSXChild
    // We process everything in one pass while vize allocator is alive
    let mut children: ArenaVec<'a, JSXChild<'a>> = self.ast.vec();
    let mut text_start: u32 = 0;
    let mut source_types: HashSet<&str> = HashSet::new();

    // First pass: process script elements (must be parsed before template for source type)
    for child in &root.children {
      if let TemplateChildNode::Element(node) = child {
        let tag_text = node.loc.span().source_text(self.source_text);
        if tag_text.starts_with("<script") {
          self.parse_script(node, &mut source_types)?;
        }
      }
    }

    // Second pass: process all children in order
    for child in &root.children {
      if let TemplateChildNode::Element(node) = child {
        let node_loc_start = node.loc.start.offset;

        // Process the texts between last element and current element
        let text_span = Span::new(text_start, node_loc_start);
        if !text_span.is_empty() {
          let atom = self.ast.str(text_span.source_text(self.source_text));
          children.push(self.ast.jsx_child_text(text_span, atom, Some(atom)));
        }

        // Compute true end of element (vize loc only covers opening tag)
        let tag_name = node.tag.as_str();
        let true_end = if node.is_self_closing || is_void_tag!(tag_name) {
          node.loc.end.offset
        } else {
          self.element_close_span(node.loc.end.offset, tag_name).end
        };
        text_start = true_end;

        let tag_text = node.loc.span().source_text(self.source_text);
        if tag_text.starts_with("<script") {
          children.push(self.parse_element_ref(node, Some(self.ast.vec())).0);
        } else if tag_text.starts_with("<template") {
          children.push(self.parse_element_ref(node, None).0);
        } else {
          // Process other tags like <style>
          let text = if let Some(first) = node.children.first() {
            let last = node.children.last().unwrap();
            let span = Span::new(first.loc().start.offset, last.loc().end.offset);

            let atom = self.ast.str(span.source_text(self.source_text));
            self.ast.vec1(self.ast.jsx_child_text(span, atom, Some(atom)))
          } else {
            self.ast.vec()
          };

          children.push(self.parse_element_ref(node, Some(text)).0);
        }
      }
    }
    // Process the texts after last element
    let text_span = Span::new(text_start, self.source_text.len() as u32);
    if !text_span.is_empty() {
      let atom = self.ast.str(text_span.source_text(self.source_text));
      children.push(self.ast.jsx_child_text(text_span, atom, Some(atom)));
    }

    self.sort_errors_and_commends();

    self.sfc_struct_jsx_statement = Some(self.ast.statement_expression(
      SPAN,
      self.ast.expression_jsx_fragment(
        SPAN,
        self.ast.jsx_opening_fragment(SPAN),
        children,
        self.ast.jsx_closing_fragment(SPAN),
      ),
    ));

    ResParse::success(())
  }

  fn sort_errors_and_commends(&mut self) {
    self.comments.sort_by(|a, b| a.span.start.cmp(&b.span.start));
    self.errors.sort_by(|a, b| {
      let Some(a_labels) = &a.labels else { return Ordering::Less };
      let Some(b_labels) = &b.labels else { return Ordering::Greater };

      let Some(a_first) = a_labels.first() else { return Ordering::Less };
      let Some(b_first) = b_labels.first() else { return Ordering::Greater };

      a_first.offset().cmp(&b_first.offset())
    });
  }
}

/// Replace `<` and `>` inside `<script>` and `<style>` blocks with spaces so vize
/// doesn't mis-parse TypeScript generics or inline expressions as HTML tags.
/// Returns `None` if no script/style blocks were found (no allocation needed).
/// Vize bug: it doesn't handle RAWTEXT mode for script/style.
fn sanitize_rawtext_blocks(source: &str) -> Option<String> {
  const RAWTEXT_TAGS: &[&str] = &["script", "style"];
  let bytes = source.as_bytes();
  let mut result: Option<Vec<u8>> = None;
  let mut pos = 0usize;

  while pos < bytes.len() {
    // Find next '<'
    let Some(lt) = memchr::memchr(b'<', &bytes[pos..]) else { break };
    let lt_abs = pos + lt;

    // Check if this is an opening rawtext tag like <script or <style
    let after_lt = &bytes[lt_abs + 1..];
    let matching_tag = RAWTEXT_TAGS.iter().find(|&&tag| {
      let tb = tag.as_bytes();
      after_lt.starts_with(tb)
        && after_lt.get(tb.len()).map_or(true, |&c| {
          c == b'>' || c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'/'
        })
    });

    let Some(tag) = matching_tag else {
      pos = lt_abs + 1;
      continue;
    };

    // Find the end of the opening tag '>'
    let Some(open_gt) = memchr::memchr(b'>', &bytes[lt_abs..]) else { break };
    let content_start = lt_abs + open_gt + 1;

    // Find the matching closing tag </script> or </style> (case-insensitive)
    let close_pat = format!("</{tag}");
    let close_bytes = close_pat.as_bytes();
    let mut search_pos = content_start;
    let close_start = loop {
      let Some(rel) = memchr::memchr(b'<', &bytes[search_pos..]) else { break None };
      let cs = search_pos + rel;
      if bytes[cs..].len() >= close_bytes.len()
        && bytes[cs..cs + close_bytes.len()].eq_ignore_ascii_case(close_bytes)
      {
        break Some(cs);
      }
      search_pos = cs + 1;
    };

    let Some(close_start) = close_start else {
      pos = content_start;
      continue;
    };

    // Sanitize bytes[content_start..close_start]: replace < and > with spaces
    let needs_sanitize = bytes[content_start..close_start].iter().any(|&b| b == b'<' || b == b'>');
    if needs_sanitize {
      let buf = result.get_or_insert_with(|| bytes.to_vec());
      for b in &mut buf[content_start..close_start] {
        if *b == b'<' || *b == b'>' {
          *b = b' ';
        }
      }
    }

    pos = close_start;
  }

  result.map(|b| String::from_utf8(b).expect("sanitized source is valid utf8"))
}

// Easy transform from vize_armature::SourceLocation to oxc_span::Span
pub trait SourceLocatonSpan {
  fn span(&self) -> Span;
}

impl SourceLocatonSpan for SourceLocation {
  fn span(&self) -> Span {
    Span::new(self.start.offset, self.end.offset)
  }
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn basic_vue() {
    test_ast!("basic.vue");
    test_ast!("typescript.vue");
    test_ast!("void.vue", true, false);
    test_ast!("tags.vue");
    test_ast!("root_texts.vue");
    test_ast!("components.vue");
  }

  #[test]
  fn comments() {
    test_ast!("comments.vue");
  }

  #[test]
  fn errors() {
    test_ast!("error/template.vue", true, true);
    test_ast!("error/interpolation.vue", true, true);
    test_ast!("error/script.vue", true, false);
    test_ast!("error/directive.vue", true, false);
    test_ast!("error/script.vue", true, false);
    test_ast!("error/directive.vue", true, false);
    test_ast!("error/multiple_langs.vue", true, true);
    test_ast!("error/multiple_scripts.vue", true, true);
    test_ast!("error/empty_multiple_scripts.vue");
  }

  #[test]
  fn scripts() {
    test_ast!("scripts/basic.vue");
    test_ast!("scripts/setup.vue");
    test_ast!("scripts/both.vue");
    test_ast!("scripts/empty.vue");
    test_ast!("scripts/directives.vue");
  }
}
