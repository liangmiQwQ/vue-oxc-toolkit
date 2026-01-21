use std::cell::RefCell;

use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;
use vue_compiler_core::error::{CompilationError, CompilationErrorKind, ErrorHandler};

pub struct OxcErrorHandler<'a> {
  errors: &'a RefCell<&'a mut Vec<OxcDiagnostic>>,
  panicked: &'a RefCell<bool>,
}

impl<'a> OxcErrorHandler<'a> {
  pub const fn new(
    errors: &'a RefCell<&'a mut Vec<OxcDiagnostic>>,
    panicked: &'a RefCell<bool>,
  ) -> Self {
    Self { errors, panicked }
  }
}

impl ErrorHandler for OxcErrorHandler<'_> {
  fn on_error(&self, error: CompilationError) {
    if should_panick(&error) {
      *self.panicked.borrow_mut() = true;
    }
    if !is_warn(&error) {
      self.errors.borrow_mut().push(
        OxcDiagnostic::error(error.to_string()).with_label(Span::new(
          error.location.start.offset as u32,
          error.location.end.offset as u32,
        )),
      );
    }
  }
}

#[must_use]
const fn is_warn(error: &CompilationError) -> bool {
  matches!(
    error.kind,
    CompilationErrorKind::InvalidFirstCharacterOfTagName
  )
}

#[must_use]
const fn should_panick(error: &CompilationError) -> bool {
  matches!(error.kind, CompilationErrorKind::MissingEndTag)
}
