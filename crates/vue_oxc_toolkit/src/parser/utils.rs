use std::cell::RefMut;

use oxc_diagnostics::OxcDiagnostic;

#[must_use]
pub fn is_simple_identifier(s: &str) -> bool {
  let mut chars = s.chars();
  let Some(first) = chars.next() else {
    return false;
  };
  if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
    return false;
  }
  for c in chars {
    if !(c.is_ascii_alphanumeric()
      || c == '_'
      || c == '$'
      || (c as u32 >= 0x00A0 && c as u32 <= 0xFFFF))
    {
      return false;
    }
  }
  true
}

/// A workaround to process false unnecessary errors from vue-compiler-core
pub fn filter_vue_parser_errors(mut errors: RefMut<Vec<OxcDiagnostic>>) {
  errors.retain(|e| e.message != "Illegal tag name. Use '&lt;' to print '<'.");
}
