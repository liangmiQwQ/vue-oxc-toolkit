use oxc_allocator::Vec;
use oxc_ast::{
  AstBuilder, NONE,
  ast::{
    Expression, FormalParameters, FunctionType, JSXChild, JSXExpression, PropertyKey, PropertyKind,
    Statement,
  },
};
use oxc_span::SPAN;
use vue_compiler_core::parser::Directive;

use crate::parser::ParserImpl;

pub struct VSlotWrapper<'a, 'b> {
  ast: &'a AstBuilder<'b>,
  key: Option<PropertyKey<'b>>,
  params: Option<FormalParameters<'b>>,
  is_computed: Option<bool>,
}

impl<'a> ParserImpl<'a> {
  pub fn analyze_v_slot(
    &mut self,
    dir: &Directive<'a>,
    wrapper: &mut VSlotWrapper<'_, 'a>,
  ) -> Option<()> {
    todo!()
  }
}

impl<'a, 'b> VSlotWrapper<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self { ast, key: None, params: None, is_computed: None }
  }

  pub fn wrap(self, children: Vec<'b, JSXChild<'b>>) -> Vec<'b, JSXChild<'b>> {
    if self.include_v_slot() {
      let Self { ast, key, params, is_computed } = self;
      let key = key.unwrap();
      let params = params.unwrap();
      let is_computed = is_computed.unwrap();

      ast.vec1(ast.jsx_child_expression_container(
        SPAN,
        JSXExpression::ObjectExpression(ast.alloc_object_expression(
          SPAN,
          ast.vec1(ast.object_property_kind_object_property(
            SPAN,
            PropertyKind::Init,
            key,
            Expression::FunctionExpression(ast.alloc_function(
              SPAN,
              FunctionType::FunctionExpression,
              None,
              false,
              false,
              false,
              NONE,
              NONE,
              params,
              NONE,
              Some(ast.alloc_function_body(
                SPAN,
                ast.vec(),
                ast.vec1(Statement::ReturnStatement(ast.alloc_return_statement(
                  SPAN,
                  Some(Expression::JSXFragment(ast.alloc_jsx_fragment(
                    SPAN,
                    ast.jsx_opening_fragment(SPAN),
                    children,
                    ast.jsx_closing_fragment(SPAN),
                  ))),
                ))),
              )),
            )),
            true,
            false,
            is_computed,
          )),
        )),
      ))
    } else {
      children
    }
  }
}

impl<'b> VSlotWrapper<'_, 'b> {
  const fn include_v_slot(&self) -> bool {
    self.key.is_some() || self.params.is_some() || self.is_computed.is_some()
  }

  const fn set_key(&mut self, key: PropertyKey<'b>) {
    self.key = Some(key);
  }

  const fn set_params(&mut self, params: FormalParameters<'b>) {
    self.params = Some(params);
  }

  const fn set_is_computed(&mut self, is_computed: bool) {
    self.is_computed = Some(is_computed);
  }
}
