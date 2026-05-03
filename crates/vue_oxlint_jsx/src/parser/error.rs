use oxc_span::Span;

use oxc_diagnostics::OxcDiagnostic;

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
