use oxc_ast::{
  AstBuilder, NONE,
  ast::{Argument, Expression, JSXChild, JSXElement, JSXExpression, Statement},
};
use oxc_span::SPAN;
use vue_compiler_core::parser::Element;

pub struct VForWrapper<'a, 'b> {
  inclulde_v_for: bool,
  ast: &'a AstBuilder<'b>,
}

impl<'a, 'b> VForWrapper<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self {
      inclulde_v_for: false,
      ast,
    }
  }

  pub fn wrapper(self, element: JSXElement<'b>) -> JSXChild<'b> {
    if self.inclulde_v_for {
      let expr_statement = self.ast.alloc_expression_statement(
        SPAN,
        Expression::ParenthesizedExpression(
          self
            .ast
            .alloc_parenthesized_expression(SPAN, Expression::JSXElement(self.ast.alloc(element))),
        ),
      );

      let arrow_function = Expression::ArrowFunctionExpression(
        self.ast.alloc_arrow_function_expression(
          SPAN,
          true,
          false,
          NONE,
          todo!(),
          NONE,
          self.ast.function_body(
            SPAN,
            self.ast.vec(),
            self
              .ast
              .vec1(Statement::ExpressionStatement(expr_statement)),
          ),
        ),
      );

      let call_expr = Expression::CallExpression(self.ast.alloc_call_expression(
        SPAN,
        todo!("IDENTIFIER THERE"),
        NONE,
        self.ast.vec1(Argument::from(arrow_function)),
        false,
      ));

      self
        .ast
        .jsx_child_expression_container(SPAN, JSXExpression::from(call_expr))
    } else {
      JSXChild::Element(self.ast.alloc(element))
    }
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
