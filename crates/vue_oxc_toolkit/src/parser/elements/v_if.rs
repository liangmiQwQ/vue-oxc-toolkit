#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn v_if() {
    test_ast!("directive/v-if.vue");
  }
}
