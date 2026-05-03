use std::cmp::Ordering;

use oxc_allocator::{self, CloneIn, Dummy, Vec as ArenaVec};
use oxc_ast::ast::{Directive, Expression, FormalParameterKind, JSXChild, Program, Statement};
use oxc_ast::{AstBuilder, NONE};
use oxc_diagnostics::OxcDiagnostic;

use oxc_span::{SPAN, Span};
use oxc_syntax::module_record::ModuleRecord;
use vue_oxlint_parser::{
  VueParseConfig, VueParser, VueParserReturn,
  ast::{VNode, VScriptKind},
};

use crate::parser::irregular_whitespaces::collect_irregular_whitespaces;
use crate::parser::{ResParse, ResParseExt, modules::Merge};

use super::ParserImpl;
use super::ParserImplReturn;

impl<'a> ParserImpl<'a> {
  pub fn parse(mut self) -> ParserImplReturn<'a> {
    let mut ret = self.parse_sfc();
    if !self.load_script_blocks_from_vue_parser(&mut ret) || ret.panicked {
      return ParserImplReturn {
        program: Program::dummy(self.allocator),
        fatal: true,
        errors: self.errors,
        module_record: ModuleRecord::new(self.allocator),
        irregular_whitespaces: Box::new([]),
        clean_spans: rustc_hash::FxHashSet::default(),
      };
    }
    let result = self.analyze(&ret.sfc.children);
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
          clean_spans,
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
          irregular_whitespaces: collect_irregular_whitespaces(source_text),
          clean_spans,
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
        irregular_whitespaces: Box::new([]),
        clean_spans: rustc_hash::FxHashSet::default(),
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
  fn analyze(&mut self, nodes: &[VNode<'a, 'a>]) -> ResParse<()> {
    let mut children: ArenaVec<'a, JSXChild<'a>> = self.ast.vec();
    for child in nodes {
      let VNode::Element(element) = child else {
        continue;
      };

      let tag_name = element.start_tag.name_span.source_text(self.source_text);
      let parsed_child = if tag_name.eq_ignore_ascii_case("template") {
        self.parse_element(element, None).0
      } else {
        self.parse_element(element, Some(self.ast.vec())).0
      };
      children.push(parsed_child);
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

  fn parse_sfc(&self) -> VueParserReturn<'a, 'a> {
    VueParser::new(
      self.allocator,
      self.allocator,
      self.origin_source_text,
      self.options,
      VueParseConfig { track_clean_spans: true },
    )
    .parse()
  }

  fn load_script_blocks_from_vue_parser(&mut self, ret: &mut VueParserReturn<'a, 'a>) -> bool {
    self.source_type = ret.sfc.source_type;
    let script_spans = collect_script_spans(&ret.sfc.children, self.source_text);
    let mut script_comments = ArenaVec::new_in(self.allocator);
    for comment in &ret.sfc.script_comments {
      if span_in_spans(comment.span, &script_spans) {
        script_comments.push(*comment);
      }
    }
    self.comments.append(&mut script_comments);
    let script_fatal = ret.errors.iter().any(is_fatal_script_block_error);
    self.errors.extend(ret.errors.iter().cloned());
    self.clean_spans.extend(ret.clean_spans.iter().copied());
    self
      .module_record
      .merge(std::mem::replace(&mut ret.module_record, ModuleRecord::new(self.allocator)));

    for child in &ret.sfc.children {
      self.load_script_node(child);
    }

    !script_fatal
  }

  fn load_script_node(&mut self, node: &VNode<'a, 'a>) {
    let VNode::Element(element) = node else {
      return;
    };

    if let Some(script) = &element.script {
      let mut directives = script.program.directives.clone_in(self.allocator);
      let body = script.program.body.clone_in(self.allocator);

      match script.kind {
        VScriptKind::Script => {
          self.global.directives.append(&mut directives);
          self.global.statements.extend(body);
        }
        VScriptKind::Setup => {
          self.setup.directives.append(&mut directives);

          let mut imports = self.ast.vec();
          let mut statements = self.ast.vec();
          for statement in body {
            match statement {
              Statement::ImportDeclaration(_) => imports.push(statement),
              _ => statements.push(statement),
            }
          }

          imports.append(&mut self.global.statements);
          self.global.statements = imports;
          self.setup.statements = statements;
        }
      }
    }

    for child in &element.children {
      self.load_script_node(child);
    }
  }

  fn sort_errors_and_commends(&mut self) {
    self.comments.sort_by_key(|a| a.span.start);
    self.errors.sort_by(|a, b| {
      let Some(a_labels) = &a.labels else { return Ordering::Less };
      let Some(b_labels) = &b.labels else { return Ordering::Greater };

      let Some(a_first) = a_labels.first() else { return Ordering::Less };
      let Some(b_first) = b_labels.first() else { return Ordering::Greater };

      a_first.offset().cmp(&b_first.offset())
    });
  }
}

fn collect_script_spans(children: &[VNode<'_, '_>], source_text: &str) -> Vec<Span> {
  let mut spans = Vec::new();
  for child in children {
    let VNode::Element(element) = child else {
      continue;
    };
    if element.start_tag.name_span.source_text(source_text).eq_ignore_ascii_case("script") {
      spans.push(element.span);
    }
    if let Some(script) = &element.script {
      spans.push(script.body_span);
    }
    spans.extend(collect_script_spans(&element.children, source_text));
  }
  spans
}

fn span_in_spans(inner: Span, spans: &[Span]) -> bool {
  spans.iter().any(|span| inner.start >= span.start && inner.end <= span.end)
}

fn is_fatal_script_block_error(error: &OxcDiagnostic) -> bool {
  matches!(
    error.message.as_ref(),
    "<script> and <script setup> must have the same language type."
      | "Single file component can contain only one <script> element."
      | "Single file component can contain only one <script setup> element."
  ) || error.message.starts_with("Unsupported lang ")
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  test_ast!(basic_vue, "basic.vue");
  test_ast!(typescript_vue, "typescript.vue");
  test_ast!(void_vue, "void.vue", true, false);
  test_ast!(tags_vue, "tags.vue");
  test_ast!(root_texts_vue, "root_texts.vue");
  test_ast!(components_vue, "components.vue");
  test_ast!(comments_vue, "comments.vue");
  test_ast!(error_template_vue, "error/template.vue", true, true);
  test_ast!(error_interpolation_vue, "error/interpolation.vue", true, true);
  test_ast!(error_script_vue, "error/script.vue", true, false);
  test_ast!(error_directive_vue, "error/directive.vue", true, false);
  test_ast!(error_multiple_langs_vue, "error/multiple_langs.vue", true, true);
  test_ast!(error_multiple_scripts_vue, "error/multiple_scripts.vue", true, true);
  test_ast!(error_empty_multiple_scripts_vue, "error/empty_multiple_scripts.vue");
  test_ast!(scripts_basic_vue, "scripts/basic.vue");
  test_ast!(scripts_setup_vue, "scripts/setup.vue");
  test_ast!(scripts_both_vue, "scripts/both.vue");
  test_ast!(scripts_empty_vue, "scripts/empty.vue");
  test_ast!(scripts_directives_vue, "scripts/directives.vue");
}
