use std::mem::take;

use oxc_ast::{
  AstBuilder,
  ast::{Expression, JSXChild},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::GetSpan;

use crate::parser::ParserImpl;

pub enum VIf<'a> {
  If(Expression<'a>),
  ElseIf(Expression<'a>),
  Else,
}

// The manager of v-if / v-else-if / v-else, different from the wrapper, it works across multiple elements
pub struct VIfManager<'a, 'b> {
  ast: &'a AstBuilder<'b>,
  chain: Vec<(JSXChild<'b>, VIf<'b>)>, // child, v_if
}

impl<'a> ParserImpl<'a> {
  pub fn add_v_if(
    &mut self,
    child: JSXChild<'a>,
    v_if: VIf<'a>,
    manager: &mut VIfManager<'_, 'a>,
  ) -> Option<JSXChild<'a>> {
    if matches!(v_if, VIf::If(_)) {
      manager.chain.push((child, v_if));
      None
    } else if manager.chain.is_empty() {
      // Orphan v-else-if / v-else
      // https://play.vuejs.org/#eNp9kLFuwjAQhl/FuhnC0E4ordRWDO3QVi2jlyg5gsGxLd85REJ5d2wjAgNis/7v8+m/O8Kbc0UfEJZQMnZOV4yv0ghRNqoX/Rw14VxtXiSwDyhBLCItF5MKM2CqrdmottiRNXHOMX2XUNvOKY3+x7GyhiQsRSaJVVrbw1fO0tjZJa+3WO/v5DsaUibh1yOh72ORiXHlW+QzXv1/4xDfE+xsE3S0H8A/JKtD6njW3oNpYu0bL7f97Jz1rEy7ptXAaOiyVL5LNMfsS4jH/Hiw+rXuU/Gc/0kzwngCD9Z/dQ==
      self.errors.push(
        OxcDiagnostic::error("v-else/v-else-if has no adjacent v-if or v-else-if.")
          .with_label(child.span()),
      );
      Some(child)
    } else if matches!(v_if, VIf::Else) {
      manager.chain.push((child, v_if));
      // The chain is finished, return the result directly, for possible next node
      Some(manager.take_chain())
    } else {
      manager.chain.push((child, v_if));
      None
    }
  }
}

impl<'a, 'b> VIfManager<'a, 'b> {
  pub const fn new(ast: &'a AstBuilder<'b>) -> Self {
    Self { ast, chain: vec![] }
  }

  pub fn take_chain(&mut self) -> JSXChild<'b> {
    let rev_stack = take(&mut self.chain).reverse();
    todo!();
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
