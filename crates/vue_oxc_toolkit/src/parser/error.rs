use std::cell::RefCell;

use oxc_span::Span;

use oxc_diagnostics::OxcDiagnostic;
use vue_compiler_core::error::{CompilationError, CompilationErrorKind, ErrorHandler};

use crate::parser::{ParserImpl, ResParse, ResParseExt, parse::SourceLocatonSpan};

pub struct OxcErrorHandler<'a> {
  errors: &'a RefCell<&'a mut Vec<CompilationError>>,
}

impl<'a> OxcErrorHandler<'a> {
  pub const fn new(errors: &'a RefCell<&'a mut Vec<CompilationError>>) -> Self {
    Self { errors }
  }
}

impl ErrorHandler for OxcErrorHandler<'_> {
  fn on_error(&self, error: CompilationError) {
    self.errors.borrow_mut().push(error);
  }
}

impl ParserImpl<'_> {
  pub fn filter_and_append_errors(&mut self, errors: Vec<CompilationError>) -> ResParse<()> {
    for error in errors {
      if !self.is_warn(&error) {
        self.errors.push(OxcDiagnostic::error(error.to_string()).with_label(error.location.span()));
        // The panic error must not be a warn error
        if Self::should_panic(&error) {
          return ResParse::panic();
        }
      }
    }

    Ok(())
  }

  #[must_use]
  fn is_warn(&self, error: &CompilationError) -> bool {
    // Skip errors in <script> tags
    if self.script_tags.iter().any(|span| span.contains_inclusive(error.location.span())) {
      return true;
    }

    matches!(
      error.kind,
      CompilationErrorKind::InvalidFirstCharacterOfTagName
        | CompilationErrorKind::NestedComment
        | CompilationErrorKind::IncorrectlyClosedComment
        | CompilationErrorKind::IncorrectlyOpenedComment
        | CompilationErrorKind::AbruptClosingOfEmptyComment
        | CompilationErrorKind::MissingWhitespaceBetweenAttributes
        | CompilationErrorKind::MissingDirectiveArg
    )
  }

  #[must_use]
  const fn should_panic(error: &CompilationError) -> bool {
    matches!(
      error.kind,
      // EOF errors - incomplete template structure
      CompilationErrorKind::EofInTag
      | CompilationErrorKind::EofInComment
      | CompilationErrorKind::EofInCdata
      | CompilationErrorKind::EofBeforeTagName
      | CompilationErrorKind::EofInScriptHtmlCommentLikeText
      // Vue syntax incomplete - can't generate valid JSX
      | CompilationErrorKind::MissingInterpolationEnd
      | CompilationErrorKind::MissingDynamicDirectiveArgumentEnd
      | CompilationErrorKind::MissingEndTag
      // Critical structural issues
      | CompilationErrorKind::UnexpectedNullCharacter
      | CompilationErrorKind::CDataInHtmlContent
    )
  }
}

#[cold]
pub fn unexpected_script_lang(errors: &mut Vec<OxcDiagnostic>, lang: &str) {
  errors.push(OxcDiagnostic::error(format!("Unsupported lang {lang} in <script> blocks.")));
}

#[cold]
pub fn multiple_script_langs(errors: &mut Vec<OxcDiagnostic>) {
  errors
    .push(OxcDiagnostic::error("<script> and <script setup> must have the same language type."));
}

#[cold]
pub fn multiple_script_tags(errors: &mut Vec<OxcDiagnostic>, span: Span) {
  errors.push(
    OxcDiagnostic::error("Single file component can contain only one <script> element.")
      .with_label(span),
  );
}

#[cold]
pub fn multiple_script_setup_tags(errors: &mut Vec<OxcDiagnostic>, span: Span) {
  errors.push(
    OxcDiagnostic::error("Single file component can contain only one <script setup> element.")
      .with_label(span),
  );
}

#[cold]
pub fn v_else_without_adjacent_if(errors: &mut Vec<OxcDiagnostic>, span: Span) {
  errors.push(
    OxcDiagnostic::error("v-else/v-else-if has no adjacent v-if or v-else-if.").with_label(span),
  );
}

#[cold]
pub fn invalid_v_for_expression(errors: &mut Vec<OxcDiagnostic>, span: Span) {
  errors.push(OxcDiagnostic::error("v-for has invalid expression.").with_label(span));
}

#[cold]
pub fn v_if_else_without_expression(errors: &mut Vec<OxcDiagnostic>, span: Span) {
  errors.push(OxcDiagnostic::error("v-if/v-else-if is missing expression.").with_label(span));
}
