use std::cell::RefCell;
use std::collections::HashSet;

use oxc_allocator::{self, Dummy, Vec as ArenaVec};
use oxc_ast::AstBuilder;
use oxc_ast::ast::{JSXChild, Program, Statement};

use oxc_span::{SPAN, Span};
use oxc_syntax::module_record::ModuleRecord;
use vue_compiler_core::SourceLocation;
use vue_compiler_core::parser::{AstNode, ParseOption, Parser, WhitespaceStrategy};
use vue_compiler_core::scanner::{ScanOption, Scanner};

use crate::is_void_tag;
use crate::parser::error::OxcErrorHandler;
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
          setup,
          statements,
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
            statements.directives,
            Self::get_body_statements(statements.statements, setup, sfc_return, ast),
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

  fn push_text_child(&self, children: &mut ArenaVec<'a, JSXChild<'a>>, span: Span) {
    if !span.is_empty() {
      let atom = self.ast.atom(span.source_text(self.source_text));
      children.push(self.ast.jsx_child_text(span, atom, Some(atom)));
    }
  }

  fn analyze(&mut self) -> ResParse<()> {
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
      return ResParse::panic();
    }

    let mut children = self.ast.vec();
    let mut text_start: u32 = 0;
    let mut source_types: HashSet<&str> = HashSet::new();
    for child in result.children {
      if let AstNode::Element(node) = child {
        // Process the texts between last element and current element
        self
          .push_text_child(&mut children, Span::new(text_start, node.location.start.offset as u32));
        text_start = node.location.end.offset as u32;

        if node.tag_name == "script" {
          // Fill self.setup, self.statements
          if let Some(child) = self.parse_script(node, &mut source_types)? {
            children.push(child);
          }
        } else if node.tag_name == "template" {
          children.push(self.parse_element(node, None).0);
        } else {
          // Process other tags like <style>
          let text = if let Some(first) = node.children.first() {
            let last = node.children.last().unwrap(); // SAFETY: if first exists, last must exist
            let span = Span::new(
              first.get_location().start.offset as u32,
              last.get_location().end.offset as u32,
            );

            let atom = self.ast.atom(span.source_text(self.source_text));
            self.ast.vec1(self.ast.jsx_child_text(span, atom, Some(atom)))
          } else {
            self.ast.vec()
          };

          children.push(self.parse_element(node, Some(text)).0);
        }
      }
    }
    // Process the texts after last element
    self.push_text_child(&mut children, Span::new(text_start, self.source_text.len() as u32));

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

  fn get_body_statements(
    mut statements: ArenaVec<'a, Statement<'a>>,
    mut setup: ArenaVec<'a, Statement<'a>>,
    sfc_return: Option<Statement<'a>>,
    ast: AstBuilder<'a>,
  ) -> ArenaVec<'a, Statement<'a>> {
    statements.push(ast.statement_block(SPAN, {
      if let Some(ret) = sfc_return {
        setup.push(ret);
      }
      setup
    }));

    statements
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
    test_ast!("scripts/invaild-export.vue", true, false);
  }
}
