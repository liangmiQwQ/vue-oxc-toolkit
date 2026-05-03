//! Structs defined in this mod aren't V* nodes, it is just a helper struct to store binding-related things.
use oxc_ast::ast::IdentifierReference;

#[derive(Debug)]
pub struct Reference<'a> {
  pub id: IdentifierReference<'a>,
  pub mode: &'static str,
}

#[derive(Debug)]
pub struct Variable<'a> {
  pub id: IdentifierReference<'a>,
  pub kind: &'static str,
}
