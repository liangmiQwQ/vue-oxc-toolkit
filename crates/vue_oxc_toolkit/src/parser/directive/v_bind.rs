#[cfg(test)]
mod tests {
  use crate::test_ast;

  #[test]
  fn v_bind() {
    test_ast!("directive/v-bind.vue");
  }
}
