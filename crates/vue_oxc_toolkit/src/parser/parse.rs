use std::cell::RefCell;

use oxc_allocator::{self, Dummy, TakeIn};
use oxc_ast::ast::{Program, Statement};

use oxc_span::{SPAN, Span};
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
