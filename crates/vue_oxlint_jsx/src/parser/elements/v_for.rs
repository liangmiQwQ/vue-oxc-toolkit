use oxc_allocator::CloneIn;
use oxc_ast::{
  AstBuilder, NONE,
  ast::{
    Argument, Expression, FormalParameters, JSXChild, JSXElement, JSXExpression,
    ParenthesizedExpression,
  },
};

use oxc_span::{SPAN, Span};
use vue_oxlint_parser::ast::{VDirective, VDirectiveExpression};

use crate::parser::{ParserImpl, error};

pub struct VForWrapper<'a, 'b> {
  ast: &'a AstBuilder<'b>,
  data_origin: Option<ParenthesizedExpression<'b>>,
  params: Option<FormalParameters<'b>>,
}

impl<'a> ParserImpl<'a> {
  fn invalid_v_for_expression(&mut self, span: Span) -> Option<()> {
    error::invalid_v_for_expression(&mut self.errors, span);
    None
  }

  pub fn analyze_v_for(&mut self, dir: &VDirective<'a, 'a>, wrapper: &mut VForWrapper<'_, 'a>) {
    (|| {
      if let Some(value) = &dir.value
        && let VDirectiveExpression::VFor(v_for) = &value.expression
      {
        wrapper.set_data_origin(
          self.ast.parenthesized_expression(SPAN, v_for.right.clone_in(self.allocator)),
        );
        wrapper.set_params(v_for.left.clone_in(self.allocator));
      } else {
        self.invalid_v_for_expression(dir.span)?;
      }

      Some(())
    })();
  }
}

/// Wrap the JSX element with a function call, similar to jsx {items.map(items => <div key={item.id} />)} but with vue semantic.
impl<'a, 'b> VForWrapper<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self { ast, data_origin: None, params: None }
  }

  pub fn wrap(self, element: JSXElement<'b>) -> JSXChild<'b> {
    if self.include_v_for() {
      let Self { ast, data_origin, params } = self;
      let data_origin = data_origin.unwrap();
      let params = params.unwrap();

      ast.jsx_child_expression_container(
        SPAN,
        JSXExpression::CallExpression(ast.alloc_call_expression(
          SPAN,
          Expression::ParenthesizedExpression(ast.alloc(data_origin)),
          NONE,
          self.ast.vec1(Argument::ArrowFunctionExpression(ast.alloc_arrow_function_expression(
            SPAN,
            true,
            false,
            NONE,
            params,
            NONE,
            ast.function_body(
              SPAN,
              ast.vec(),
              ast.vec1(ast.statement_expression(
                SPAN,
                ast.expression_parenthesized(SPAN, Expression::JSXElement(self.ast.alloc(element))),
              )),
            ),
          ))),
          false,
        )),
      )
    } else {
      JSXChild::Element(self.ast.alloc(element))
    }
  }
}

impl<'b> VForWrapper<'_, 'b> {
  const fn include_v_for(&self) -> bool {
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

  test_ast!(v_for_vue, "directive/v-for.vue");
  test_ast!(v_for_error_vue, "directive/v-for-error.vue", true, false);
}
