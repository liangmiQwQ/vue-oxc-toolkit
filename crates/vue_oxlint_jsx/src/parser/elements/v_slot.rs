use oxc_allocator::{CloneIn, Vec};
use oxc_ast::{
  AstBuilder, NONE,
  ast::{
    FormalParameterKind, FormalParameters, JSXChild, JSXExpression, PropertyKey, PropertyKind,
  },
};
use oxc_span::SPAN;
use vue_oxlint_parser::ast::{VDirective, VDirectiveArgumentKind, VDirectiveExpression};

use crate::parser::ParserImpl;

pub struct VSlotWrapper<'a, 'b> {
  ast: &'a AstBuilder<'b>,
  key: Option<PropertyKey<'b>>,
  params: Option<FormalParameters<'b>>,
  is_computed: Option<bool>,
}

impl<'a> ParserImpl<'a> {
  pub fn analyze_v_slot(
    &self,
    dir: &VDirective<'a, 'a>,
    wrapper: &mut VSlotWrapper<'_, 'a>,
    _dir_name: &oxc_ast::ast::JSXAttributeName<'a>,
  ) {
    (|| {
      // --- Process Key ---
      if let Some(argument) = &dir.key.argument {
        if argument.kind == VDirectiveArgumentKind::Dynamic {
          let expression = argument.expression?.clone_in(self.allocator);
          wrapper.set_is_computed(true);
          wrapper.set_key(expression.into());
        } else {
          wrapper.set_is_computed(false);
          wrapper.set_key(self.ast.property_key_static_identifier(
            argument.span,
            self.codegen_directive_identifier(argument.raw),
          ));
        }
      } else {
        wrapper.set_is_computed(false);
        wrapper.set_key(self.ast.property_key_static_identifier(SPAN, "default"));
      }

      // --- Process Params ---
      // As vue use arrow function to wrap the slot content, we use it as well to deal with some edge cases
      // https://play.vuejs.org/#eNp9kD1PwzAQhv+KdXNJB5iigASoAwyAgNFLlBxpir/kO4dIkf87tquGDsBmvc9z9utb4Na5agoINTSM2qmW8UYaIZp7q52YLkhZrvfY9uivJSxCI1E7oIgSiifEchbGMrrNs4k22/VK2ABTZ83HOFQHsia9t2RXQpfcUaF/djxaQxJqUUhmrVL267Fk7ANuTnm3x+7zl/xAc84kvHgk9BNKWBm3fkA+4t3bE87pvEJt+6CS/Q98RbIq5I5H7S6YPtU+80rbB+2s59EM77SbGQ2dPpWLZjMWX0Jael7TX1//qXtZXZU5aSLEbzFYjTA=
      if let Some(value) = &dir.value
        && let VDirectiveExpression::VSlot(slot) = &value.expression
      {
        wrapper.set_params(slot.params.clone_in(self.allocator));
      } else {
        wrapper.set_params(self.ast.formal_parameters(
          SPAN,
          FormalParameterKind::ArrowFormalParameters,
          self.ast.vec(),
          NONE,
        ));
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
            ast.expression_arrow_function(
              SPAN,
              true,
              false,
              NONE,
              params,
              NONE,
              ast.alloc_function_body(
                SPAN,
                ast.vec(),
                ast.vec1(ast.statement_expression(
                  SPAN,
                  ast.expression_jsx_fragment(
                    SPAN,
                    ast.jsx_opening_fragment(SPAN),
                    children,
                    ast.jsx_closing_fragment(SPAN),
                  ),
                )),
              ),
            ),
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

  test_ast!(v_slot_vue, "directive/v-slot.vue");
}
