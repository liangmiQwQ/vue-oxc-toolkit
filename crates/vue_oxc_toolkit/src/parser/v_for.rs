use oxc_ast::{
  AstBuilder, NONE,
  ast::{
    Argument, Expression, FormalParameters, JSXChild, JSXElement, JSXExpression,
    ParenthesizedExpression, Statement,
  },
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::{SPAN, Span};
use regex::Regex;
use vue_compiler_core::parser::Directive;

use crate::parser::{ParserImpl, parse::SourceLocatonSpan};

pub struct VForWrapper<'a, 'b> {
  ast: &'a AstBuilder<'b>,
  data_origin: Option<ParenthesizedExpression<'b>>,
  params: Option<FormalParameters<'b>>,
}

impl ParserImpl<'_> {
  fn invalid_v_for_expression(&mut self, span: Span) {
    self
      .errors
      .push(OxcDiagnostic::error("Invalid v-for expression").with_label(span));
  }

  pub fn analyze_v_for<'b>(&mut self, dir: &Directive<'b>, wrapper: &mut VForWrapper<'_, 'b>) {
    if dir.has_empty_expr() {
      self.invalid_v_for_expression(dir.location.span());
    }
    let expr = dir.expression.as_ref().unwrap();
    let for_alias_regex = Regex::new(r"^([\s\S]*?)\s+(?:in|of)\s+(\S[\s\S]*)").unwrap();

    if let Some(caps) = for_alias_regex.captures(expr.content.raw) {
      let data_origin = &caps[1];
      // let data_origin = self.parse_expression(data_origin);
      let params = &caps[2];
    } else {
      self.invalid_v_for_expression(dir.location.span());
    }
  }
}

impl<'a, 'b> VForWrapper<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self {
      ast,
      data_origin: None,
      params: None,
    }
  }

  pub fn wrapper(self, element: JSXElement<'b>) -> JSXChild<'b> {
    if self.inclulde_v_for() {
      let Self {
        ast,
        data_origin,
        params,
      } = self;
      let data_origin = data_origin.unwrap();
      let params = params.unwrap();

      ast.jsx_child_expression_container(
        SPAN,
        JSXExpression::from(Expression::CallExpression(
          ast.alloc_call_expression(
            SPAN,
            Expression::ParenthesizedExpression(ast.alloc(data_origin)),
            NONE,
            self
              .ast
              .vec1(Argument::from(Expression::ArrowFunctionExpression(
                ast.alloc_arrow_function_expression(
                  SPAN,
                  true,
                  false,
                  NONE,
                  params,
                  NONE,
                  ast.function_body(
                    SPAN,
                    ast.vec(),
                    ast.vec1(Statement::ExpressionStatement(
                      ast.alloc_expression_statement(
                        SPAN,
                        Expression::ParenthesizedExpression(ast.alloc_parenthesized_expression(
                          SPAN,
                          Expression::JSXElement(self.ast.alloc(element)),
                        )),
                      ),
                    )),
                  ),
                ),
              ))),
            false,
          ),
        )),
      )
    } else {
      JSXChild::Element(self.ast.alloc(element))
    }
  }
}

impl<'b> VForWrapper<'_, 'b> {
  const fn inclulde_v_for(&self) -> bool {
    self.data_origin.is_some() && self.params.is_some()
  }

  const fn set_data_origin(&mut self, data_origin: ParenthesizedExpression<'b>) {
    self.data_origin = Some(data_origin);
  }

  const fn set_params(&mut self, params: FormalParameters<'b>) {
    self.params = Some(params);
  }
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn v_for() {
    test_ast!("directive/v-for.vue");
  }

  #[test]
  fn v_for_error() {
    test_ast!("directive/v-for-error.vue", true, false);
  }
}
