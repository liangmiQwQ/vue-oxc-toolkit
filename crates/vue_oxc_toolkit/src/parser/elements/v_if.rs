use oxc_ast::{
  AstBuilder,
  ast::{Expression, JSXChild},
};

pub enum VIf<'a> {
  If(Expression<'a>),
  ElseIf(Expression<'a>),
  Else,
}

// The manager of v-if / v-else-if / v-else, different from the wrapper, it works across multiple elements
pub struct VIfManager<'a, 'b> {
  ast: &'a AstBuilder<'b>,
}

impl<'a, 'b> VIfManager<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self { ast }
  }
}

#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn v_if() {
    test_ast!("directive/v-if.vue");
  }
}
