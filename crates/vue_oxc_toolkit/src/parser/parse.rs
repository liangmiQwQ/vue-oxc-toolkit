use std::cell::RefCell;

use oxc_allocator::{self, Dummy, TakeIn, Vec as ArenaVec};
use oxc_ast::ast::{Program, Statement};

use oxc_span::{SPAN, SourceType, Span};
use oxc_syntax::module_record::ModuleRecord;
use vue_compiler_core::SourceLocation;
use vue_compiler_core::parser::{AstNode, ParseOption, Parser, WhitespaceStrategy};
use vue_compiler_core::scanner::{ScanOption, Scanner};

use crate::is_void_tag;
use crate::parser::error::OxcErrorHandler;

use super::ParserImpl;
use super::ParserImplReturn;

impl<'a> ParserImpl<'a> {
  pub fn parse(mut self) -> ParserImplReturn<'a> {
    match self.analyze() {
      Some(()) => {
        let span = Span::new(0, self.source_text.len() as u32);
        self.fix_module_records(span);

        ParserImplReturn {
          program: self.ast.program(
            span,
            self.source_type,
            self.source_text,
            self.comments.take_in(self.ast.allocator),
            None, // no hashbang needed for vue files
            self.ast.vec(),
            self.statements,
          ),
          fatal: false,
          errors: self.errors,
          module_record: self.module_record,
        }
      }
      None => ParserImplReturn {
        program: Program::dummy(self.allocator),
        fatal: true,
        errors: self.errors,
        module_record: ModuleRecord::new(self.allocator),
      },
    }
  }

  fn analyze(&mut self) -> Option<()> {
    let parser = Parser::new(ParseOption {
      whitespace: WhitespaceStrategy::Preserve,
      is_void_tag: |name| is_void_tag!(name),
      ..Default::default()
    });

    // get ast from vue-compiler-core
    let scanner = Scanner::new(ScanOption::default());
    // error processing
    let errors = RefCell::from(&mut self.errors);
    let panicked = RefCell::from(false);
    let tokens = scanner.scan(self.source_text, OxcErrorHandler::new(&errors, &panicked));
    let result = parser.parse(tokens, OxcErrorHandler::new(&errors, &panicked));

    if *panicked.borrow() {
      return None;
    }

    let mut children = self.ast.vec();
    for child in result.children {
      #[allow(clippy::single_match)]
      match child {
        AstNode::Element(node) => {
          if node.tag_name == "script" {
            // Fill self.setup, self.statements
            children.push(self.parse_script(node)?);
          } else if node.tag_name == "template" {
            children.push(self.parse_element(node, None)?);
          }
        }
        // TODO: Do not add comment, interpolation nodes for root elements, regard all of them as texts
        // AstNode::Text(text) => children.push(self.parse_text(&text)),
        // AstNode::Comment(comment) => children.push(self.parse_comment(&comment)),
        // AstNode::Interpolation(interp) => children.push(self.parse_interpolation(&interp)?),
        _ => (),
      }
    }

    self.sfc_return = Some(Statement::ReturnStatement(self.ast.alloc_return_statement(
      SPAN,
      Some(self.ast.expression_jsx_fragment(
        SPAN,
        self.ast.jsx_opening_fragment(SPAN),
        children,
        self.ast.jsx_closing_fragment(SPAN),
      )),
    )));

    Some(())
  }
}

// Easy transform from vue_compiler_core::SourceLocation to oxc_span::Span
pub trait SourceLocatonSpan {
  fn span(&self) -> Span;
}

impl SourceLocatonSpan for SourceLocation {
  fn span(&self) -> Span {
    Span::new(self.start.offset as u32, self.end.offset as u32)
  }
}

// Some public utils
impl<'a> ParserImpl<'a> {
  pub fn oxc_parse(
    &mut self,
    source: &str,
    source_type: SourceType,
    start: usize,
  ) -> Option<(ArenaVec<'a, Statement<'a>>, ModuleRecord<'a>)> {
    let source_text = self.ast.atom(&self.pad_source(source, start));
    let mut ret = oxc_parser::Parser::new(self.allocator, source_text.as_str(), source_type)
      .with_options(self.options)
      .parse();

    self.errors.append(&mut ret.errors);
    if ret.panicked {
      // TODO: do not panic for js parsing error
      None
    } else {
      self.comments.extend(&ret.program.comments[1..]);
      Some((ret.program.body, ret.module_record))
    }
  }

  /// A workaround
  /// Use comment placeholder to make the location AST returned correct
  /// The start must > 4 in any valid Vue files
  pub fn pad_source(&self, source: &str, start: usize) -> String {
    format!("/*{}*/{source}", &self.empty_str[..start - 4])
  }
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn basic_vue() {
    test_ast!("basic.vue");
    test_ast!("typescript.vue");
    test_ast!("void.vue");
    test_ast!("tags.vue");
  }

  #[test]
  fn comments() {
    test_ast!("comments.vue");
  }

  #[test]
  fn errors() {
    test_ast!("error/template.vue", true, true);
    test_ast!("error/interpolation.vue", true, true);
    test_ast!("error/recoverable-script.vue", true, false);
    test_ast!("error/recoverable-directive.vue", true, false);
    test_ast!("error/irrecoverable-script.vue", true, true);
    test_ast!("error/irrecoverable-directive.vue", true, true);
  }
}
