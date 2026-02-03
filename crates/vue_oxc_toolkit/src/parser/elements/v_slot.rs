use oxc_allocator::{TakeIn, Vec};
use oxc_ast::{
  AstBuilder, NONE,
  ast::{
    Expression, FormalParameterKind, FormalParameters, JSXAttributeName, JSXChild, JSXExpression,
    ObjectPropertyKind, PropertyKey, PropertyKind, Statement,
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
    dir_name: &JSXAttributeName<'a>,
  ) {
    (|| {
      // --- Process Key ---
      let JSXAttributeName::NamespacedName(name_space) = dir_name else { unreachable!() };
      let key_span = name_space.name.span;
      if key_span.is_empty() {
        // Generate a dummy one
        wrapper.set_is_computed(false);
        wrapper.set_key(self.ast.property_key_static_identifier(SPAN, "default"));
      } else {
        // Parse with a object expression wrapper
        let str = key_span.source_text(self.source_text);
        let Expression::ObjectExpression(mut object_expression) = self.parse_expression(
          self.ast.atom(&format!("{{{str}: 0}}")).as_str(),
          key_span.start as usize - 1,
        )?
        else {
          unreachable!()
        };
        // SAFETY: must get wrapped
        let ObjectPropertyKind::ObjectProperty(object_property) =
          object_expression.properties.first_mut().unwrap()
        else {
          unreachable!()
        };
        if let PropertyKey::StaticIdentifier(_) = object_property.key {
          wrapper.set_is_computed(false);
        } else {
          wrapper.set_is_computed(true);
        }
        wrapper.set_key(object_property.key.take_in(self.allocator));
      }

      // --- Process Params ---
      // As vue use arrow function to wrap the slot content, we use it as well to prevent some edge cases
      // https://play.vuejs.org/#eNp9kD1PwzAQhv+KdXNJB5iigASoAwyAgNFLlBxpir/kO4dIkf87tquGDsBmvc9z9utb4Na5agoINTSM2qmW8UYaIZp7q52YLkhZrvfY9uivJSxCI1E7oIgSiifEchbGMrrNs4k22/VK2ABTZ83HOFQHsia9t2RXQpfcUaF/djxaQxJqUUhmrVL267Fk7ANuTnm3x+7zl/xAc84kvHgk9BNKWBm3fkA+4t3bE87pvEJt+6CS/Q98RbIq5I5H7S6YPtU+80rbB+2s59EM77SbGQ2dPpWLZjMWX0Jael7TX1//qXtZXZU5aSLEbzFYjTA=
      if dir.has_empty_expr() {
        wrapper.set_params(self.ast.formal_parameters(
          SPAN,
          FormalParameterKind::ArrowFormalParameters,
          self.ast.vec(),
          NONE,
        ));
      } else {
        let expr = dir.expression.as_ref().unwrap();
        let params = format!("({}) => 0", expr.content.raw);
        let Expression::ArrowFunctionExpression(mut arrow_function_expression) = self
          .parse_expression(self.ast.atom(params.as_str()).as_str(), expr.location.start.offset)?
        else {
          unreachable!()
        };
        wrapper.set_params(arrow_function_expression.params.take_in(self.allocator));
      }

      Some(())
    })();
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
            Expression::ArrowFunctionExpression(ast.alloc_arrow_function_expression(
              SPAN,
              true,
              false,
              NONE,
              params,
              NONE,
              ast.alloc_function_body(
                SPAN,
                ast.vec(),
                ast.vec1(Statement::ExpressionStatement(ast.alloc_expression_statement(
                  SPAN,
                  Expression::JSXFragment(ast.alloc_jsx_fragment(
                    SPAN,
                    ast.jsx_opening_fragment(SPAN),
                    children,
                    ast.jsx_closing_fragment(SPAN),
                  )),
                ))),
              ),
            )),
            false,
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
    self.key.is_some() && self.params.is_some() && self.is_computed.is_some()
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

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn v_slot() {
    test_ast!("directive/v-slot.vue");
  }
}
